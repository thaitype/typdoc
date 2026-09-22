//! `mv::commit`'s two-phase promise (decision 1): every content change is prepared first, then
//! the renames happen in one run, the document itself last of all. A stop partway through the
//! renames leaves every rename not yet reached exactly as it was — proved deterministically with
//! a fake staged to stop after a fixed number of operations, the same way `crates/typdoc-fs/
//! tests/write_seam.rs` already proves `write_atomically`'s own promise; no real project is
//! needed for this, since `commit` reads no index and no config, only the paths and bytes it is
//! given.

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

/// `holder-a.md` and `holder-b.md` hold refs to `old.md`; `holder-a.md`'s content change and the
/// document's own move are the two renames this table cares about telling apart.
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

/// Each content change's prepare is four fake operations (read the mode to carry, create the
/// temp, set its mode, write its bytes), so preparing both holders is eight; stopped right after
/// the ninth (the first content rename, `holder-a.md`'s) but before the tenth (`holder-b.md`'s)
/// or the eleventh (the document's own move): `holder-a.md` already reads the new form,
/// `holder-b.md` and the document itself are exactly as they were before the run — decision 1's
/// own description of the window, produced on purpose rather than described.
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

/// Stopped partway through preparing the first content change (before its bytes are even
/// written): neither holder nor the document is touched, and the run's own cleanup removes the
/// one temp file it had started — the "nothing renamed yet" state a stop during prepare always
/// leaves, decision 4's leftover when that cleanup itself cannot run.
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

/// The same table, run to completion, with the document's own final rename staged to fail: every
/// content change is committed, and only the move is left undone — the exact state a re-run of
/// the same `mv` command finds and finishes (rediscovering nothing to rewrite, since every ref
/// now names the new path, and only the document itself still needs to move).
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
