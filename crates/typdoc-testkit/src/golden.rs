//! Golden files: the `--json` output of a command, pinned. A case is a folder
//! `fixtures/output/<command>/<case>/` that holds
//!
//! - `case.json`, written by hand: the fixture project to run in (`project`, by its path below
//!   `fixtures/`) and the arguments of the run (`command`, after the program name);
//! - `assertions.json`, written by hand: a JSON object from a JSON pointer to the value the
//!   output holds there, for the fields that carry the load;
//! - `golden/stdout.json`, the only file the generator writes.
//!
//! A run must end with 0 and print nothing on standard error; its standard output is compared
//! with the golden as parsed JSON, so whitespace and the order of keys do not matter, and the
//! order of an array does: nothing is sorted and nothing is scrubbed before comparing. The
//! assertions are checked against the output itself, not against the golden, so a golden that
//! was regenerated wrongly is still caught by them.
//!
//! One thing this comparison cannot see. A `number` is printed with the digits written in the
//! document, and reading JSON turns a number into a primitive, so `1e3` and `1000.0` are one
//! value here and a golden records whichever form that primitive prints in. The digits
//! themselves are pinned where they can be seen, against the bytes on standard output, in
//! `crates/typdoc/tests/frontmatter_scalars.rs`.
//!
//! The generator writes one golden, named by `<command>/<case>`, and never a file whose parent
//! folder is not named `golden` or whose name is not `stdout.json`. It has no mode that writes
//! every golden, and no path by which it writes an assertion file.
//!
//! To regenerate one: `TYPDOC_REGENERATE_GOLDEN=<command>/<case> cargo test -p typdoc --test
//! golden regenerate -- --ignored`. The assertions of a case are written first.

use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};

use serde::Deserialize;
use serde_json::Value;

use crate::fixtures;

/// The variable that names the one golden to regenerate.
pub const REGENERATE_VAR: &str = "TYPDOC_REGENERATE_GOLDEN";

const GOLDEN_DIR: &str = "golden";
const GOLDEN_FILE: &str = "stdout.json";
const CASE_FILE: &str = "case.json";
const ASSERTIONS_FILE: &str = "assertions.json";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CaseSpec {
    project: String,
    command: Vec<String>,
    /// A variable the case's run needs set, by name. A case whose command stamps an
    /// `auto: create`/`auto: update` field uses it to name the instant the shipped binary's
    /// clock reports instead of the machine's own (`typdoc::clock::FIXED_CLOCK_VAR`).
    #[serde(default)]
    env: BTreeMap<String, String>,
}

/// One golden case: what to run, and where its files are.
#[derive(Debug)]
pub struct Case {
    /// `<command>/<case>`
    pub id: String,
    pub dir: PathBuf,
    /// The fixture project to run in, by its path below `fixtures/`.
    pub project: String,
    /// The arguments of the run, after the program name.
    pub command: Vec<String>,
    /// A variable the run needs set, by name; empty when the case needs none (`CaseSpec::env`).
    pub env: BTreeMap<String, String>,
}

impl Case {
    fn load(root: &Path, id: &str) -> Result<Case, String> {
        let dir = root.join(id);
        let file = dir.join(CASE_FILE);
        let text =
            std::fs::read_to_string(&file).map_err(|e| format!("{}: {e}", file.display()))?;
        let spec: CaseSpec =
            serde_json::from_str(&text).map_err(|e| format!("{}: {e}", file.display()))?;
        if spec.command.is_empty() {
            return Err(format!("{}: `command` is empty", file.display()));
        }
        Ok(Case {
            id: id.to_owned(),
            dir,
            project: spec.project,
            command: spec.command,
            env: spec.env,
        })
    }

    pub fn golden_path(&self) -> PathBuf {
        self.dir.join(GOLDEN_DIR).join(GOLDEN_FILE)
    }

    fn assertions_path(&self) -> PathBuf {
        self.dir.join(ASSERTIONS_FILE)
    }

