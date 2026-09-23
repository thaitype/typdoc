// PASS: the sanctioned way to get a `NamespaceLock` — `acquire` — compiles and runs. Paired
// with `construct_lock_outside_module.rs`, which must not compile: together they show the
// failure there is the private constructor, not something wrong with the fixture harness.

use typdoc_testkit::fake::{FakeFs, FixedClock};

fn main() {
    let fs = FakeFs::new();
    let clock = FixedClock::new();
    let lock = typdoc_core::acquire(
        &fs,
        &clock,
        std::path::PathBuf::from("/project/.typdoc/locks/default.lock"),
        "host",
        std::time::Duration::from_secs(1),
    )
    .expect("nothing holds this path in a fresh fake");
    typdoc_core::release(lock).expect("releasing a lock this process just acquired succeeds");
}
