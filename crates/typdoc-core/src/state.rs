//! The state file, `.typdoc/state/<namespace>.json`: the highest number `typdoc new` has
//! allocated per collection in one namespace. `read` is used by every command; `write` is used
//! only by the commands that issue a number, `new` and `mv --renumber` (decision 13), through
//! the one seam every write in this crate goes through (`write_atomically`), under the
//! namespace's lock.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use crate::config::Namespace;
use crate::error::Error;
use crate::fs::{Fs, write_atomically};
use crate::namespace_lock::NamespaceLock;

const STATE_DIR: &str = ".typdoc/state";

/// The path of a namespace's state file, relative to the project folder.
pub(crate) fn file_path(namespace: &str) -> String {
    format!("{STATE_DIR}/{namespace}.json")
}

/// One namespace's state file: every collection named there, split into `last` (a usable whole
/// number) and `malformed` (present but not one, described for `state.malformed`'s message). An
/// entry with no `last` field at all is read as not recorded, the same reading `state.missing`
/// already gives a namespace whose collection is new — decision 13 only gives a name to a `last`
/// that is *there* and wrong, not to one that is absent.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct StateFile {
    pub last: BTreeMap<String, u64>,
    pub malformed: BTreeMap<String, String>,
}

impl StateFile {
    /// Whether `collection` has a record at all, usable or not: `state.missing` and
    /// `state.malformed` are different findings for two different situations (decision 13), and
    /// this is what tells them apart.
    pub(crate) fn has(&self, collection: &str) -> bool {
        self.last.contains_key(collection) || self.malformed.contains_key(collection)
    }
}

/// Reads one namespace's state file. A missing file is not an error: an empty `StateFile` is
/// exactly what "no record kept yet" means, the same reading `state.missing` gives a namespace
/// whose collection is new. A file that cannot be read as a JSON object has no id of its own in
/// the design's table, so it stops the command the same id-less way an unreadable schema
/// already does (`schema::load`'s own `Schema::parse` failure).
pub(crate) fn read(root: &Path, namespace: &str) -> Result<StateFile, Error> {
    let relative = file_path(namespace);
    let file = root.join(&relative);
    let bytes = match fs::read(&file) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(StateFile::default()),
        Err(source) => return Err(Error::Io { file, source }),
    };
    parse(&bytes).map_err(|message| Error::Config { file, message })
}

/// The parsing [`read`] is built from, kept apart from the file so it can be unit-tested on
/// plain bytes with no file system at all: one JSON object, split into `last` (a usable whole
/// number) and `malformed` (present but not one). An entry with no `last` field at all is read
/// as not recorded (decision 13 only gives a name to a `last` that is present and wrong). `Err`
/// when the bytes are not one JSON object.
fn parse(bytes: &[u8]) -> Result<StateFile, String> {
    let value: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|e| format!("cannot be parsed: {e}"))?;
    let serde_json::Value::Object(top) = value else {
        return Err("must be one JSON object".to_owned());
    };
    let mut last = BTreeMap::new();
    let mut malformed = BTreeMap::new();
    for (name, entry) in top {
        let Some(value) = entry.get("last") else {
            continue;
        };
        match value.as_u64() {
            Some(number) => {
                last.insert(name, number);
            }
            None => {
                malformed.insert(name, value.to_string());
            }
        }
    }
    Ok(StateFile { last, malformed })
}

/// Records `last` as `collection`'s new number in `namespace`'s state file, under `lock`: what
/// `typdoc new` and `mv --renumber` call once they have decided the number, never the
/// allocation itself, which is the caller's. `.typdoc/state/` is created first if this is the
/// namespace's first write, the same way `acquire` creates `.typdoc/locks/` first. The write
/// itself goes through `write_atomically`, so a reader sees the old file or the new one whole.
///
/// `pub`, and re-exported at the crate root: `new` and `mv --renumber` are the two callers, and
/// `crates/typdoc-fs/tests/` is where a real file system exercises this function directly,
/// against the crate that is allowed to write.
pub fn write(
    fs: &dyn Fs,
    lock: &NamespaceLock<'_>,
    root: &Path,
    namespace: &str,
    collection: &str,
    last: u64,
) -> Result<(), Error> {
    let path = root.join(file_path(namespace));
    let current = match fs::read(&path) {
        Ok(bytes) => Some(bytes),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(source) => return Err(Error::Io { file: path, source }),
    };
    // A file that is there but is not valid UTF-8 cannot be reached in practice: `write` is only
    // ever asked to update a file `read` already parsed as JSON, under the same lock throughout,
    // and JSON is UTF-8. `rewrite` falls back to a fresh file the same way it would for a file
    // that was never there, rather than this function inventing a second way to refuse.
    let current_text = current
        .as_deref()
        .and_then(|bytes| std::str::from_utf8(bytes).ok());
    let bytes = rewrite(current_text, collection, last);
    let state_dir = root.join(STATE_DIR);
    fs.create_dir_all(&state_dir)
        .map_err(Error::io_at(&state_dir))?;
    write_atomically(fs, lock, &path, &bytes).map_err(Error::io_at(&path))
}

