//! What a command reaches the outside world through: the write seam, and a clock whose
//! instant a test chose rather than the machine.

use std::ffi::OsString;
use std::io;
use std::path::PathBuf;
use std::time::Duration;

use typdoc_core::{Clock, Deps, Env, acquire, at_one_second, write_atomically};
use typdoc_testkit::fake::{FIXED_INSTANT, FakeFs, FixedClock};

struct NoEnv;

impl Env for NoEnv {
    fn var(&self, _name: &str) -> Option<OsString> {
        None
    }

    fn current_dir(&self) -> io::Result<PathBuf> {
        Ok(PathBuf::from("/project"))
    }
}

/// The instant a command gets from `deps`, written out here rather than read from the fake,
/// so that changing the fake's instant is a change a test reports.
const CHOSEN: &str = "2001-02-03T04:05:06+07:00";

#[test]
fn the_clock_in_deps_gives_the_instant_a_test_chose_and_not_the_machine_s() {
    let fake = FakeFs::new();
    let clock = FixedClock::new();
    let deps = Deps {
        env: &NoEnv,
        fs: &fake,
        clock: &clock,
    };

    assert_eq!(deps.clock.now().to_rfc3339(), CHOSEN);
    assert_eq!(FIXED_INSTANT, CHOSEN);
}

#[test]
fn the_instant_the_injected_clock_gives_is_at_one_second() {
    let instant = FixedClock::new().now();

    assert_eq!(instant.timestamp_subsec_nanos(), 0);
    assert_eq!(at_one_second(instant), instant);
}

#[test]
fn the_injected_clock_carries_an_offset_of_its_own_rather_than_none() {
    let instant = FixedClock::new().now();

    assert_eq!(instant.offset().local_minus_utc(), 7 * 3600);
}

#[test]
fn the_clock_in_deps_does_not_move_between_two_readings() {
    let clock = FixedClock::new();

    assert_eq!(clock.now(), clock.now());
}

#[test]
fn a_write_reaches_a_file_system_only_through_the_seam_in_deps() {
    let fake = FakeFs::new();
    let clock = FixedClock::new();
    let deps = Deps {
        env: &NoEnv,
        fs: &fake,
        clock: &clock,
    };
    let note = PathBuf::from("/project/note.md");
    let lock = acquire(
        deps.fs,
        deps.clock,
        PathBuf::from("/locks/note.lock"),
        "test-host",
        Duration::from_secs(5),
    )
    .expect("the lock can be acquired");

    write_atomically(deps.fs, &lock, &note, b"through the seam").expect("the write succeeds");

    assert_eq!(fake.bytes(&note).as_deref(), Some(&b"through the seam"[..]));
}
