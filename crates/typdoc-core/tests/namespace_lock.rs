//! `acquire`, `release`, and the message a timeout produces (ticket 3's "done when" (b) and
//! (d)): the lock file made with `O_EXCL`, the identity check before removal, and the exit that
//! maps to code 4. Decision 3's ordering and decision 14's project hash are pure functions and
//! are tested beside the code, in `src/namespace_lock.rs`; what is here needs a fake file
//! system and so cannot live there (see the note at the top of that module's own test section).

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

// ---- acquiring and releasing, the happy path ----

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
        // Dropped here, at the end of the block, with `release` never called: decision 6's "a
        // command cannot hold a lock past the scope that acquired it" holds even then.
    }

    assert_eq!(
        fake.bytes(&path),
        None,
        "the lock outlived the scope that acquired it"
    );
}

// ---- done when (d): a lock another process created is never removed, at any age ----

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

    // Someone else removes what this process holds and puts their own file at the same path,
    // entirely outside this module: the fake models exactly what a lock's release has to
    // notice on a real file system, without needing two real processes to do it.
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

// ---- ticket 4: the registry the signal-triggered cleanup thread reads, since the boxed
// handle inside a `NamespaceLock` cannot cross a thread the way `acquire`'s own caller can ----

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
    // Kept alive until here on purpose, so neither value's own `Drop` runs before the
    // assertions above; the drop that follows is a second, harmless no-op release.
    drop(lock_one);
    drop(lock_two);
}

#[test]
fn release_all_for_signal_never_touches_a_lock_taken_away_and_replaced() {
    let fake = FakeFs::new();
    let clock = FixedClock::new();
    let path = PathBuf::from("/project/.typdoc/locks/default.lock");
    let lock = acquire(&fake, &clock, path.clone(), HOST, AMPLE).expect("nothing holds it");
    // Someone else removes what this process holds and puts their own file at the same path,
    // the same substitution `a_lock_taken_away_and_replaced_is_reported...` above models for
    // the ordinary release path; this is the same property proved for the signal path instead.
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

// The two tests below show the public pipeline (`acquire` then `release`, or `acquire` then a
// plain drop) leaves nothing for `release_all_for_signal` to touch afterward, through a foreign
// file at the same path exactly as the identity-mismatch test above uses. They cannot, on their
// own, tell a properly deregistered entry apart from a merely leaked one that happens to be
// harmless: `typdoc_testkit::fake`'s own `fresh_identity` never repeats a value, so a leaked
// entry's cached identity can never coincide with a later file's, and the identity check alone
// already protects the foreign file either way. What actually proves `Drop` calls `deregister`
// is `namespace_lock::tests::dropping_a_namespace_lock_deregisters_it_even_when_release_is_never_called`,
// a unit test beside the code that reads the registry's own length directly, which is not
// reachable from here.

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
        // Dropped here, exactly as `a_lock_dropped_without_calling_release_is_released_anyway`
        // already covers for the ordinary path.
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

// The message a timeout produces when the competing lock's content can actually be read is
// tested in `crates/typdoc-fs/tests/namespace_lock_timeout.rs`, not here: reading it goes
// through `std::fs::read` directly (this crate reads nothing through `Fs`, which is a write
// seam — see `fs.rs`'s own module doc), so a lock file the fake only holds in memory is
// invisible to it and every message-content assertion below would see the fallback text
// instead. The two tests just above this note stay here because they do not depend on the
// message's content, only on the error's kind and on the file being untouched, both of which
// the fake answers for correctly.

// ---- retry-with-backoff, and whether a caller can tell a lock apart from one never
// contended ----

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
