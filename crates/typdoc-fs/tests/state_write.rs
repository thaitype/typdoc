//! Covers SPC-8.
//!
//! `typdoc_core::write_state` against a real directory: the glue the unit tests of `state.rs`
//! do not reach.

use std::fs;
use std::path::Path;
use std::time::Duration;

use typdoc_core::{Fs, acquire, write_state};
use typdoc_fs::SystemFs;
use typdoc_testkit::fake::FixedClock;

fn a_lock<'a>(fs: &'a dyn Fs, lock_path: &Path) -> typdoc_core::NamespaceLock<'a> {
    acquire(
        fs,
        &FixedClock::new(),
        lock_path.to_path_buf(),
        "state-write-test-host",
        Duration::from_secs(5),
    )
    .unwrap_or_else(|e| panic!("the test's own lock could not be acquired: {e}"))
}

#[test]
fn the_first_write_in_a_namespace_creates_the_state_directory_and_the_file() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let fs = SystemFs;
    let lock_path = dir.path().join("outside/lock");
    let lock = a_lock(&fs, &lock_path);

    write_state(&fs, &lock, dir.path(), "default", "tickets", 1)
        .unwrap_or_else(|e| panic!("the write failed: {e}"));

    let written = fs::read_to_string(dir.path().join(".typdoc/state/default.json"))
        .expect("the state file exists");
    assert_eq!(written, "{\n  \"tickets\": {\n    \"last\": 1\n  }\n}\n");
}

#[test]
fn updating_an_entry_already_there_leaves_a_hand_formatted_file_otherwise_untouched() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let fs = SystemFs;
    let lock_path = dir.path().join("outside/lock");
    let lock = a_lock(&fs, &lock_path);
    fs::create_dir_all(dir.path().join(".typdoc/state")).expect("the state dir");
    // No indentation, no trailing newline, an entry for another collection kept exactly as
    // written: none of this is typdoc's own canonical form, on purpose.
    fs::write(
        dir.path().join(".typdoc/state/default.json"),
        "{\"rfcs\":{\"last\":7},\"tickets\":{\"last\":3}}",
    )
    .expect("the setup file");

    write_state(&fs, &lock, dir.path(), "default", "tickets", 4)
        .unwrap_or_else(|e| panic!("the write failed: {e}"));

    let written = fs::read_to_string(dir.path().join(".typdoc/state/default.json"))
        .expect("the state file exists");
    assert_eq!(written, "{\"rfcs\":{\"last\":7},\"tickets\":{\"last\":4}}");
}

#[test]
fn adding_an_entry_reformats_the_whole_file_into_canonical_form() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let fs = SystemFs;
    let lock_path = dir.path().join("outside/lock");
    let lock = a_lock(&fs, &lock_path);
    fs::create_dir_all(dir.path().join(".typdoc/state")).expect("the state dir");
    fs::write(
        dir.path().join(".typdoc/state/default.json"),
        "{\"tickets\":{\"last\":3}}",
    )
    .expect("the setup file");

    write_state(&fs, &lock, dir.path(), "default", "rfcs", 1)
        .unwrap_or_else(|e| panic!("the write failed: {e}"));

    let written = fs::read_to_string(dir.path().join(".typdoc/state/default.json"))
        .expect("the state file exists");
    assert_eq!(
        written,
        "{\n  \"rfcs\": {\n    \"last\": 1\n  },\n  \"tickets\": {\n    \"last\": 3\n  }\n}\n"
    );
}

#[test]
fn a_successful_write_leaves_no_temp_file_behind() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let fs = SystemFs;
    let lock_path = dir.path().join("outside/lock");
    let lock = a_lock(&fs, &lock_path);

    write_state(&fs, &lock, dir.path(), "default", "tickets", 1)
        .unwrap_or_else(|e| panic!("the write failed: {e}"));

    let names: Vec<String> = fs::read_dir(dir.path().join(".typdoc/state"))
        .expect("the state dir")
        .map(|entry| {
            entry
                .expect("an entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    assert_eq!(names, vec!["default.json".to_owned()], "{names:?}");
}
