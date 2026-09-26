//! `namespace_lock::acquire` reads the competing lock file with `std::fs::read`, the same way
//! every other read in `typdoc-core` reaches a file: not through `Fs`, which is the write seam
//! only. A fake file system therefore cannot back this: it holds files in memory, and
//! `std::fs::read` never sees them. This file uses a real temporary directory instead, which is
//! also why it lives beside `write_seam.rs` rather than in `typdoc-core`'s own `tests/`. The
//! same reasoning is why the directory-creation test below is here too: a fake never refuses a
//! `create_new` under a directory it never made, so only a real one shows the gap.

use std::path::PathBuf;
use std::time::Duration;

use tempfile::TempDir;
use typdoc_core::{Clock, ErrorKind, acquire};
use typdoc_testkit::fake::FixedClock;

const HOST: &str = "test-host";

fn foreign_lock(dir: &TempDir, name: &str, pid: u32, host: &str, timestamp: &str) -> PathBuf {
    let path = dir.path().join(name);
    let bytes = format!(r#"{{"pid":{pid},"host":"{host}","timestamp":"{timestamp}"}}"#);
    std::fs::write(&path, bytes).expect("the fixture's own write, outside the seam, succeeds");
    path
}

#[test]
fn a_timeout_names_the_path_pid_host_and_age_and_maps_to_exit_4() {
    let dir = TempDir::new().expect("a temporary directory can be made");
    let clock = FixedClock::new();
    // Five hours before the fixed clock's own instant (2001-02-03T04:05:06+07:00, `FIXED_INSTANT`),
    // written by hand so this test needs no arithmetic on the clock's reading.
    let path = foreign_lock(
        &dir,
        "default.lock",
        4242,
        HOST,
        "2001-02-02T23:05:06+07:00",
    );

    let err = acquire(
        &typdoc_fs::SystemFs,
        &clock,
        path.clone(),
        HOST,
        Duration::from_millis(30),
    )
    .expect_err("the path is already a lock file");

    assert_eq!(err.kind(), ErrorKind::LockTimeout);
    let message = err.to_string();
    assert!(message.contains(&path.display().to_string()), "{message}");
    assert!(message.contains("4242"), "{message}");
    assert!(message.contains(HOST), "{message}");
    assert!(message.contains("5h0m0s"), "{message}");
}

#[test]
fn a_timeout_says_the_owner_is_running_here_when_the_host_matches_and_the_pid_is_alive() {
    let dir = TempDir::new().expect("a temporary directory can be made");
    let clock = FixedClock::new();
    let path = foreign_lock(
        &dir,
        "default.lock",
        std::process::id(),
        HOST,
        &clock.now().to_rfc3339(),
    );

    let err = acquire(
        &typdoc_fs::SystemFs,
        &clock,
        path,
        HOST,
        Duration::from_millis(30),
    )
    .expect_err("the path is already a lock file");

    assert!(err.to_string().contains("running on this machine"), "{err}");
}

#[test]
fn a_timeout_says_the_lock_is_stale_when_the_host_matches_and_the_pid_is_not_alive() {
    let dir = TempDir::new().expect("a temporary directory can be made");
    let clock = FixedClock::new();
    // No real process has this pid: `pid_max` on Linux never reaches anywhere near it.
    let path = foreign_lock(
        &dir,
        "default.lock",
        u32::MAX,
        HOST,
        &clock.now().to_rfc3339(),
    );

    let err = acquire(
        &typdoc_fs::SystemFs,
        &clock,
        path,
        HOST,
        Duration::from_millis(30),
    )
    .expect_err("the path is already a lock file");

    assert!(
        err.to_string().contains("not running on this machine"),
        "{err}"
    );
}

#[test]
fn a_timeout_says_it_cannot_be_checked_when_the_recorded_host_differs() {
    let dir = TempDir::new().expect("a temporary directory can be made");
    let clock = FixedClock::new();
    let path = foreign_lock(
        &dir,
        "default.lock",
        4242,
        "some-other-machine",
        &clock.now().to_rfc3339(),
    );

    let err = acquire(
        &typdoc_fs::SystemFs,
        &clock,
        path,
        HOST,
        Duration::from_millis(30),
    )
    .expect_err("the path is already a lock file");

    assert!(err.to_string().contains("cannot be checked"), "{err}");
}

#[test]
fn acquire_creates_the_directory_that_will_hold_the_lock_file() {
    let dir = TempDir::new().expect("a temporary directory can be made");
    let clock = FixedClock::new();
    let path = dir.path().join(".typdoc/locks/default.lock");
    assert!(
        !path.parent().unwrap().exists(),
        "the fixture must start without the directory, or this test proves nothing"
    );

    let lock = acquire(
        &typdoc_fs::SystemFs,
        &clock,
        path.clone(),
        HOST,
        Duration::from_secs(1),
    )
    .expect("acquire makes the directory itself rather than requiring the caller to");

    assert!(path.exists(), "the lock file itself was not created");
    typdoc_core::release(lock).expect("releasing succeeds");
}
