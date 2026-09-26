// PASS: the compiling twin of `write_without_lock.rs`.

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
    typdoc_core::write_atomically(&fs, &lock, std::path::Path::new("/project/note.md"), b"x")
        .expect("a fresh fake refuses nothing about this write");
}
