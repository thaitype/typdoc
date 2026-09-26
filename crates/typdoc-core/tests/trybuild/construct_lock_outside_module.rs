// FAIL: `NamespaceLock` has no public constructor and no public field, so this must not compile
// however it is spelled (SPC-10).

fn main() {
    let _lock = typdoc_core::NamespaceLock {
        path: std::path::PathBuf::new(),
    };
}
