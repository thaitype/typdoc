//! Covers SPC-10.
//!
//! The parts of the lock that need a fake file system. The lock order and the project hash are
//! tested beside the code, in `src/namespace_lock.rs`.

use std::path::{Path, PathBuf};
use std::time::Duration;

use typdoc_core::{Clock, ErrorKind, Fs, Released, acquire, release, release_all_for_signal};
use typdoc_testkit::fake::{FakeFs, FixedClock};

const HOST: &str = "test-host";
/// Longer than any single test needs to actually wait, but short enough that a suite of these
/// tests stays fast; the scenarios that must time out use much shorter values still.
const AMPLE: Duration = Duration::from_secs(5);

fn foreign_lock(fake: &FakeFs, path: &Path, pid: u32, host: &str, timestamp: &str) {
    let bytes =
        format!(r#"{{"pid":{pid},"host":"{host}","timestamp":"{timestamp}"}}"#).into_bytes();
    fake.put(path, &bytes, 0o100_644);
}

#[test]
fn acquiring_an_uncontended_lock_stamps_pid_host_and_the_clock_s_time() {
    let fake = FakeFs::new();
    let clock = FixedClock::new();
    let path = PathBuf::from("/project/.typdoc/locks/default.lock");

    let lock = acquire(&fake, &clock, path.clone(), HOST, AMPLE).expect("nothing holds it");

    assert_eq!(lock.path(), path);
    let bytes = fake.bytes(&path).expect("the lock file was created");
    let stamped: serde_json::Value = serde_json::from_slice(&bytes).expect("valid JSON");
    assert_eq!(stamped["pid"], serde_json::json!(std::process::id()));
    assert_eq!(stamped["host"], serde_json::json!(HOST));
    assert_eq!(
        stamped["timestamp"],
        serde_json::json!(clock.now().to_rfc3339())
    );
}

#[test]
fn releasing_a_lock_that_is_still_ours_removes_it() {
    let fake = FakeFs::new();
    let clock = FixedClock::new();
    let path = PathBuf::from("/project/.typdoc/locks/default.lock");
    let lock = acquire(&fake, &clock, path.clone(), HOST, AMPLE).expect("nothing holds it");

    let outcome = release(lock).expect("the release itself does not fail");

    assert_eq!(outcome, Released::ByUs);
    assert_eq!(fake.bytes(&path), None, "the lock file was not removed");
}

#[test]
fn a_lock_dropped_without_calling_release_is_released_anyway() {
    let fake = FakeFs::new();
    let clock = FixedClock::new();
    let path = PathBuf::from("/project/.typdoc/locks/default.lock");
    {
        let _lock = acquire(&fake, &clock, path.clone(), HOST, AMPLE).expect("nothing holds it");
    }

    assert_eq!(
        fake.bytes(&path),
        None,
        "the lock outlived the scope that acquired it"
    );
}

#[test]
fn acquire_times_out_against_a_lock_it_did_not_create_and_never_touches_it_however_young() {
    let fake = FakeFs::new();
    let clock = FixedClock::new();
    let path = PathBuf::from("/project/.typdoc/locks/default.lock");
    foreign_lock(&fake, &path, 4242, HOST, &clock.now().to_rfc3339());
    let before = fake
        .bytes(&path)
        .expect("the fixture wrote the foreign lock");

    let err = acquire(&fake, &clock, path.clone(), HOST, Duration::from_millis(30))
        .expect_err("the path is already a lock file");

    assert_eq!(err.kind(), ErrorKind::LockTimeout);
    assert_eq!(
        fake.bytes(&path),
        Some(before),
        "a lock this process did not create must never be touched, freshly made or not"
    );
}

#[test]
fn acquire_times_out_against_a_lock_it_did_not_create_and_never_touches_it_however_old() {
    let fake = FakeFs::new();
    let clock = FixedClock::new();
    let path = PathBuf::from("/project/.typdoc/locks/default.lock");
    let ancient = "1990-01-01T00:00:00+00:00";
    foreign_lock(&fake, &path, 4242, HOST, ancient);
    let before = fake
        .bytes(&path)
        .expect("the fixture wrote the foreign lock");

    let err = acquire(&fake, &clock, path.clone(), HOST, Duration::from_millis(30))
        .expect_err("the path is already a lock file, decades old or not");

    assert_eq!(err.kind(), ErrorKind::LockTimeout);
    assert_eq!(
        fake.bytes(&path),
        Some(before),
        "age never earns a takeover: the design has no threshold at all"
    );
}

#[test]
fn a_lock_taken_away_and_replaced_is_reported_and_the_replacement_is_left_alone() {
    let fake = FakeFs::new();
    let clock = FixedClock::new();
    let path = PathBuf::from("/project/.typdoc/locks/default.lock");
    let lock = acquire(&fake, &clock, path.clone(), HOST, AMPLE).expect("nothing holds it");

    // Another process replaces the lock file with its own.
    fake.remove_file(&path).expect("the path existed");
    foreign_lock(
        &fake,
        &path,
        9999,
        "another-host",
        &clock.now().to_rfc3339(),
    );
    let replacement = fake.bytes(&path).expect("the replacement is there");

    let outcome = release(lock).expect("the release call itself does not fail");

    assert_eq!(outcome, Released::TakenByAnother);
    assert_eq!(
        fake.bytes(&path),
        Some(replacement),
        "the file that replaced ours must be left exactly as it was"
    );
}

#[test]
fn release_all_for_signal_removes_every_lock_this_process_still_holds() {
    let fake = FakeFs::new();
    let clock = FixedClock::new();
    let one = PathBuf::from("/project/.typdoc/locks/a.lock");
    let two = PathBuf::from("/project/.typdoc/locks/b.lock");
    let lock_one = acquire(&fake, &clock, one.clone(), HOST, AMPLE).expect("nothing holds it");
    let lock_two = acquire(&fake, &clock, two.clone(), HOST, AMPLE).expect("nothing holds it");

    release_all_for_signal(&fake);

    assert_eq!(fake.bytes(&one), None, "the first lock was not removed");
    assert_eq!(fake.bytes(&two), None, "the second lock was not removed");
    // Alive until here, so no `Drop` runs before the assertions; this second release is a no-op.
    drop(lock_one);
    drop(lock_two);
}

#[test]
fn release_all_for_signal_never_touches_a_lock_taken_away_and_replaced() {
    let fake = FakeFs::new();
    let clock = FixedClock::new();
    let path = PathBuf::from("/project/.typdoc/locks/default.lock");
    let lock = acquire(&fake, &clock, path.clone(), HOST, AMPLE).expect("nothing holds it");
    // Another process replaces the lock file with its own.
    fake.remove_file(&path).expect("the path existed");
    foreign_lock(
        &fake,
        &path,
        9999,
        "another-host",
        &clock.now().to_rfc3339(),
    );
    let replacement = fake.bytes(&path).expect("the replacement is there");

    release_all_for_signal(&fake);

    assert_eq!(
        fake.bytes(&path),
        Some(replacement),
        "a lock taken away and replaced must be left exactly as it was, even through the \
         signal path"
    );
    drop(lock);
}

// The two tests below cannot tell a removed registry entry from a leaked one: the fake never
// repeats an identity, so a leaked entry is refused anyway.
// `namespace_lock::tests::dropping_a_namespace_lock_deregisters_it_even_when_release_is_never_called`
// proves the removal.

#[test]
fn a_lock_released_normally_leaves_a_later_file_at_the_same_path_untouched() {
    let fake = FakeFs::new();
    let clock = FixedClock::new();
    let path = PathBuf::from("/project/.typdoc/locks/default.lock");
    let lock = acquire(&fake, &clock, path.clone(), HOST, AMPLE).expect("nothing holds it");
    release(lock).expect("the release itself does not fail");
    foreign_lock(&fake, &path, 4242, HOST, &clock.now().to_rfc3339());
    let unrelated = fake.bytes(&path).expect("the foreign lock is there");

    release_all_for_signal(&fake);

    assert_eq!(
        fake.bytes(&path),
        Some(unrelated),
        "a lock already released through the ordinary path must leave nothing for the signal \
         path to remove"
    );
}

#[test]
fn a_lock_dropped_without_releasing_leaves_a_later_file_at_the_same_path_untouched() {
    let fake = FakeFs::new();
    let clock = FixedClock::new();
    let path = PathBuf::from("/project/.typdoc/locks/default.lock");
    {
        let _lock = acquire(&fake, &clock, path.clone(), HOST, AMPLE).expect("nothing holds it");
    }
    foreign_lock(&fake, &path, 4242, HOST, &clock.now().to_rfc3339());
    let unrelated = fake.bytes(&path).expect("the foreign lock is there");

    release_all_for_signal(&fake);

    assert_eq!(
        fake.bytes(&path),
        Some(unrelated),
        "a lock already dropped without releasing must leave nothing for the signal path to \
         remove"
    );
}

// The timeout message is tested in `crates/typdoc-fs/tests/namespace_lock_timeout.rs`: the
// owner is read with `std::fs::read`, which cannot see a lock file the fake holds in memory.

#[test]
fn an_uncontended_lock_is_acquired_well_before_the_timeout() {
    let fake = FakeFs::new();
    let clock = FixedClock::new();
    let path = PathBuf::from("/project/.typdoc/locks/default.lock");
    let start = std::time::Instant::now();

    acquire(&fake, &clock, path, HOST, Duration::from_secs(3600)).expect("nothing holds it");

    assert!(
        start.elapsed() < Duration::from_secs(1),
        "an uncontended acquire waited as though something held the lock"
    );
}

#[test]
fn a_lock_released_partway_through_the_wait_is_acquired_before_the_timeout() {
    use std::thread;

    let fake = FakeFs::new();
    let clock = FixedClock::new();
    let path = PathBuf::from("/project/.typdoc/locks/default.lock");
    foreign_lock(&fake, &path, 4242, HOST, &clock.now().to_rfc3339());

    let releasing_fake = fake.clone();
    let releasing_path = path.clone();
    let releaser = thread::spawn(move || {
        thread::sleep(Duration::from_millis(60));
        releasing_fake
            .remove_file(&releasing_path)
            .expect("the foreign lock is there to remove");
    });

    let outcome = acquire(&fake, &clock, path, HOST, Duration::from_secs(2));
    releaser
        .join()
        .expect("the releasing thread does not panic");

    assert!(
        outcome.is_ok(),
        "a lock freed before the timeout should be acquired, not timed out"
    );
}
