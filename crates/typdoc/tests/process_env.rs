//! Covers SPC-10: what the shipped environment answers about this machine. These ask the real
//! system, so they run on every Unix the suite runs on, macOS included.
#![cfg(unix)]

#[allow(dead_code, reason = "each test file uses part of the shared helper")]
mod common;

use common::Spawn;
use typdoc::process_env::ProcessEnv;
use typdoc_core::{Env, ProcessStatus};

#[test]
fn this_process_is_running() {
    assert_eq!(
        ProcessEnv.process_status(std::process::id()),
        ProcessStatus::Running
    );
}

#[test]
fn a_process_of_another_user_is_running() {
    // SAFETY: `geteuid` takes nothing and cannot fail.
    if unsafe { libc::geteuid() } == 0 {
        eprintln!("skipped: as root, every process may be signalled, so no EPERM is met");
        return;
    }
    // Process 1 belongs to root on every Unix, so asking about it as anyone else is refused
    // with `EPERM`.
    assert_eq!(ProcessEnv.process_status(1), ProcessStatus::Running);
}

#[test]
fn a_process_that_has_ended_and_been_reaped_is_not_running() {
    let child = Spawn::args(["--version"]).spawn();
    let pid = child.pid();
    child.wait();

    assert_eq!(ProcessEnv.process_status(pid), ProcessStatus::NotRunning);
}

#[test]
fn an_id_no_process_can_have_is_unknown_rather_than_running() {
    // As a `pid_t`, `0` names this process's own group and `u32::MAX` becomes `-1`, every
    // process: asked as they are, both would answer "running".
    for pid in [0, u32::MAX, i32::MAX as u32 + 1] {
        assert_eq!(
            ProcessEnv.process_status(pid),
            ProcessStatus::Unknown,
            "pid {pid}"
        );
    }
}

#[test]
fn the_hostname_is_read_from_the_system() {
    let name = ProcessEnv.hostname();

    assert!(!name.is_empty());
    assert_ne!(name, "unknown-host");
}
