//! Covers SPC-3, SPC-10.
//!
//! Under the lock, before it writes, `new` reads every document already there from disk for its
//! ref checks (`Project::prescan_refs`). With enough documents that holds the lock for a real
//! stretch of time, which these tests wait on instead of a sleep: the binary has no mode that
//! holds a lock and waits, and needs none.

#[allow(dead_code, reason = "each test file uses part of the shared helper")]
mod common;

use common::{
    LOCK_APPEARS_WITHIN, Scratch, Spawn, WF_COLLECTION, large_project, lock_path, wait_for_file,
};

/// Large enough that the scan holds the lock for seconds. Each test signals as soon as the lock
/// file appears, before the scan starts.
const DOCUMENT_COUNT: u32 = 20_000;

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

/// The test that makes the binary exit 4, which is why `registry::UNPRODUCED_EXIT_CODES` does not
/// list it.
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
    assert!(
        lock.is_file(),
        "a lock this process did not create must never be removed"
    );
}
