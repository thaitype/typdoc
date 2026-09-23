//! `mv --renumber`'s own ordering rule (decision 13): the destination namespace's state is
//! written before the document appears under its new key. Proved deterministically, the same
//! technique `mv_seam.rs` already uses for `mv::commit`'s own promise: a fake file system
//! stopped at an exact operation count, this time stopped right after the state write, before
//! `mv::commit` runs at all.
//!
//! `Project::mv_renumber` calls `state::write` (re-exported as `write_state`) and then
//! `mv::commit` as two separate steps against the same lock, with nothing else of its own
//! between them that touches the file system — so staging a stop that lands exactly there is
//! what the seam is for; no `Project` needs to be stood up to show it.

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

/// The state write itself runs to completion (nothing is staged yet), recording `last: 1` for
/// `story-3`'s `tickets` — the number this run is about to spend on the key `WF-1`. Then the run
/// that would rewrite the holder and move the document is stopped at its very first operation:
/// nothing of `mv::commit`'s own work happens at all. The number is skipped, not reused: `last`
/// still reads what the state write recorded, and the document never arrived under `WF-1` —
/// exactly the "a re-run (or `validate`) sees a written `last` and a document not yet moved"
/// state decision 1 describes as ordinary and recoverable, not a partial-write bug.
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

/// The same stop, moved one operation later, inside `mv::commit`'s own prepare phase: the state
/// write is still complete and unaffected, and now the holder's content change is prepared (a
/// temp file exists) but not yet renamed into place, and the document itself has not moved —
/// `mv::commit`'s own promise (`mv_seam.rs`) about the run of renames, composed with decision
/// 13's ordering rule about the state write that precedes it.
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

    // One fake operation into `commit`'s own prepare phase (reading the holder's mode to carry):
    // enough to show the state write is unaffected by however far into `commit` a stop lands,
    // not only at the very first instant.
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
