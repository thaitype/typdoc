//! Covers SPC-3, SPC-10: the Windows form of `signals.rs`.
//!
//! Windows only. A console event there reaches typdoc as Ctrl+C, Ctrl+Break or the console
//! closing, and ends it with `STATUS_CONTROL_C_EXIT` after the locks it holds are removed.
//!
//! Under the lock, before it writes, `new` reads every document already there from disk for its
//! ref checks (`Project::prescan_refs`), which holds the lock long enough to interrupt.

#![cfg(windows)]

#[allow(dead_code, reason = "each test file uses part of the shared helper")]
mod common;

use common::{LOCK_APPEARS_WITHIN, Spawn, large_project, lock_path, wait_for_file};

/// As in `signals.rs`.
const DOCUMENT_COUNT: u32 = 20_000;

/// `STATUS_CONTROL_C_EXIT` as `ExitStatus::code` reports it.
const CONTROL_C_EXIT: i32 = windows_sys::Win32::Foundation::STATUS_CONTROL_C_EXIT;

#[test]
fn ctrl_break_sent_while_the_lock_is_held_removes_it_and_ends_with_the_ctrl_c_exit_code() {
    let project = large_project(DOCUMENT_COUNT);
    let lock = lock_path(project.path());
    let running = Spawn::args(["new", "WF", "Interrupted", "--set", "kind=task", "--json"])
        .cwd(project.path())
        .spawn();

    assert!(
        wait_for_file(&lock, LOCK_APPEARS_WITHIN),
        "the lock file never appeared"
    );
    running.interrupt().expect("Ctrl+Break is sent");
    let ended = running.wait();

    assert_eq!(
        ended.code,
        Some(CONTROL_C_EXIT),
        "an interrupted typdoc ends with STATUS_CONTROL_C_EXIT (stdout: {:?}, stderr: {:?})",
        ended.stdout,
        ended.stderr
    );
    assert!(!lock.exists(), "the lock file was not removed");
    assert!(
        !project.path().join("tickets/WF-20001.md").is_file(),
        "an interrupted new must not leave the document it was about to create"
    );
}

#[test]
fn a_second_event_arriving_right_behind_the_first_does_not_cut_the_cleanup_short() {
    let project = large_project(DOCUMENT_COUNT);
    let lock = lock_path(project.path());
    let running = Spawn::args(["new", "WF", "Interrupted", "--set", "kind=task", "--json"])
        .cwd(project.path())
        .spawn();

    assert!(
        wait_for_file(&lock, LOCK_APPEARS_WITHIN),
        "the lock file never appeared"
    );
    running.interrupt().expect("Ctrl+Break is sent");
    // Back to back: the first may already have ended the process, so the second may not be sent.
    let _ = running.interrupt();
    let ended = running.wait();

    assert_eq!(ended.code, Some(CONTROL_C_EXIT));
    assert!(
        !lock.exists(),
        "the cleanup must still have run to completion: a second event arriving during it must \
         not leave the lock file behind"
    );
}
