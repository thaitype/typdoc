//! A folder of files for a test that builds its project in place, since `typdoc-core`'s own tests
//! may not write a file (its lint list refuses the calls).

use tempfile::TempDir;

/// A new temporary folder holding each `(path, text)` of `files`, folders made as needed.
pub fn tree(files: &[(&str, &str)]) -> TempDir {
    let dir = tempfile::tempdir().expect("a temporary folder");
    for (path, text) in files {
        let file = dir.path().join(path);
        if let Some(parent) = file.parent() {
            std::fs::create_dir_all(parent).expect("the file's folder");
        }
        std::fs::write(&file, text).expect("the file");
    }
    dir
}
