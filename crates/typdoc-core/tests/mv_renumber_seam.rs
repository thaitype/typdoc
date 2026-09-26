//! Covers SPC-2.
//!
//! `mv --renumber` writes the destination's `last` before the document appears under its new
//! key. `Project::mv_renumber` calls `write_state` and then `mv::commit` under one lock, with no
//! file operation between them, so a fake stopped at an exact operation count shows it without
//! a `Project`.

use std::path::{Path, PathBuf};
use std::time::Duration;

use typdoc_core::{ContentChange, NamespaceLock, acquire, commit, write_state};
use typdoc_testkit::fake::{FakeFs, FixedClock, Stage};

fn a_lock(fake: &FakeFs) -> NamespaceLock<'_> {
    acquire(
        fake,
        &FixedClock::new(),
        PathBuf::from("/project/.typdoc/locks/story-3.lock"),
        "mv-renumber-seam-test-host",
        Duration::from_secs(5),
    )
    .expect("nothing holds it yet")
}

fn changes() -> Vec<ContentChange> {
    vec![ContentChange {
        path: PathBuf::from("/project/story-1/notes/holder.md"),
        bytes: b"see: story-3:WF-1\n".to_vec(),
    }]
}

/// A skipped number is ordinary, not a partial write to repair.
#[test]
fn a_stop_right_after_the_state_write_skips_the_number_and_leaves_the_document_unmoved() {
    let fake = FakeFs::new();
    fake.put(
        Path::new("/project/old.md"),
        b"---\ntitle: old\n---\n",
        0o100_644,
    );
    fake.put(
        Path::new("/project/story-1/notes/holder.md"),
        b"see: WF-5\n",
        0o100_644,
    );
    let lock = a_lock(&fake);

    write_state(&fake, &lock, Path::new("/project"), "story-3", "tickets", 1)
        .expect("the state write itself is not staged to fail");

    fake.arm(Stage::StopAfter(0));
    let err = commit(
        &fake,
        &lock,
        &changes(),
        Path::new("/project/old.md"),
        Path::new("/project/story-3/tickets/WF-1.md"),
    )
    .expect_err("staged to stop before any operation of the commit itself");
    assert_eq!(err.to_string(), "the run stopped");

    let written = fake
        .bytes(Path::new("/project/.typdoc/state/story-3.json"))
        .expect("the state file exists: the write above completed before the stop");
    assert_eq!(
        String::from_utf8(written).unwrap(),
        "{\n  \"tickets\": {\n    \"last\": 1\n  }\n}\n",
        "the number `WF-1` was spent here and is never handed out again"
    );
    assert_eq!(
        fake.bytes(Path::new("/project/story-3/tickets/WF-1.md")),
        None,
        "the document never arrived under the number that was spent"
    );
    assert_eq!(
        fake.bytes(Path::new("/project/old.md")).as_deref(),
        Some(&b"---\ntitle: old\n---\n"[..]),
        "still at the old name, exactly where it was"
    );
    assert_eq!(
        fake.bytes(Path::new("/project/story-1/notes/holder.md"))
            .as_deref(),
        Some(&b"see: WF-5\n"[..]),
        "the holder's ref was never rewritten either: nothing of commit's own work ran"
    );
}

#[test]
fn a_stop_during_commits_own_prepare_phase_still_leaves_the_spent_number_recorded_once() {
    let fake = FakeFs::new();
    fake.put(
        Path::new("/project/old.md"),
        b"---\ntitle: old\n---\n",
        0o100_644,
    );
    fake.put(
        Path::new("/project/story-1/notes/holder.md"),
        b"see: WF-5\n",
        0o100_644,
    );
    let lock = a_lock(&fake);

    write_state(&fake, &lock, Path::new("/project"), "story-3", "tickets", 1)
        .expect("the state write itself is not staged to fail");

    // One operation into `commit`: the holder's mode has been read, and nothing written.
    fake.arm(Stage::StopAfter(1));
    let err = commit(
        &fake,
        &lock,
        &changes(),
        Path::new("/project/old.md"),
        Path::new("/project/story-3/tickets/WF-1.md"),
    )
    .expect_err("staged to stop one operation into commit's own prepare phase");
    assert_eq!(err.to_string(), "the run stopped");

    let written = fake
        .bytes(Path::new("/project/.typdoc/state/story-3.json"))
        .expect("the state file exists");
    assert_eq!(
        String::from_utf8(written).unwrap(),
        "{\n  \"tickets\": {\n    \"last\": 1\n  }\n}\n",
        "one write, not two: a later stop does not record the number again"
    );
    assert_eq!(
        fake.bytes(Path::new("/project/story-3/tickets/WF-1.md")),
        None
    );
}
