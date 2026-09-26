//! Covers SPC-10.
//!
//! `find_leftovers` is a read, so it is checked against a real temporary directory — a leftover
//! is manufactured directly by writing it with `std::fs::write`, outside typdoc entirely, since
//! nothing ships yet that would leave one by being killed mid-write. `remove_leftovers` is a
//! write, so it goes through `Fs` and is checked against the fake, which can stage a removal to
//! fail on request; a real directory has no ordinary way to make `remove_file` fail on one
//! particular name.

use std::path::PathBuf;
use std::time::Duration;

use tempfile::TempDir;
use typdoc_core::{TEMP_PREFIX, acquire, find_leftovers, remove_leftovers, write_atomically};
use typdoc_testkit::fake::{Failure, FakeFs, FixedClock, On, Stage};

fn a_lock<'a>(fs: &'a FakeFs, path: &str) -> typdoc_core::NamespaceLock<'a> {
    acquire(
        fs,
        &FixedClock::new(),
        PathBuf::from(path),
        "leftovers-test-host",
        Duration::from_secs(5),
    )
    .unwrap_or_else(|e| panic!("the scenario's own lock could not be acquired: {e}"))
}

#[test]
fn find_leftovers_finds_a_leftover_at_the_top_and_one_nested_a_directory_down() {
    let dir = TempDir::new().expect("a temporary directory");
    let root = dir.path();
    std::fs::write(root.join("a.md"), "---\n---\n").unwrap();
    std::fs::create_dir_all(root.join("tickets")).unwrap();
    std::fs::write(root.join("tickets/WF-1.md"), "---\n---\n").unwrap();
    let top = root.join(format!("{TEMP_PREFIX}111-aaa"));
    let nested = root.join("tickets").join(format!("{TEMP_PREFIX}222-bbb"));
    std::fs::write(&top, "").unwrap();
    std::fs::write(&nested, "").unwrap();

    let mut found = find_leftovers(root);
    found.sort();
    let mut expected = vec![top, nested];
    expected.sort();

    assert_eq!(found, expected);
}

#[test]
fn find_leftovers_never_follows_a_symbolic_link() {
    let dir = TempDir::new().expect("a temporary directory");
    let root = dir.path();
    std::fs::create_dir_all(root.join("real")).unwrap();
    let real_leftover = root.join("real").join(format!("{TEMP_PREFIX}333-ccc"));
    std::fs::write(&real_leftover, "").unwrap();
    std::os::unix::fs::symlink(root.join("real"), root.join("linked")).unwrap();
    // A symbolic link straight to a leftover-shaped name: not followed either, and its own
    // name (however it is spelled) is never returned, since it is a link and not a file.
    std::os::unix::fs::symlink(&real_leftover, root.join(format!("{TEMP_PREFIX}444-ddd"))).unwrap();

    let found = find_leftovers(root);

    assert_eq!(found, vec![real_leftover]);
}

#[test]
fn find_leftovers_does_not_enter_a_folder_that_holds_its_own_project() {
    let dir = TempDir::new().expect("a temporary directory");
    let root = dir.path();
    std::fs::create_dir_all(root.join("imported/.typdoc")).unwrap();
    std::fs::write(root.join("imported/.typdoc/config.json"), "{}").unwrap();
    std::fs::write(
        root.join("imported").join(format!("{TEMP_PREFIX}555-eee")),
        "",
    )
    .unwrap();
    let outside = root.join(format!("{TEMP_PREFIX}666-fff"));
    std::fs::write(&outside, "").unwrap();

    let found = find_leftovers(root);

    assert_eq!(
        found,
        vec![outside],
        "a separate project's own leftovers are its own to find"
    );
}

#[test]
fn remove_leftovers_removes_every_path_it_is_given_through_the_seam() {
    let fs = FakeFs::new();
    let lock = a_lock(&fs, "/project/.typdoc/locks/outside.lock");
    fs.put(
        std::path::Path::new("/project/.typdoc-tmp-1-a"),
        b"",
        0o100_644,
    );
    fs.put(
        std::path::Path::new("/project/.typdoc-tmp-2-b"),
        b"",
        0o100_644,
    );
    let paths = vec![
        PathBuf::from("/project/.typdoc-tmp-1-a"),
        PathBuf::from("/project/.typdoc-tmp-2-b"),
    ];

    let removed = remove_leftovers(&fs, &lock, &paths);

    assert_eq!(removed, 2);
    assert_eq!(fs.bytes(&paths[0]), None);
    assert_eq!(fs.bytes(&paths[1]), None);
}

#[test]
fn remove_leftovers_counts_only_what_actually_came_off() {
    let fs = FakeFs::new();
    let lock = a_lock(&fs, "/project/.typdoc/locks/outside.lock");
    fs.put(
        std::path::Path::new("/project/.typdoc-tmp-1-a"),
        b"",
        0o100_644,
    );
    // The second path names nothing: `remove_leftovers` never fails the sweep over it, the
    // same as any other removal that does not come off.
    let paths = vec![
        PathBuf::from("/project/.typdoc-tmp-1-a"),
        PathBuf::from("/project/.typdoc-tmp-absent"),
    ];

    let removed = remove_leftovers(&fs, &lock, &paths);

    assert_eq!(removed, 1);
}

#[test]
fn a_removal_made_to_fail_leaves_the_write_successful() {
    let fs = FakeFs::new();
    let lock = a_lock(&fs, "/project/.typdoc/locks/outside.lock");
    fs.put(
        std::path::Path::new("/project/.typdoc-tmp-1-a"),
        b"",
        0o100_644,
    );
    let paths = vec![PathBuf::from("/project/.typdoc-tmp-1-a")];

    fs.arm(Stage::Fail(On::Remove, Failure::PermissionDenied));

    let removed = remove_leftovers(&fs, &lock, &paths);
    assert_eq!(removed, 0, "a removal staged to fail removes nothing");
    assert!(
        fs.bytes(&paths[0]).is_some(),
        "the leftover the failed removal was told to remove is still there"
    );

    let written = write_atomically(
        &fs,
        &lock,
        std::path::Path::new("/project/note.md"),
        b"body",
    );
    written.expect(
        "the write itself never touches `remove_file` on the destination, so a \
        removal staged to fail has no bearing on it",
    );
    assert_eq!(
        fs.bytes(std::path::Path::new("/project/note.md"))
            .as_deref(),
        Some(&b"body"[..])
    );
}