/// The bytes [`write`] puts on disk for `collection`'s new `last`, given the file's current text
/// when it has one.
///
/// When `collection` already has an entry there (`state.malformed`'s own case included, though
/// `write` is only ever asked to update one `state.missing` and `state.malformed` both let
/// through, which means a valid one in practice), only that entry's `last` value is replaced —
/// found by walking the file's own structure, not by searching its text, so a value elsewhere
/// that happens to spell `last` inside a string is never mistaken for the field — and every
/// other byte is left exactly where it was. Otherwise the whole file is (re)written in the
/// canonical form decision 13 gives a file typdoc creates or adds an entry to ([`canonical`]):
/// every existing entry's own value carried over unchanged, `collection`'s entry set, keys
/// sorted alphabetically at every level, two-space indentation, `\n` endings and a final
/// newline.
///
/// `current` that is present but does not parse as one JSON object cannot be reached in
/// practice either, for the same reason: it falls back to the same canonical rewrite a missing
/// file gets. This function returns bytes, not a `Result`, so refusing outright is not an
/// option open to it; the fallback is the safest thing left to do with a fault it did not cause.
pub(crate) fn rewrite(current: Option<&str>, collection: &str, last: u64) -> Vec<u8> {
    if let Some(text) = current
        && let Ok(serde_json::Value::Object(map)) = serde_json::from_str::<serde_json::Value>(text)
    {
        if let Some(patched) = patch_in_place(text, collection, last) {
            return patched;
        }
        return canonical(&map, collection, last);
    }
    canonical(&serde_json::Map::new(), collection, last)
}

/// The in-place edit: `collection`'s entry already has a `last` value in `text`, at a span
/// [`value_span`] finds by walking `text`'s structure, and that span is replaced with `last`'s
/// digits. `None` when `collection` has no entry yet, or its entry has no `last` field, so the
/// caller falls back to the canonical rewrite instead.
fn patch_in_place(text: &str, collection: &str, last: u64) -> Option<Vec<u8>> {
    let (entry_start, entry_end) = value_span(text, collection)?;
    let entry_text = &text[entry_start..entry_end];
    let (rel_start, rel_end) = value_span(entry_text, "last")?;
    let abs_start = entry_start + rel_start;
    let abs_end = entry_start + rel_end;
    let mut bytes = text.as_bytes().to_vec();
    bytes.splice(abs_start..abs_end, last.to_string().into_bytes());
    Some(bytes)
}

/// The canonical text of a state file typdoc creates or adds an entry to (decision 13):
/// `existing`'s entries, whatever they hold, plus `collection` set to its new `last`, printed as
/// one JSON object with two-space indentation and a final `\n`, keys sorted alphabetically at
/// every level ([`sorted`]). Falls back to an empty object on the one `to_string_pretty` failure
/// a `String`-keyed, finite-valued map can never actually produce, the same defensive shape
/// `namespace_lock::acquire` already gives its own stamp.
fn canonical(
    existing: &serde_json::Map<String, serde_json::Value>,
    collection: &str,
    last: u64,
) -> Vec<u8> {
    let mut map = existing.clone();
    map.insert(collection.to_owned(), serde_json::json!({ "last": last }));
    let value = sorted(serde_json::Value::Object(map));
    let mut text = serde_json::to_string_pretty(&value).unwrap_or_else(|_| "{}".to_owned());
    text.push('\n');
    text.into_bytes()
}

