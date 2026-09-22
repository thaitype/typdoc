// FAIL: `write_atomically` takes a `&NamespaceLock` (decision 6, "every function that writes
// takes it by reference"); calling it without one must not compile.

use typdoc_testkit::fake::FakeFs;

fn main() {
    let fs = FakeFs::new();
    let _ = typdoc_core::write_atomically(&fs, std::path::Path::new("/project/note.md"), b"x");
}
