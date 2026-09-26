//! The seam every write goes through, and the functions above it that hold the rules for temp
//! files, modes and renames.
//!
//! The seam is the file operations a write is built from rather than "write this document", so
//! those rules sit here rather than below the seam. The `typdoc-fs` crate implements it and is
//! the only crate that may change a file.

use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// A file's permission bits, as the platform's metadata gives them.
pub type Mode = u32;

/// What every temp file's name begins with. A name that begins with it is never a document,
/// whatever any `match` says.
pub const TEMP_PREFIX: &str = ".typdoc-tmp-";

/// Whether a file name has the reserved shape, and so is a temp file rather than a document.
///
/// The writer that makes such a name and the walker that skips one both ask here, because a
/// second spelling of the shape is how a walker comes to miss what a writer makes.
pub fn is_temp_name(name: &str) -> bool {
    name.starts_with(TEMP_PREFIX)
}

/// What the file system reports about a file: enough to tell whether two names lead to one
/// file, and whether a file behind a still-open handle has been unlinked.
///
/// `device` and `inode` are what a lock's release compares between the path and its handle.
/// Only the handle's `links` is read: whether the file behind the handle has been unlinked is
/// not something a `stat` on a path that may now lead elsewhere can tell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileId {
    pub device: u64,
    pub inode: u64,
    pub links: u64,
}

/// A file open for writing, from [`Fs::create_new`]. Dropping it closes the file.
pub trait WriteHandle {
    /// Writes every byte, or fails having written some of them: what is on disk after a
    /// failure is not defined, which is why the file being written is never the file a
    /// reader can see.
    fn write_all(&mut self, bytes: &[u8]) -> io::Result<()>;

    /// Asks the storage to hold what has been written. Whether a write flushes before it
    /// renames is not decided, so the operation is on the seam and the decision stays open.
    fn sync(&mut self) -> io::Result<()>;

    /// The identity of the open file (`fstat`), which nothing done to its path afterwards
    /// changes: a lock's release compares it with the path's.
    fn identity(&self) -> io::Result<FileId>;
}

/// The file operations a write is built from.
///
/// Reading a project does not go through here; three reads do, because a write's own rules
/// rest on them: whether the destination exists, what an existing file's mode is, and whether
/// two paths are one file.
pub trait Fs {
    /// Creates a file that must not already exist, and fails with
    /// [`io::ErrorKind::AlreadyExists`] when it does, so that the file system refuses a write
    /// onto an existing path rather than typdoc remembering to look first.
    fn create_new(&self, path: &Path) -> io::Result<Box<dyn WriteHandle>>;

    /// Moves a file, replacing whatever is at the destination. It cannot cross a file system.
    fn rename(&self, from: &Path, to: &Path) -> io::Result<()>;

    fn remove_file(&self, path: &Path) -> io::Result<()>;

    /// Creates a directory and every parent of it that is missing, and succeeds when they are
    /// all there already.
    fn create_dir_all(&self, path: &Path) -> io::Result<()>;

    fn mode(&self, path: &Path) -> io::Result<Mode>;

    fn set_mode(&self, path: &Path, mode: Mode) -> io::Result<()>;

    /// Whether anything is at `path`. A path that cannot be looked at is an error rather than
    /// an absence, since a write refused because the destination exists must not be allowed
    /// through by a permission that could not be read.
    fn exists(&self, path: &Path) -> io::Result<bool>;

    /// Whether two paths name one file, which is a question for the file system rather than
    /// for the text of the two paths: the two answers differ exactly where the damage would
    /// be. A path that is not there names no file, so it is one file with nothing.
    fn same_file(&self, a: &Path, b: &Path) -> io::Result<bool>;

    /// The identity of whatever is at `path` right now (`stat`), or `None` when nothing is.
    /// The other half of a lock's release check: see [`FileId`].
    fn identity_at(&self, path: &Path) -> io::Result<Option<FileId>>;
}