    /// Compares `actual`, the standard output of the case's run parsed as JSON, with the golden
    /// and with the assertions. Every problem is reported, and a missing file is a problem.
    pub fn check(&self, actual: &Value) -> Result<(), String> {
        let mut problems = Vec::new();
        match read_json(&self.assertions_path()) {
            Ok(assertions) => {
                if let Err(e) = check_assertions(&assertions, actual) {
                    problems.push(format!("{}: {e}", self.assertions_path().display()));
                }
            }
            Err(e) => problems.push(e),
        }
        match read_json(&self.golden_path()) {
            Ok(golden) => {
                if let Err(e) = compare(&golden, actual) {
                    problems.push(format!("{}: {e}", self.golden_path().display()));
                }
            }
            Err(e) => problems.push(format!(
                "{e} (write the assertions, then regenerate this one golden: {REGENERATE_VAR}={} \
                 scripts/test.sh -p typdoc --test golden regenerate -- --ignored)",
                self.id
            )),
        }
        if problems.is_empty() {
            Ok(())
        } else {
            Err(problems.join("\n"))
        }
    }
}

fn read_json(file: &Path) -> Result<Value, String> {
    let text = std::fs::read_to_string(file).map_err(|e| format!("{}: {e}", file.display()))?;
    serde_json::from_str(&text).map_err(|e| format!("{}: {e}", file.display()))
}

/// Every case below `root`, sorted by id. A file where a folder is expected, and a case folder
/// with no `case.json`, are errors: nothing that sits in the tree is passed over.
pub fn discover(root: &Path) -> Result<Vec<Case>, String> {
    let mut cases = Vec::new();
    for command in entries(root)? {
        for name in entries(&root.join(&command))? {
            cases.push(Case::load(root, &format!("{command}/{name}"))?);
        }
    }
    Ok(cases)
}

/// Every case of `fixtures/output/`. The folder being absent stops the test with the reason.
pub fn discover_fixtures() -> Result<Vec<Case>, String> {
    discover(&fixtures::path("output"))
}

/// The names of the folders in `dir`, sorted. Anything else in it is an error.
fn entries(dir: &Path) -> Result<Vec<String>, String> {
    let mut names = Vec::new();
    let read = std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    for entry in read {
        let entry = entry.map_err(|e| format!("{}: {e}", dir.display()))?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|name| format!("{name:?} in {} is not UTF-8", dir.display()))?;
        let is_dir = entry
            .file_type()
            .map_err(|e| format!("{}: {e}", entry.path().display()))?
            .is_dir();
        if !is_dir {
            return Err(format!(
                "{} is not a folder: only <command>/<case>/ folders belong here",
                entry.path().display()
            ));
        }
        names.push(name);
    }
    names.sort();
    Ok(names)
}

/// The first difference between the golden `expected` and `actual`, as parsed JSON: the order
/// of keys does not matter, the order of an array does, and no value is set aside.
pub fn compare(expected: &Value, actual: &Value) -> Result<(), String> {
    match difference(expected, actual, "") {
        None => Ok(()),
        Some(problem) => Err(problem),
    }
}

fn pointer_segment(key: &str) -> String {
    key.replace('~', "~0").replace('/', "~1")
}

fn difference(expected: &Value, actual: &Value, at: &str) -> Option<String> {
    let place = if at.is_empty() { "/" } else { at };
    match (expected, actual) {
        (Value::Object(expected), Value::Object(actual)) => {
            for (key, value) in expected {
                let here = format!("{at}/{}", pointer_segment(key));
                match actual.get(key) {
                    None => return Some(format!("{here}: missing, the golden has {value}")),
                    Some(found) => {
                        if let Some(problem) = difference(value, found, &here) {
                            return Some(problem);
                        }
                    }
                }
            }
            actual
                .iter()
                .find(|(key, _)| !expected.contains_key(*key))
                .map(|(key, found)| {
                    format!(
                        "{at}/{}: not in the golden, the output has {found}",
                        pointer_segment(key)
                    )
                })
        }
        (Value::Array(expected), Value::Array(actual)) => {
            for (index, (value, found)) in expected.iter().zip(actual).enumerate() {
                if let Some(problem) = difference(value, found, &format!("{at}/{index}")) {
                    return Some(problem);
                }
            }
            (expected.len() != actual.len()).then(|| {
                format!(
                    "{place}: the golden has {} items, the output has {}",
                    expected.len(),
                    actual.len()
                )
            })
        }
        _ => (expected != actual)
            .then(|| format!("{place}: the golden has {expected}, the output has {actual}")),
    }
}