/// `value`, with every object's keys sorted alphabetically, at every level.
///
/// `serde_json::Map` is a plain `BTreeMap`, and sorts on its own, only when nothing in the build
/// asks `serde_json` for its `preserve_order` feature — and something does: `typdoc`, the
/// binary crate, needs it for a reason of its own (`--json`'s numbers, project.md), and a
/// workspace build unifies features across members, so it reaches `typdoc-core`'s `serde_json`
/// the same way `project.md` already documents for `chrono`'s `clock` feature. This function is
/// what makes the order right regardless of which one `serde_json::Map` happens to be.
fn sorted(value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let mut ordered: BTreeMap<String, serde_json::Value> = BTreeMap::new();
            for (key, value) in map {
                ordered.insert(key, sorted(value));
            }
            serde_json::Value::Object(ordered.into_iter().collect())
        }
        serde_json::Value::Array(items) => {
            serde_json::Value::Array(items.into_iter().map(sorted).collect())
        }
        other => other,
    }
}

/// The byte range of `key`'s value within `object`'s own JSON text (which begins at its `{`),
/// found by walking the object's structure rather than searching its text, so a value elsewhere
/// in it that happens to contain the key's own spelling is never mistaken for the key itself.
/// `None` when the object has no such key at its own top level, or when `object` does not begin
/// with `{` the way a JSON object's text always does, or runs out before a value it expects.
fn value_span(object: &str, key: &str) -> Option<(usize, usize)> {
    let bytes = object.as_bytes();
    if bytes.first() != Some(&b'{') {
        return None;
    }
    let mut i = skip_ws(bytes, 1);
    loop {
        if bytes.get(i) == Some(&b'}') {
            return None;
        }
        let (found, after_key) = parse_string(bytes, i)?;
        i = skip_ws(bytes, after_key);
        if bytes.get(i) != Some(&b':') {
            return None;
        }
        i = skip_ws(bytes, i + 1);
        let value_start = i;
        let value_end = skip_value(bytes, i)?;
        if found == key {
            return Some((value_start, value_end));
        }
        i = skip_ws(bytes, value_end);
        match bytes.get(i) {
            Some(b',') => i = skip_ws(bytes, i + 1),
            _ => return None,
        }
    }
}

fn skip_ws(bytes: &[u8], mut i: usize) -> usize {
    while matches!(bytes.get(i), Some(b' ' | b'\t' | b'\n' | b'\r')) {
        i += 1;
    }
    i
}

/// Reads a JSON string starting at `bytes[start]` (its opening `"`), returning the text between
/// the quotes exactly as written and the index just past the closing quote. Every key this is
/// ever asked for is a plain name typdoc itself would write, so nothing here decodes an escape;
/// a backslash is only ever skipped past, never interpreted, which is enough to find the
/// closing quote correctly whatever it precedes. `None` when `bytes` runs out first, or
/// `bytes[start]` is not `"`.
fn parse_string(bytes: &[u8], start: usize) -> Option<(&str, usize)> {
    if bytes.get(start) != Some(&b'"') {
        return None;
    }
    let mut i = start + 1;
    loop {
        match bytes.get(i)? {
            b'\\' => i += 2,
            b'"' => {
                let raw = std::str::from_utf8(&bytes[start + 1..i]).ok()?;
                return Some((raw, i + 1));
            }
            _ => i += 1,
        }
    }
}

/// The index just past the JSON value starting at `bytes[start]`, whatever its type: an object
/// or array is matched by depth, respecting strings so a `}` or `]` written inside one is never
/// mistaken for the end; anything else (a number, `true`, `false`, `null`) ends at the next
/// character JSON never lets a value hold: a comma, a closing brace or bracket, or whitespace.
/// `None` when `bytes` runs out before the value ends.
fn skip_value(bytes: &[u8], start: usize) -> Option<usize> {
    match *bytes.get(start)? {
        b'"' => parse_string(bytes, start).map(|(_, end)| end),
        open @ (b'{' | b'[') => {
            let close = if open == b'{' { b'}' } else { b']' };
            let mut depth = 0usize;
            let mut i = start;
            loop {
                match *bytes.get(i)? {
                    b'"' => i = parse_string(bytes, i)?.1,
                    found if found == open => {
                        depth += 1;
                        i += 1;
                    }
                    found if found == close => {
                        depth -= 1;
                        i += 1;
                        if depth == 0 {
                            return Some(i);
                        }
                    }
                    _ => i += 1,
                }
            }
        }
        _ => {
            let mut i = start;
            while !matches!(
                bytes.get(i),
                None | Some(b',' | b'}' | b']' | b' ' | b'\t' | b'\n' | b'\r')
            ) {
                i += 1;
            }
            Some(i)
        }
    }
}