/// Writes `bytes` to `path` so that a reader sees the old file or the new one whole.
///
/// Commands call this rather than the seam, so none of them has to know a temp file exists.
/// A rename replaces the inode, so the mode of the file being replaced is carried to the temp
/// file, and carried before the bytes are written: a document nobody else may read must not
/// have its contents sitting beside it, under a looser mode, for as long as the write takes.
/// A file that was not there has no mode to carry and gets the default. The owner, the group,
/// access control lists and extended attributes are not carried.
///
/// It does not promise that no temp file is left behind, since a kill that cannot be caught
/// leaves one. The temp file this call made is removed when the call itself fails, on the
/// evidence that it was made here and by nobody else.
///
/// `_lock` is not inspected: it makes a call without a lock fail to compile (SPC-10). Which lock
/// a command must hold for `path` is the command's to get right.
pub fn write_atomically(
    fs: &dyn Fs,
    _lock: &crate::namespace_lock::NamespaceLock<'_>,
    path: &Path,
    bytes: &[u8],
) -> io::Result<()> {
    let carried = match fs.mode(path) {
        Ok(mode) => Some(mode),
        Err(e) if e.kind() == io::ErrorKind::NotFound => None,
        Err(e) => return Err(e),
    };

    let temp = temp_path(path);
    let mut handle = fs.create_new(&temp)?;

    // The handle was opened before the mode was narrowed, and a mode is read when a file is
    // opened rather than when it is written, so a carried mode of 0400 still takes the bytes.
    let written = match carried {
        Some(mode) => fs.set_mode(&temp, mode),
        None => Ok(()),
    }
    .and_then(|()| handle.write_all(bytes))
    .and_then(|()| fs.rename(&temp, path));

    if written.is_err() {
        // A tidy folder does not matter as much as the error the caller is about to see.
        let _ = fs.remove_file(&temp);
    }
    written
}

/// The first half of [`write_atomically`]: a temp file beside `path` holding `bytes`, with
/// `path`'s mode carried to it, not yet renamed, for a caller that must prepare every file
/// before renaming any (`mv`, SPC-2). The temp file is removed if this call fails; once it is
/// returned, it is the caller's.
pub fn prepare_replacement(
    fs: &dyn Fs,
    _lock: &crate::namespace_lock::NamespaceLock<'_>,
    path: &Path,
    bytes: &[u8],
) -> io::Result<PathBuf> {
    let carried = match fs.mode(path) {
        Ok(mode) => Some(mode),
        Err(e) if e.kind() == io::ErrorKind::NotFound => None,
        Err(e) => return Err(e),
    };
    let temp = temp_path(path);
    let mut handle = fs.create_new(&temp)?;
    let written = match carried {
        Some(mode) => fs.set_mode(&temp, mode),
        None => Ok(()),
    }
    .and_then(|()| handle.write_all(bytes));
    match written {
        Ok(()) => Ok(temp),
        Err(e) => {
            let _ = fs.remove_file(&temp);
            Err(e)
        }
    }
}

/// Creates `path`, which must not exist, and writes `bytes` into it directly, with no temp file:
/// `new` adds a file where none was, so there is nothing a temp file would protect, and
/// `O_EXCL` makes the file system refuse a destination that exists (SPC-10). The folder that
/// holds `path` is created first.
///
/// `_lock` is not inspected, as in [`write_atomically`].
pub fn create_exclusively(
    fs: &dyn Fs,
    _lock: &crate::namespace_lock::NamespaceLock<'_>,
    path: &Path,
    bytes: &[u8],
) -> io::Result<()> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs.create_dir_all(parent)?;
    }
    let mut handle = fs.create_new(path)?;
    let written = handle.write_all(bytes);
    if written.is_err() {
        // This is the document itself, not a temp file: left half written, it would look like
        // a real one.
        let _ = fs.remove_file(path);
    }
    written
}

/// Every leftover temp file below `dir`. Symbolic links are not followed, and a folder that is
/// a project of its own is not entered: its leftovers are its own. A read, so it takes no lock.
pub fn find_leftovers(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    find_leftovers_into(dir, &mut found);
    found
}