/// Checks each assertion, a JSON pointer and the value it must find, against `actual`. An
/// assertion file that asserts nothing is refused: it would be a net with no mesh.
pub fn check_assertions(assertions: &Value, actual: &Value) -> Result<(), String> {
    let Value::Object(assertions) = assertions else {
        return Err("assertions are a JSON object from a JSON pointer to a value".into());
    };
    if assertions.is_empty() {
        return Err("no assertion is written".into());
    }
    let mut problems = Vec::new();
    for (pointer, expected) in assertions {
        match actual.pointer(pointer) {
            None => problems.push(format!("{pointer}: the output has nothing there")),
            Some(found) => {
                if let Err(problem) = compare(expected, found) {
                    problems.push(format!("{pointer}: {problem}"));
                }
            }
        }
    }
    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems.join("; "))
    }
}

/// The one place a golden is written. It refuses any target whose parent folder is not named
/// `golden`, whose name is not `stdout.json`, that has `..` in it, or that is or sits behind a
/// symbolic link, so that no assertion file can be reached through it. The file is made whole
/// beside its target and renamed onto it, so a link that the target or a stale temporary file
/// is leaves what it points to as it was.
pub fn write_golden(target: &Path, value: &Value) -> Result<(), String> {
    let refuse = |why: &str| Err(format!("refused to write {}: {why}", target.display()));
    if target.components().any(|c| c == Component::ParentDir) {
        return refuse("the path goes through `..`");
    }
    if target.file_name().and_then(|n| n.to_str()) != Some(GOLDEN_FILE) {
        return refuse(&format!("a golden is a file named {GOLDEN_FILE}"));
    }
    let Some(folder) = target.parent() else {
        return refuse("there is no folder around it");
    };
    if folder.file_name().and_then(|n| n.to_str()) != Some(GOLDEN_DIR) {
        return refuse(&format!(
            "a golden is written inside a folder named {GOLDEN_DIR}"
        ));
    }
    match std::fs::symlink_metadata(folder) {
        Ok(meta) if meta.is_dir() => {}
        Ok(_) => return refuse(&format!("{GOLDEN_DIR} is not a plain folder")),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            std::fs::create_dir(folder).map_err(|e| format!("{}: {e}", folder.display()))?;
        }
        Err(e) => return Err(format!("{}: {e}", folder.display())),
    }
    if let Ok(meta) = std::fs::symlink_metadata(target)
        && !meta.is_file()
    {
        return refuse("it exists and is not a plain file");
    }
    let mut text = serde_json::to_string_pretty(value).map_err(|e| e.to_string())?;
    text.push('\n');
    // The name of a stale temporary file is removed, not written through, and the new one is
    // made only if it does not exist, so a link left there reaches nothing. The rename puts
    // the new file in place of the target's name and never writes into what the target names.
    let temporary = folder.join(format!(".{GOLDEN_FILE}.tmp"));
    match std::fs::remove_file(&temporary) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(format!("{}: {e}", temporary.display())),
    }
    let written = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .and_then(|mut file| std::io::Write::write_all(&mut file, text.as_bytes()));
    written
        .and_then(|()| std::fs::rename(&temporary, target))
        .map_err(|e| {
            let _ = std::fs::remove_file(&temporary);
            format!("{}: {e}", target.display())
        })
}

