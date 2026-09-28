//! Symbolic links for tests, on Unix and on Windows. Windows makes a link to a file and a link to a
//! folder with two different calls, so the caller says which one it wants.

use std::io;
use std::path::Path;

/// A link at `at` to the file `target`, which may be relative to the link's folder.
pub fn file(target: impl AsRef<Path>, at: impl AsRef<Path>) -> io::Result<()> {
    #[cfg(unix)]
    return std::os::unix::fs::symlink(target, at);
    #[cfg(windows)]
    return std::os::windows::fs::symlink_file(target, at);
}

/// A link at `at` to the folder `target`, which may be relative to the link's folder.
pub fn dir(target: impl AsRef<Path>, at: impl AsRef<Path>) -> io::Result<()> {
    #[cfg(unix)]
    return std::os::unix::fs::symlink(target, at);
    #[cfg(windows)]
    return std::os::windows::fs::symlink_dir(target, at);
}

/// Recreates the link `from` at `to`, pointing where `from` points, as a folder link when what it
/// leads to is a folder.
pub fn recreate(from: &Path, to: &Path) -> io::Result<()> {
    let target = std::fs::read_link(from)?;
    if std::fs::metadata(from).is_ok_and(|m| m.is_dir()) {
        dir(target, to)
    } else {
        file(target, to)
    }
}
