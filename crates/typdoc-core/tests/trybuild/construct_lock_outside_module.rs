// FAIL: `NamespaceLock` has no public constructor and no public field, so this must not compile
// however it is spelled. `acquire` is the only function that returns one (decision 6).

fn main() {
    let _lock = typdoc_core::NamespaceLock {
        path: std::path::PathBuf::new(),
    };
}