/// Regenerates the one golden that `id` names, `<command>/<case>`, from the output `run` gives
/// for the case, and returns the file written. The case and its assertions must exist already.
/// There is no id that means every case.
pub fn regenerate(
    root: &Path,
    id: &str,
    run: impl FnOnce(&Case) -> Result<Value, String>,
) -> Result<PathBuf, String> {
    let segments: Vec<&str> = id.split('/').collect();
    let plain = |segment: &str| {
        !segment.is_empty()
            && segment
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
    };
    if segments.len() != 2 || !segments.iter().all(|s| plain(s)) {
        return Err(format!(
            "`{id}` does not name one golden: write <command>/<case>, with lowercase letters, \
             digits, `-` and `_`"
        ));
    }
    let case = Case::load(root, id)?;
    if !case.assertions_path().is_file() {
        return Err(format!(
            "{} is missing: the assertions are written by hand before the golden",
            case.assertions_path().display()
        ));
    }
    let output = run(&case)?;
    check_assertions(&read_json(&case.assertions_path())?, &output).map_err(|e| {
        format!(
            "{}: the output breaks the assertions, so no golden is written: {e}",
            case.id
        )
    })?;
    write_golden(&case.golden_path(), &output)?;
    Ok(case.golden_path())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    /// A case tree in a scratch folder: `<root>/get/one/` with the three files.
    struct Tree {
        dir: tempfile::TempDir,
    }

    impl Tree {
        fn new() -> Tree {
            let tree = Tree {
                dir: tempfile::tempdir().unwrap(),
            };
            tree.put(
                "get/one/case.json",
                r#"{ "project": "valid/minimal", "command": ["get", "note.md", "--json"] }"#,
            );
            tree.put(
                "get/one/assertions.json",
                r#"{ "/document/path": "note.md" }"#,
            );
            tree.put(
                "get/one/golden/stdout.json",
                r#"{ "document": { "path": "note.md", "fields": { "tags": ["a", "b"] } } }"#,
            );
            tree
        }

        fn put(&self, path: &str, text: &str) {
            let file = self.dir.path().join(path);
            std::fs::create_dir_all(file.parent().unwrap()).unwrap();
            std::fs::write(file, text).unwrap();
        }

        fn read(&self, path: &str) -> String {
            std::fs::read_to_string(self.dir.path().join(path)).unwrap()
        }

        fn root(&self) -> &Path {
            self.dir.path()
        }

        fn case(&self) -> Case {
            Case::load(self.root(), "get/one").unwrap()
        }
    }

    fn output() -> Value {
        json!({ "document": { "path": "note.md", "fields": { "tags": ["a", "b"] } } })
    }

    #[test]
    fn output_equal_to_the_golden_and_the_assertions_is_green() {
        assert_eq!(Tree::new().case().check(&output()), Ok(()));
    }

    #[test]
    fn whitespace_and_the_order_of_keys_do_not_matter() {
        let tree = Tree::new();
        tree.put(
            "get/one/golden/stdout.json",
            "{\"document\":{\"fields\":{\"tags\":[\"a\",\"b\"]},\n\n \"path\":\"note.md\"}}",
        );

        assert_eq!(tree.case().check(&output()), Ok(()));
    }

    #[test]
    fn a_golden_changed_on_purpose_is_red_and_says_where() {
        let tree = Tree::new();
        tree.put(
            "get/one/golden/stdout.json",
            r#"{ "document": { "path": "other.md", "fields": { "tags": ["a", "b"] } } }"#,
        );

        let error = tree.case().check(&output()).unwrap_err();

        assert!(error.contains("/document/path"), "{error}");
        assert!(error.contains("other.md"), "{error}");
    }

    #[test]
    fn an_array_in_the_wrong_order_is_red() {
        let tree = Tree::new();
        tree.put(
            "get/one/golden/stdout.json",
            r#"{ "document": { "path": "note.md", "fields": { "tags": ["b", "a"] } } }"#,
        );

        let error = tree.case().check(&output()).unwrap_err();

        assert!(error.contains("/document/fields/tags/0"), "{error}");
    }

    #[test]
    fn objects_in_an_array_are_not_sorted_before_comparing() {
        let one = json!({ "findings": [{ "path": "a.md" }, { "path": "b.md" }] });
        let swapped = json!({ "findings": [{ "path": "b.md" }, { "path": "a.md" }] });

        assert!(compare(&one, &swapped).is_err());
        assert_eq!(compare(&one, &one), Ok(()));
    }

    #[test]
    fn an_array_with_more_or_fewer_items_is_red() {
        let two = json!({ "tags": ["a", "b"] });

        let shorter = compare(&two, &json!({ "tags": ["a"] })).unwrap_err();
        let longer = compare(&two, &json!({ "tags": ["a", "b", "c"] })).unwrap_err();

        assert!(
            shorter.contains("2 items") && shorter.contains("has 1"),
            "{shorter}"
        );
        assert!(
            longer.contains("2 items") && longer.contains("has 3"),
            "{longer}"
        );
    }

    #[test]
    fn a_key_missing_from_the_output_and_a_key_not_in_the_golden_are_both_red() {
        let golden = json!({ "a": 1 });

        assert!(
            compare(&golden, &json!({}))
                .unwrap_err()
                .contains("/a: missing")
        );
        assert!(
            compare(&golden, &json!({ "a": 1, "b": 2 }))
                .unwrap_err()
                .contains("/b: not in the golden")
        );
    }

    #[test]
    fn nothing_is_scrubbed_a_value_that_looks_like_a_time_is_compared_like_any_other() {
        let golden = json!({ "created_at": "2026-09-20T12:00:00Z" });
        let later = json!({ "created_at": "2026-09-20T12:00:01Z" });

        assert!(compare(&golden, &later).is_err());
    }

    #[test]
    fn a_number_and_the_text_of_that_number_are_not_the_same_value() {
        assert!(compare(&json!({ "n": 1 }), &json!({ "n": "1" })).is_err());
    }

    #[test]
    fn a_missing_golden_is_red_and_names_the_way_to_regenerate_that_one() {
        let tree = Tree::new();
        std::fs::remove_file(tree.root().join("get/one/golden/stdout.json")).unwrap();

        let error = tree.case().check(&output()).unwrap_err();

        assert!(error.contains("golden/stdout.json"), "{error}");
        assert!(
            error.contains("TYPDOC_REGENERATE_GOLDEN=get/one"),
            "{error}"
        );
    }

    #[test]
    fn a_missing_or_empty_assertion_file_is_red_even_when_the_golden_matches() {
        let tree = Tree::new();
        std::fs::remove_file(tree.root().join("get/one/assertions.json")).unwrap();
        assert!(
            tree.case()
                .check(&output())
                .unwrap_err()
                .contains("assertions.json")
        );

        tree.put("get/one/assertions.json", "{}");
        assert!(
            tree.case()
                .check(&output())
                .unwrap_err()
                .contains("no assertion")
        );
    }

    #[test]
    fn an_assertion_catches_a_golden_regenerated_from_wrong_output() {
        let tree = Tree::new();
        let wrong = json!({ "document": { "path": "wrong.md", "fields": { "tags": ["a", "b"] } } });
        tree.put(
            "get/one/golden/stdout.json",
            &serde_json::to_string(&wrong).unwrap(),
        );

        let error = tree.case().check(&wrong).unwrap_err();

        assert!(error.contains("assertions.json"), "{error}");
        assert!(error.contains("/document/path"), "{error}");
    }

    #[test]
    fn an_assertion_at_a_place_the_output_does_not_have_is_red() {
        let tree = Tree::new();
        tree.put("get/one/assertions.json", r#"{ "/document/key": "WF-1" }"#);

        let error = tree.case().check(&output()).unwrap_err();

        assert!(error.contains("/document/key"), "{error}");
    }

    #[test]
    fn the_generator_refuses_to_write_an_assertion_file() {
        let tree = Tree::new();
        let assertions = tree.root().join("get/one/assertions.json");
        let before = tree.read("get/one/assertions.json");

        let error = write_golden(&assertions, &json!({ "/document/path": "x" })).unwrap_err();

        assert!(error.contains("refused"), "{error}");
        assert_eq!(tree.read("get/one/assertions.json"), before);
    }

    #[test]
    fn the_generator_refuses_a_case_file_and_a_name_other_than_the_golden_inside_golden() {
        let tree = Tree::new();
        tree.put("get/one/golden/assertions.json", "{}");

        for target in ["get/one/case.json", "get/one/golden/assertions.json"] {
            let error = write_golden(&tree.root().join(target), &json!({})).unwrap_err();
            assert!(error.contains("refused"), "{target}: {error}");
        }
        assert_eq!(tree.read("get/one/golden/assertions.json"), "{}");
    }

    #[test]
    fn the_generator_refuses_a_path_that_goes_up_through_dot_dot() {
        let tree = Tree::new();
        let sneaky = tree.root().join("get/one/golden/../golden/stdout.json");

        let error = write_golden(&sneaky, &json!({})).unwrap_err();

        assert!(error.contains(".."), "{error}");
    }

    #[test]
    fn the_generator_refuses_a_golden_that_is_a_link_to_an_assertion_file() {
        let tree = Tree::new();
        let golden = tree.root().join("get/one/golden/stdout.json");
        std::fs::remove_file(&golden).unwrap();
        std::os::unix::fs::symlink("../assertions.json", &golden).unwrap();
        let before = tree.read("get/one/assertions.json");

        let error = write_golden(&golden, &json!({ "x": 1 })).unwrap_err();

        assert!(error.contains("refused"), "{error}");
        assert_eq!(tree.read("get/one/assertions.json"), before);
    }

    #[test]
    fn a_stale_temporary_file_that_links_to_an_assertion_file_is_not_written_through() {
        let tree = Tree::new();
        let temporary = tree.root().join("get/one/golden/.stdout.json.tmp");
        std::os::unix::fs::symlink("../assertions.json", &temporary).unwrap();
        let before = tree.read("get/one/assertions.json");

        write_golden(
            &tree.root().join("get/one/golden/stdout.json"),
            &json!({ "x": 1 }),
        )
        .unwrap();

        assert_eq!(tree.read("get/one/assertions.json"), before);
        assert_eq!(
            tree.read("get/one/golden/stdout.json").trim(),
            "{\n  \"x\": 1\n}"
        );
    }

    #[test]
    fn a_stale_temporary_file_that_is_a_hard_link_to_an_assertion_file_is_not_written_through() {
        let tree = Tree::new();
        std::fs::hard_link(
            tree.root().join("get/one/assertions.json"),
            tree.root().join("get/one/golden/.stdout.json.tmp"),
        )
        .unwrap();
        let before = tree.read("get/one/assertions.json");

        write_golden(
            &tree.root().join("get/one/golden/stdout.json"),
            &json!({ "x": 1 }),
        )
        .unwrap();

        assert_eq!(tree.read("get/one/assertions.json"), before);
    }

    #[test]
    fn a_golden_that_is_a_hard_link_to_an_assertion_file_is_replaced_and_the_assertions_stay() {
        let tree = Tree::new();
        let golden = tree.root().join("get/one/golden/stdout.json");
        std::fs::remove_file(&golden).unwrap();
        std::fs::hard_link(tree.root().join("get/one/assertions.json"), &golden).unwrap();
        let before = tree.read("get/one/assertions.json");

        write_golden(&golden, &json!({ "x": 1 })).unwrap();

        assert_eq!(tree.read("get/one/assertions.json"), before);
        assert_eq!(
            serde_json::from_str::<Value>(&tree.read("get/one/golden/stdout.json")).unwrap(),
            json!({ "x": 1 })
        );
    }

    #[test]
    fn a_golden_is_not_written_from_output_that_breaks_the_assertions() {
        let tree = Tree::new();
        let before = tree.read("get/one/golden/stdout.json");

        let error = regenerate(tree.root(), "get/one", |_| {
            Ok(json!({ "document": { "path": "wrong.md" } }))
        })
        .unwrap_err();

        assert!(error.contains("/document/path"), "{error}");
        assert_eq!(tree.read("get/one/golden/stdout.json"), before);
    }

    #[test]
    fn a_golden_is_written_whole_and_no_temporary_file_is_left() {
        let tree = Tree::new();

        write_golden(
            &tree.root().join("get/one/golden/stdout.json"),
            &json!({ "x": 1 }),
        )
        .unwrap();

        assert!(!tree.root().join("get/one/golden/.stdout.json.tmp").exists());
        assert_eq!(
            serde_json::from_str::<Value>(&tree.read("get/one/golden/stdout.json")).unwrap(),
            json!({ "x": 1 })
        );
    }

    #[test]
    fn the_generator_refuses_a_golden_folder_that_is_a_link() {
        let tree = Tree::new();
        std::fs::remove_dir_all(tree.root().join("get/one/golden")).unwrap();
        tree.put("elsewhere/keep", "");
        std::os::unix::fs::symlink("../../elsewhere", tree.root().join("get/one/golden")).unwrap();

        let error =
            write_golden(&tree.root().join("get/one/golden/stdout.json"), &json!({})).unwrap_err();

        assert!(error.contains("refused"), "{error}");
        assert!(!tree.root().join("elsewhere/stdout.json").exists());
    }

    #[test]
    fn regenerating_one_golden_writes_that_file_and_only_that_file() {
        let tree = Tree::new();
        tree.put(
            "get/two/case.json",
            r#"{ "project": "p", "command": ["get"] }"#,
        );
        tree.put("get/two/assertions.json", r#"{ "/a": 1 }"#);
        tree.put("get/two/golden/stdout.json", r#"{ "a": "untouched" }"#);
        let fresh = json!({ "document": { "path": "note.md", "fields": { "tags": ["c"] } } });

        let written = regenerate(tree.root(), "get/one", |case| {
            assert_eq!(case.command, ["get", "note.md", "--json"]);
            Ok(fresh.clone())
        })
        .unwrap();

        assert_eq!(written, tree.root().join("get/one/golden/stdout.json"));
        let text = tree.read("get/one/golden/stdout.json");
        assert!(text.ends_with("}\n"), "{text:?}");
        assert_eq!(serde_json::from_str::<Value>(&text).unwrap(), fresh);
        assert_eq!(
            tree.read("get/two/golden/stdout.json"),
            r#"{ "a": "untouched" }"#
        );
        assert_eq!(
            tree.read("get/one/assertions.json"),
            r#"{ "/document/path": "note.md" }"#
        );
    }

    #[test]
    fn there_is_no_way_to_regenerate_every_golden() {
        let tree = Tree::new();
        let before = tree.read("get/one/golden/stdout.json");

        for id in [
            "",
            "*",
            "all",
            "get",
            "get/*",
            "get/",
            "/one",
            "get/one/x",
            "../get/one",
            "get/..",
        ] {
            let result = regenerate(tree.root(), id, |_| panic!("`{id}` ran a case"));
            assert!(result.is_err(), "`{id}` was accepted");
        }
        assert_eq!(tree.read("get/one/golden/stdout.json"), before);
    }

    #[test]
    fn a_golden_is_not_regenerated_before_its_assertions_are_written() {
        let tree = Tree::new();
        std::fs::remove_file(tree.root().join("get/one/assertions.json")).unwrap();

        let error = regenerate(tree.root(), "get/one", |_| panic!("ran a case")).unwrap_err();

        assert!(error.contains("assertions"), "{error}");
    }

    #[test]
    fn a_failed_run_writes_nothing() {
        let tree = Tree::new();
        let before = tree.read("get/one/golden/stdout.json");

        let error =
            regenerate(tree.root(), "get/one", |_| Err("the run failed".into())).unwrap_err();

        assert_eq!(error, "the run failed");
        assert_eq!(tree.read("get/one/golden/stdout.json"), before);
    }

    #[test]
    fn every_case_is_found_and_a_stray_file_or_a_case_with_no_spec_is_red() {
        let tree = Tree::new();
        tree.put(
            "list/two/case.json",
            r#"{ "project": "p", "command": ["list"] }"#,
        );

        let ids: Vec<String> = discover(tree.root())
            .unwrap()
            .into_iter()
            .map(|c| c.id)
            .collect();
        assert_eq!(ids, ["get/one", "list/two"]);

        tree.put("stray.json", "{}");
        assert!(discover(tree.root()).unwrap_err().contains("stray.json"));
        std::fs::remove_file(tree.root().join("stray.json")).unwrap();

        std::fs::create_dir(tree.root().join("get/empty")).unwrap();
        assert!(discover(tree.root()).unwrap_err().contains("case.json"));
    }

    #[test]
    fn a_case_spec_with_an_unknown_key_or_no_command_is_refused() {
        let tree = Tree::new();
        tree.put(
            "get/one/case.json",
            r#"{ "project": "p", "command": ["get"], "setup": "rm" }"#,
        );
        assert!(Case::load(tree.root(), "get/one").is_err());

        tree.put("get/one/case.json", r#"{ "project": "p", "command": [] }"#);
        assert!(
            Case::load(tree.root(), "get/one")
                .unwrap_err()
                .contains("empty")
        );
    }
}