fn find_leftovers_into(dir: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_symlink() {
            continue;
        }
        let path = entry.path();
        if file_type.is_dir() {
            if crate::config::config_file(&path).is_file() {
                continue;
            }
            find_leftovers_into(&path, found);
        } else if file_type.is_file() && entry.file_name().to_str().is_some_and(is_temp_name) {
            found.push(path);
        }
    }
}

/// Removes `paths`, which must lie within the scope of `_lock`: while it is held no other typdoc
/// writes there, so a leftover belongs to a process that has gone. Age is never a criterion
/// (SPC-10). A removal that fails is skipped: the caller's own write matters more than a tidy
/// folder. Returns how many were removed.
///
/// `_lock` is not inspected; which held lock covers each path is the caller's to get right.
pub fn remove_leftovers(
    fs: &dyn Fs,
    _lock: &crate::namespace_lock::NamespaceLock<'_>,
    paths: &[PathBuf],
) -> usize {
    paths
        .iter()
        .filter(|path| fs.remove_file(path).is_ok())
        .count()
}

/// How many temp names this process has already made. It is what keeps two writers inside one
/// process apart, and it is counted rather than drawn, so that two names an instant apart
/// cannot be one name.
static MADE: AtomicU64 = AtomicU64::new(0);

/// Where the temp file for `path` goes: beside it, because a rename cannot cross a file
/// system, under a name of the reserved shape.
///
/// Three things keep two writers from choosing one name. Between processes it is the process
/// id, which no two live processes share. Inside one process it is `MADE`. Against a leftover
/// of a process that is gone and whose id has since been given out again, it is the drawn
/// part, and behind that stands the create, which refuses a name that is taken rather than
/// replacing what is under it.
fn temp_path(path: &Path) -> PathBuf {
    // A path of one component has an empty parent, which joins to the bare name, so a temp
    // file for it goes where the file itself goes: beside it, in the working directory.
    let directory = path.parent().unwrap_or(Path::new(""));
    let drawn = RandomState::new().build_hasher().finish();
    let counted = MADE.fetch_add(1, Ordering::Relaxed);
    directory.join(format!(
        "{TEMP_PREFIX}{}-{drawn:016x}{counted:x}",
        std::process::id()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Written out here rather than read from the constant: the shape is what the walker
    /// will skip by rule, so a change to it is a change a test reports.
    #[test]
    fn the_reserved_shape_is_the_one_a_document_never_has() {
        assert_eq!(TEMP_PREFIX, ".typdoc-tmp-");
        assert!(is_temp_name(".typdoc-tmp-1234-abcd"));
        assert!(!is_temp_name("note.md"));
        assert!(
            !is_temp_name(".typdoc-tmp"),
            "a name with no part after the shape is a name someone could have written"
        );
    }

    #[test]
    fn a_temp_file_goes_beside_the_file_it_will_replace() {
        let temp = temp_path(Path::new("/a/b/note.md"));

        assert_eq!(temp.parent(), Some(Path::new("/a/b")));
    }

    #[test]
    fn a_path_with_no_directory_puts_its_temp_file_in_the_working_directory() {
        let temp = temp_path(Path::new("note.md"));

        assert_eq!(temp.parent(), Some(Path::new("")));
        assert!(is_temp_name(&name_of(&temp)), "{temp:?}");
    }

    fn name_of(path: &Path) -> String {
        path.file_name()
            .and_then(|n| n.to_str())
            .map(str::to_owned)
            .unwrap_or_default()
    }

    #[test]
    fn a_temp_name_has_the_reserved_shape_with_this_process_and_a_part_that_does_not_repeat() {
        let (one, two) = (
            name_of(&temp_path(Path::new("note.md"))),
            name_of(&temp_path(Path::new("note.md"))),
        );

        assert!(is_temp_name(&one), "{one}");
        assert!(
            one.starts_with(&format!("{TEMP_PREFIX}{}-", std::process::id())),
            "{one}"
        );
        assert_ne!(
            one, two,
            "two temp names in one process were the same, so two writers in it could choose one"
        );
    }
}
