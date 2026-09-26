//! Covers SPC-2.
//!
//! Where `mv::commit` can stop, and what each stop leaves, with a fake staged to stop after a
//! fixed number of operations.

use std::path::{Path, PathBuf};
use std::time::Duration;

use typdoc_core::{ContentChange, NamespaceLock, acquire, commit};
use typdoc_testkit::fake::{FakeFs, FixedClock, Stage};

fn a_lock(fake: &FakeFs) -> NamespaceLock<'_> {
    acquire(
        fake,
        &FixedClock::new(),
        PathBuf::from("/project/.typdoc/locks/default.lock"),
        "mv-seam-test-host",
        Duration::from_secs(5),
    )
    .expect("nothing holds it yet")
}

fn seed(fake: &FakeFs) {
    fake.put(
        Path::new("/project/old.md"),
        b"---\ntitle: old\n---\n",
        0o100_644,
    );
    fake.put(
        Path::new("/project/holder-a.md"),
        b"see: old.md\n",
        0o100_644,
    );
    fake.put(
        Path::new("/project/holder-b.md"),
        b"see: old.md\n",
        0o100_644,
    );
}

fn changes() -> Vec<ContentChange> {
    vec![
        ContentChange {
            path: PathBuf::from("/project/holder-a.md"),
            bytes: b"see: new.md\n".to_vec(),
        },
        ContentChange {
            path: PathBuf::from("/project/holder-b.md"),
            bytes: b"see: new.md\n".to_vec(),
        },
    ]
}

#[test]
fn a_full_run_prepares_every_change_and_renames_the_document_last() {
    let fake = FakeFs::new();
    seed(&fake);
    let lock = a_lock(&fake);

    commit(
        &fake,
        &lock,
        &changes(),
        Path::new("/project/old.md"),
        Path::new("/project/new.md"),
    )
    .expect("nothing staged to fail");

    assert_eq!(
        fake.bytes(Path::new("/project/holder-a.md")).as_deref(),
        Some(&b"see: new.md\n"[..])
    );
    assert_eq!(
        fake.bytes(Path::new("/project/holder-b.md")).as_deref(),
        Some(&b"see: new.md\n"[..])
    );
    assert_eq!(
        fake.bytes(Path::new("/project/old.md")),
        None,
        "the source is gone"
    );
    assert_eq!(
        fake.bytes(Path::new("/project/new.md")).as_deref(),
        Some(&b"---\ntitle: old\n---\n"[..]),
        "the document arrived under its new name with its own bytes untouched"
    );
}

/// A prepare is four operations (read the mode, create the temp file, set its mode, write), so
/// both holders take eight and the ninth is `holder-a.md`'s rename.
#[test]
fn a_stop_between_two_renames_leaves_the_earlier_one_done_and_the_rest_untouched() {
    let fake = FakeFs::new();
    seed(&fake);
    let lock = a_lock(&fake);
    fake.arm(Stage::StopAfter(9));

    let err = commit(
        &fake,
        &lock,
        &changes(),
        Path::new("/project/old.md"),
        Path::new("/project/new.md"),
    )
    .expect_err("staged to stop");
    assert_eq!(err.to_string(), "the run stopped");

    assert_eq!(
        fake.bytes(Path::new("/project/holder-a.md")).as_deref(),
        Some(&b"see: new.md\n"[..]),
        "the first rename went ahead"
    );
    assert_eq!(
        fake.bytes(Path::new("/project/holder-b.md")).as_deref(),
        Some(&b"see: old.md\n"[..]),
        "the second content change was prepared but never renamed into place"
    );
    assert_eq!(
        fake.bytes(Path::new("/project/old.md")).as_deref(),
        Some(&b"---\ntitle: old\n---\n"[..]),
        "the document itself moves last, so a stop before that rename leaves it exactly where it was"
    );
    assert_eq!(
        fake.bytes(Path::new("/project/new.md")),
        None,
        "nothing exists yet under the new name"
    );
}

/// Stops after three operations, inside the first prepare, before its bytes are written.
#[test]
fn a_stop_during_prepare_renames_nothing_at_all() {
    let fake = FakeFs::new();
    seed(&fake);
    let lock = a_lock(&fake);
    fake.arm(Stage::StopAfter(3));

    let err = commit(
        &fake,
        &lock,
        &changes(),
        Path::new("/project/old.md"),
        Path::new("/project/new.md"),
    )
    .expect_err("staged to stop");
    assert_eq!(err.to_string(), "the run stopped");

    assert_eq!(
        fake.bytes(Path::new("/project/holder-a.md")).as_deref(),
        Some(&b"see: old.md\n"[..])
    );
    assert_eq!(
        fake.bytes(Path::new("/project/holder-b.md")).as_deref(),
        Some(&b"see: old.md\n"[..])
    );
    assert_eq!(
        fake.bytes(Path::new("/project/old.md")).as_deref(),
        Some(&b"---\ntitle: old\n---\n"[..])
    );
}

/// The state a re-run of the same `mv` finishes: every ref already names the new path, and only
/// the document is left to move.
#[test]
fn a_stop_at_the_documents_own_move_leaves_every_content_change_already_committed() {
    let fake = FakeFs::new();
    seed(&fake);
    let lock = a_lock(&fake);
    fake.arm(Stage::StopAfter(10));

    let err = commit(
        &fake,
        &lock,
        &changes(),
        Path::new("/project/old.md"),
        Path::new("/project/new.md"),
    )
    .expect_err("staged to stop at the last rename");
    assert_eq!(err.to_string(), "the run stopped");

    assert_eq!(
        fake.bytes(Path::new("/project/holder-a.md")).as_deref(),
        Some(&b"see: new.md\n"[..])
    );
    assert_eq!(
        fake.bytes(Path::new("/project/holder-b.md")).as_deref(),
        Some(&b"see: new.md\n"[..])
    );
    assert_eq!(
        fake.bytes(Path::new("/project/old.md")).as_deref(),
        Some(&b"---\ntitle: old\n---\n"[..]),
        "still where it was: the document moves last"
    );
}
