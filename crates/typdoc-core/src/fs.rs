//! The seam every write goes through, and the one function above it that holds what an
//! interrupted write may leave behind.
//!
//! The seam is the file operations a write is built from rather than "write this document",
//! so the rules about temp files, modes and renames sit above it, here. The write half of it is
//! the `typdoc-fs` crate, which is the only crate that may change a file: no module of this one
//! may, and the lint list beside this crate has no exception in it.

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
/// `device` and `inode` are what a lock's release compares: a `stat` on the path a lock file
/// sits at, against an `fstat` on the handle held since the lock was created. `links` is read
/// from the handle side only, since the design's second removal check ("or the open file's
/// link count is zero") is about whether the file the handle still refers to has been unlinked,
/// which a `stat` on a path that may now lead to a different file cannot tell.
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

    /// The identity the file system gives this open handle (`fstat`), unaffected by anything
    /// that happens to the path it was opened at: the inode stays alive, and reachable through
    /// the handle, for as long as the handle is open, whatever a later writer does to the name.
    /// This is what lets a lock's release tell its own file from whatever the path currently
    /// leads to.
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

    /// Removes a file.
    fn remove_file(&self, path: &Path) -> io::Result<()>;

    /// Creates a directory and every parent of it that is missing, and succeeds when they are
    /// all there already.
    fn create_dir_all(&self, path: &Path) -> io::Result<()>;

    /// The permission bits of the file at `path`.
    fn mode(&self, path: &Path) -> io::Result<Mode>;

    /// Sets the permission bits of the file at `path`.
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
/// This is the one function above the seam, so that no command has to know a temp file exists.
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
/// `_lock` is not inspected: it is here so that this function cannot be called without one
/// (decision 6, "every function that writes takes it by reference"). There is no check that it
/// is the right lock for `path`'s namespace; that a lock exists at all is what the type proves,
/// which of the possibly several held locks a command must hold before calling this is a
/// property of the command, not of this function.
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

/// Creates `path`, which must not already exist, and writes `bytes` into it directly: no temp
/// file and no rename, unlike [`write_atomically`]. `new` is the caller this is for, and its own
/// document is what decides the shape: the file being created adds no state a temp file would
/// have to protect (decision 7, "`new` keeps `O_EXCL` because it is one call that adds no state
/// and costs nothing"). Fails with [`io::ErrorKind::AlreadyExists`] when `path` is already there,
/// which is how the file system enforces decision 15's refusal rather than typdoc remembering to
/// look first. The folder that will hold `path` is created first, the same way
/// [`crate::namespace_lock::acquire`] creates its own lock folder before the first lock a project
/// takes, so the first document of a namespace's own folder does not fail on a "not found".
///
/// `_lock` is not inspected, the same as `write_atomically`'s: there is no check that it is the
/// right lock for `path`'s namespace.
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
        // The file created here is the document itself, not a hidden temp file: left in place, a
        // half-written failure would look like a real document rather than nothing having
        // happened. Best effort, the same reasoning `write_atomically` already gives its own
        // cleanup: the write's own error is what the caller needs to see.
        let _ = fs.remove_file(path);
    }
    written
}

/// Every leftover temp file below `dir`, found by an ordinary recursive read: any file whose
/// name has the reserved shape ([`is_temp_name`]), at any depth, a symbolic link never followed
/// (the same rule every other walk of this crate keeps), and a folder holding its own
/// `.typdoc/config.json` never entered, since a separate project's leftovers are its own to
/// find. A read, not a write, so it takes no lock and changes nothing; pair it with
/// [`remove_leftovers`] to act on what it finds.
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

/// Removes every one of `paths` through `fs`, within the scope [`crate::namespace_lock`] proves
/// the caller holds a lock over: a command holding a lock may remove leftovers there, on the
/// evidence that no other typdoc is writing in that namespace while the lock is held (decision
/// 4). Age is never a criterion, as it is never one for a lock: every path handed in is removed,
/// whatever its age. A removal that fails is not reported here and does not stop the sweep —
/// the caller's own write matters more than a tidy folder, the same reasoning
/// [`write_atomically`] already carries for the temp file it made itself. Returns how many were
/// actually removed, for a caller that wants to say so.
///
/// `_lock` is not inspected, the same as `write_atomically`'s: there is no check that it is the
/// right lock for every path in `paths`, which of the possibly several held locks proves a
/// caller may remove a given path is a property of the caller, not of this function.
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
