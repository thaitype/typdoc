//! `SIGINT` and `SIGTERM` sent to the shipped binary while it holds a namespace lock (ticket 4's
//! own "done when"), and the one case exit 4 has never been produced through a command before
//! this: a run that meets a lock file nobody owns.
//!
//! **Holding the lock long enough to signal, with no test-only code in the binary.** `new`'s own
//! validation checks every document the namespace already holds for a ref cycle
//! (`Project::prescan_refs`, reading each one from disk), under the lock, before the document it
//! is creating is written. That is real work every `new` and every `set` already does, not
//! something added for this test; given enough documents it keeps the lock held for a stretch of
//! real, wall-clock time, which is what these tests wait on instead of a sleep or a flag. No
//! command here has a "hold the lock and wait" mode, and none needs one.

#[allow(dead_code, reason = "each test file uses part of the shared helper")]
mod common;

use common::{
    LOCK_APPEARS_WITHIN, Scratch, Spawn, WF_COLLECTION, large_project, lock_path, wait_for_file,
};

/// How many documents the namespace holds before a test sends a signal. Large enough that
/// `Project::prescan_refs`'s read-every-document loop measurably holds the lock on every machine
/// this suite has been run on (a single real run at this size took low seconds on this one); the
/// test does not wait that long in practice, because it sends the signal as soon as the lock
/// file appears, which is before that loop even starts.
const DOCUMENT_COUNT: u32 = 20_000;

// ---- done when (a): a real SIGINT and a real SIGTERM, sent while the lock is held ----

#[test]
fn sigint_sent_while_the_lock_is_held_removes_it_and_ends_the_process_by_the_signal() {
    let project = large_project(DOCUMENT_COUNT);
    let lock = lock_path(project.path());
    let running = Spawn::args(["new", "WF", "Interrupted", "--set", "kind=task", "--json"])
        .cwd(project.path())
        .spawn();

    assert!(
        wait_for_file(&lock, LOCK_APPEARS_WITHIN),
        "the lock file never appeared"
    );
    running.signal(libc::SIGINT);
    let ended = running.wait();

    assert_eq!(
        ended.code, None,
        "a process typdoc interrupted must end by the signal, not by an exit code of its own \
         (stdout: {:?}, stderr: {:?})",
        ended.stdout, ended.stderr
    );
    assert_eq!(ended.signal, Some(libc::SIGINT));
    assert!(!lock.exists(), "the lock file was not removed");
    assert!(
        !project.path().join("tickets/WF-20001.md").is_file(),
        "an interrupted new must not leave the document it was about to create"
    );
}

#[test]
fn sigterm_sent_while_the_lock_is_held_removes_it_and_ends_the_process_by_the_signal() {
    let project = large_project(DOCUMENT_COUNT);
    let lock = lock_path(project.path());
    let running = Spawn::args(["new", "WF", "Interrupted", "--set", "kind=task", "--json"])
        .cwd(project.path())
        .spawn();

    assert!(
        wait_for_file(&lock, LOCK_APPEARS_WITHIN),
        "the lock file never appeared"
    );
    running.signal(libc::SIGTERM);
    let ended = running.wait();

    assert_eq!(
        ended.code, None,
        "a process typdoc interrupted must end by the signal, not by an exit code of its own \
         (stdout: {:?}, stderr: {:?})",
        ended.stdout, ended.stderr
    );
    assert_eq!(ended.signal, Some(libc::SIGTERM));
    assert!(!lock.exists(), "the lock file was not removed");
}

// ---- done when (b): a second signal during the cleanup does not cut it short ----

#[test]
fn a_second_signal_arriving_right_behind_the_first_does_not_cut_the_cleanup_short() {
    let project = large_project(DOCUMENT_COUNT);
    let lock = lock_path(project.path());
    let running = Spawn::args(["new", "WF", "Interrupted", "--set", "kind=task", "--json"])
        .cwd(project.path())
        .spawn();

    assert!(
        wait_for_file(&lock, LOCK_APPEARS_WITHIN),
        "the lock file never appeared"
    );
    // Sent back to back, with nothing in between: if a second signal could cut the cleanup
    // short, this is the pair of calls that would show it, by the lock file surviving the run.
    running.signal(libc::SIGINT);
    running.signal(libc::SIGTERM);
    let ended = running.wait();

    assert_eq!(
        ended.code, None,
        "the process must still end by a signal, not by an exit code of its own (stdout: \
         {:?}, stderr: {:?})",
        ended.stdout, ended.stderr
    );
    assert!(
        ended.signal == Some(libc::SIGINT) || ended.signal == Some(libc::SIGTERM),
        "expected the process to end by whichever of the two signals it finished cleaning up \
         for, got {:?}",
        ended.signal
    );
    assert!(
        !lock.exists(),
        "the cleanup must still have run to completion: a second signal arriving during it \
         must not leave the lock file behind"
    );
}

// ---- done when (c): a run meeting a lock file nobody owns stops at exit 4 with the file
// named (closes UNPRODUCED_EXIT_CODES's `4` entry) ----

#[test]
fn a_run_that_meets_a_lock_file_nobody_owns_stops_at_exit_4_with_the_file_named() {
    let project = Scratch::project(&WF_COLLECTION);
    let lock = lock_path(project.path());
    project.file(
        ".typdoc/locks/default.lock",
        r#"{"pid":999999999,"host":"nobody-here","timestamp":"1990-01-01T00:00:00+00:00"}"#,
    );

    let ran = Spawn::args([
        "new",
        "WF",
        "Blocked",
        "--set",
        "kind=task",
        "--json",
        "--lock-timeout",
        "1",
    ])
    .cwd(project.path())
    .run();

    assert_eq!(
        ran.code, 4,
        "stdout: {}, stderr: {}",
        ran.stdout, ran.stderr
    );
    let error = ran.stderr_json();
    assert_eq!(error["code"], serde_json::json!(4));
    let message = error["error"].as_str().expect("an error message");
    assert!(
        message.contains(&lock.display().to_string()),
        "the message must name the lock file: {message}"
    );
    // The lock file this run never made is still exactly the one nobody here owns: a run that
    // times out must never touch it.
    assert!(
        lock.is_file(),
        "a lock this process did not create must never be removed"
    );
}
