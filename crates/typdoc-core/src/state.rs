//! The state file, `.typdoc/state/<namespace>.json`: the highest number issued per collection
//! in one namespace (SPC-8).

use std::collections::{BTreeMap, BTreeSet};
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

/// An entry with no `last` at all is not recorded, as for `state.missing`; only a `last` that is
/// there and not a usable whole number is `state.malformed` (SPC-8).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct StateFile {
    pub last: BTreeMap<String, u64>,
    pub malformed: BTreeMap<String, String>,
}

impl StateFile {
    /// A record at all, usable or not: what tells `state.missing` from `state.malformed`.
    pub(crate) fn has(&self, collection: &str) -> bool {
        self.last.contains_key(collection) || self.malformed.contains_key(collection)
    }
}

/// A missing file is no record yet. A file that is not one JSON object has no config error id
/// (SPC-6) and stops the command, as an unreadable schema does.
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

/// Records `last` as `collection`'s number in `namespace`'s state file, atomically. The caller
/// decides the number.
///
/// Public so that `typdoc-fs`'s tests can run it against a real file system.
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
    // Text that is not UTF-8 cannot reach here: `read` parsed this file as JSON under the same
    // lock. `rewrite` would treat it as a missing file.
    let current_text = current
        .as_deref()
        .and_then(|bytes| std::str::from_utf8(bytes).ok());
    let bytes = rewrite(current_text, collection, last);
    let state_dir = root.join(STATE_DIR);
    fs.create_dir_all(&state_dir)
        .map_err(Error::io_at(&state_dir))?;
    write_atomically(fs, lock, &path, &bytes).map_err(Error::io_at(&path))
}

/// The bytes [`write()`] puts on disk (SPC-8). An entry already there has only its `last` value
/// replaced, in place, and every other byte is left as it was; otherwise the whole file is
/// written in canonical form. Text that is not one JSON object cannot reach here (see
/// [`write()`]) and gets a canonical file: returning bytes, this has no way to refuse.
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

/// Falls back to `{}` on a serialization failure that a map of this shape cannot produce.
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

/// Sorted by hand: `serde_json::Map` keeps insertion order once any crate in the build enables
/// `preserve_order`, and `typdoc` does, which feature unification carries to this crate.
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

/// The byte range of `key`'s value in `object`'s JSON text, found by walking its structure, so
/// that the key's spelling inside a string value is never taken for the key.
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

/// Escapes are skipped, not decoded: every key looked up is a plain name typdoc writes, and
/// skipping is enough to find the closing quote.
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

/// The state files that are `config.state-orphan` (SPC-8). The state file of an `excluded`
/// namespace is not an orphan.
pub(crate) fn orphans(
    root: &Path,
    namespaces: &[Namespace],
    excluded: &BTreeSet<String>,
) -> Result<Vec<String>, Error> {
    let dir = root.join(STATE_DIR);
    let entries = match fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(source) => return Err(Error::Io { file: dir, source }),
    };
    let known: BTreeSet<&str> = namespaces
        .iter()
        .map(|n| n.name.as_str())
        .chain(excluded.iter().map(String::as_str))
        .collect();
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

    // Only the logic that touches no file is tested here. `read`, `orphans` and `write` are
    // tested through the binary and, for `write`, through `typdoc-fs`, the crate allowed to write.

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