/// The state files in `.typdoc/state/` that match no namespace of `namespaces`
/// (`config.state-orphan`), by their path from the project folder, sorted. Every `*.json` file
/// there is read as a state file, the same way `.typdoc/collections/` treats its own files;
/// anything else is ignored.
pub(crate) fn orphans(root: &Path, namespaces: &[Namespace]) -> Result<Vec<String>, Error> {
    let dir = root.join(STATE_DIR);
    let entries = match fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(source) => return Err(Error::Io { file: dir, source }),
    };
    let known: std::collections::BTreeSet<&str> =
        namespaces.iter().map(|n| n.name.as_str()).collect();
    let mut found = Vec::new();
    for entry in entries {
        let entry = entry.map_err(Error::io_at(&dir))?;
        let file_name = entry.file_name().to_string_lossy().into_owned();
        let Some(name) = file_name.strip_suffix(".json") else {
            continue;
        };
        if !known.contains(name) {
            found.push(format!("{STATE_DIR}/{file_name}"));
        }
    }
    found.sort();
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Nothing here touches a file system: this crate's own lint bans a write outside the seam,
    // even in test code, so the parts that do touch one — `read` and `orphans`'s directory walk,
    // and `write`'s own glue (a real read, `create_dir_all`, `write_atomically`) — are exercised
    // through the built binary (`crates/typdoc/tests/state.rs`) and, for `write`, through
    // `crates/typdoc-fs/tests/`, which is the crate allowed to write. What is unit-tested here
    // is the pure logic underneath both: `parse`, `rewrite`, `patch_in_place`, `canonical` and
    // the byte-scanner.

    #[test]
    fn a_missing_file_gets_one_entry_in_canonical_form() {
        let bytes = rewrite(None, "tickets", 3);

        assert_eq!(
            String::from_utf8(bytes).unwrap(),
            "{\n  \"tickets\": {\n    \"last\": 3\n  }\n}\n"
        );
    }

    #[test]
    fn an_entry_already_there_has_only_its_number_replaced() {
        let current = "{\n  \"tickets\": {\n    \"last\": 3\n  }\n}\n";

        let bytes = rewrite(Some(current), "tickets", 4);

        assert_eq!(
            String::from_utf8(bytes).unwrap(),
            "{\n  \"tickets\": {\n    \"last\": 4\n  }\n}\n"
        );
    }

    #[test]
    fn updating_an_entry_leaves_a_hand_formatted_file_otherwise_untouched() {
        // No trailing newline, one space of indentation, a comment-shaped string value that is
        // not touched: exactly what "leaves the rest of the file exactly as it was found" means.
        let current =
            "{\"rfcs\":{\"last\":7,\"note\":\"the last one was odd\"},\"tickets\":{\"last\":3}}";

        let bytes = rewrite(Some(current), "tickets", 9);

        assert_eq!(
            String::from_utf8(bytes).unwrap(),
            "{\"rfcs\":{\"last\":7,\"note\":\"the last one was odd\"},\"tickets\":{\"last\":9}}"
        );
    }

    #[test]
    fn updating_one_entry_does_not_touch_a_sibling_that_is_malformed() {
        let current = "{\"rfcs\":{\"last\":\"three\"},\"tickets\":{\"last\":3}}";

        let bytes = rewrite(Some(current), "tickets", 4);

        assert_eq!(
            String::from_utf8(bytes).unwrap(),
            "{\"rfcs\":{\"last\":\"three\"},\"tickets\":{\"last\":4}}"
        );
    }

    #[test]
    fn a_new_entry_is_added_in_canonical_form_and_existing_entries_survive() {
        let current = "{\n  \"tickets\": {\n    \"last\": 3\n  }\n}\n";

        let bytes = rewrite(Some(current), "rfcs", 1);

        assert_eq!(
            String::from_utf8(bytes).unwrap(),
            "{\n  \"rfcs\": {\n    \"last\": 1\n  },\n  \"tickets\": {\n    \"last\": 3\n  }\n}\n"
        );
    }

    #[test]
    fn adding_an_entry_puts_keys_in_alphabetical_order_even_when_the_file_did_not() {
        let current = "{\"tickets\":{\"last\":3},\"aaa\":{\"last\":1}}";

        let bytes = rewrite(Some(current), "mmm", 2);
        let text = String::from_utf8(bytes).unwrap();

        let aaa = text.find("\"aaa\"").unwrap();
        let mmm = text.find("\"mmm\"").unwrap();
        let tickets = text.find("\"tickets\"").unwrap();
        assert!(aaa < mmm && mmm < tickets, "{text}");
    }

    #[test]
    fn a_value_that_happens_to_spell_the_key_inside_a_string_is_not_mistaken_for_it() {
        let object = r#"{"note":"the last one","last":5}"#;

        let span = value_span(object, "last").unwrap();

        assert_eq!(&object[span.0..span.1], "5");
    }

    #[test]
    fn a_string_value_containing_braces_does_not_confuse_the_scanner() {
        let object = r#"{"a":"{not json}","last":5}"#;

        let span = value_span(object, "last").unwrap();

        assert_eq!(&object[span.0..span.1], "5");
    }

    #[test]
    fn a_nested_object_value_is_skipped_whole() {
        let object = r#"{"a":{"deep":{"x":1}},"last":5}"#;

        let span = value_span(object, "last").unwrap();

        assert_eq!(&object[span.0..span.1], "5");
    }

    #[test]
    fn a_key_that_is_not_there_is_none() {
        let object = r#"{"a":1}"#;

        assert_eq!(value_span(object, "last"), None);
    }

    #[test]
    fn text_that_is_not_an_object_at_all_falls_back_to_canonical() {
        let bytes = rewrite(Some("[1, 2, 3]"), "tickets", 1);

        assert_eq!(
            String::from_utf8(bytes).unwrap(),
            "{\n  \"tickets\": {\n    \"last\": 1\n  }\n}\n"
        );
    }

    #[test]
    fn an_entry_with_no_last_field_falls_back_to_canonical_for_that_entry() {
        let current = r#"{"tickets":{"note":"no last yet"}}"#;

        let bytes = rewrite(Some(current), "tickets", 1);

        assert_eq!(
            String::from_utf8(bytes).unwrap(),
            "{\n  \"tickets\": {\n    \"last\": 1\n  }\n}\n"
        );
    }

    #[test]
    fn a_malformed_last_is_read_apart_from_a_usable_one() {
        let found = parse(br#"{ "tickets": { "last": "three" }, "rfcs": { "last": 4 } }"#)
            .expect("valid JSON");

        assert_eq!(found.last.get("rfcs"), Some(&4));
        assert!(found.malformed.contains_key("tickets"));
        assert!(!found.last.contains_key("tickets"));
        assert!(found.has("tickets"));
        assert!(found.has("rfcs"));
        assert!(!found.has("notes"));
    }

    #[test]
    fn every_malformed_shape_is_read_as_malformed_and_not_as_a_usable_number() {
        let text = br#"{
            "text": { "last": "three" },
            "nothing": { "last": null },
            "negative": { "last": -5 },
            "fraction": { "last": 2.7 },
            "huge": { "last": 999999999999999999999999999999 }
        }"#;

        let found = parse(text).expect("valid JSON");

        for name in ["text", "nothing", "negative", "fraction", "huge"] {
            assert!(found.malformed.contains_key(name), "{name}");
            assert!(!found.last.contains_key(name), "{name}");
        }
    }

    /// A `last` field simply absent from an entry stays read as "not recorded", not as
    /// malformed: decision 13 only names a `last` that is present and wrong.
    #[test]
    fn an_entry_with_no_last_field_is_read_as_not_recorded() {
        let found = parse(br#"{ "tickets": { "note": "no last yet" } }"#).expect("valid JSON");

        assert!(!found.has("tickets"));
    }

    #[test]
    fn bytes_that_are_not_one_json_object_are_refused() {
        assert!(parse(b"[1, 2, 3]").is_err());
        assert!(parse(b"not json").is_err());
    }
}
