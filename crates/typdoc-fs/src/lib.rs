//! The write half of the seam: the file system itself.
//!
//! This crate is the one place a file is changed. Everything else changes a file through
//! [`typdoc_core::Fs`], and `typdoc-core`'s lint list refuses the calls that would go round it,
//! with no exception. Which code may write is settled by the dependency graph: a crate that can
//! write says so in its manifest, where a review sees it, rather than by an attribute a later
//! change could add unnoticed.
//!
//! The rules about temp files, modes and renames sit above the seam, in `typdoc_core`, so they
//! are the same whichever implementation is underneath.

use std::fs;
use std::io::{self, Write};
#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::Path;

use typdoc_core::{FileId, Fs, Mode, WriteHandle};

/// The file system this process is running on. Linux is the platform that is run and claimed,
/// and the mode and the identity of a file are read the way Unix reports them.
///
/// On Windows every write is refused before it creates a file. A lock's release tells its own
/// file from another process's by the file's identity, and a replaced document keeps its mode;
/// neither has a Windows reading here, and a write without them would lose a guarantee without
/// saying so. So `create_new`, which makes the lock file every write takes first, refuses, and
/// so do the reads of mode and identity rather than answer with a value made up to pass.
pub struct SystemFs;

impl Fs for SystemFs {
    fn create_new(&self, path: &Path) -> io::Result<Box<dyn WriteHandle>> {
        refuse_on_windows()?;
        Ok(Box::new(OpenFile(fs::File::create_new(path)?)))
    }

    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        fs::rename(from, to)
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
        fs::remove_file(path)
    }

    fn create_dir_all(&self, path: &Path) -> io::Result<()> {
        fs::create_dir_all(path)
    }

    fn mode(&self, path: &Path) -> io::Result<Mode> {
        mode_at(path)
    }

    fn set_mode(&self, path: &Path, mode: Mode) -> io::Result<()> {
        set_mode_at(path, mode)
    }

    fn exists(&self, path: &Path) -> io::Result<bool> {
        match fs::metadata(path) {
            Ok(_) => Ok(true),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(e),
        }
    }

    fn same_file(&self, a: &Path, b: &Path) -> io::Result<bool> {
        let (Some(a), Some(b)) = (self.identity_at(a)?, self.identity_at(b)?) else {
            return Ok(false);
        };
        Ok(a.device == b.device && a.inode == b.inode)
    }

    fn identity_at(&self, path: &Path) -> io::Result<Option<FileId>> {
        refuse_on_windows()?;
        match fs::metadata(path) {
            Ok(data) => Ok(Some(file_id(&data)?)),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e),
        }
    }
}

#[cfg(unix)]
fn refuse_on_windows() -> io::Result<()> {
    Ok(())
}

#[cfg(windows)]
fn refuse_on_windows() -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "typdoc does not write files on Windows yet",
    ))
}

#[cfg(unix)]
fn mode_at(path: &Path) -> io::Result<Mode> {
    Ok(fs::metadata(path)?.permissions().mode())
}

#[cfg(windows)]
fn mode_at(_path: &Path) -> io::Result<Mode> {
    refuse_on_windows().map(|()| 0)
}

#[cfg(unix)]
fn set_mode_at(path: &Path, mode: Mode) -> io::Result<()> {
    fs::set_permissions(path, fs::Permissions::from_mode(mode))
}

#[cfg(windows)]
fn set_mode_at(_path: &Path, _mode: Mode) -> io::Result<()> {
    refuse_on_windows()
}

#[cfg(unix)]
fn file_id(data: &fs::Metadata) -> io::Result<FileId> {
    Ok(FileId {
        device: data.dev(),
        inode: data.ino(),
        links: data.nlink(),
    })
}

#[cfg(windows)]
fn file_id(_data: &fs::Metadata) -> io::Result<FileId> {
    refuse_on_windows().map(|()| FileId {
        device: 0,
        inode: 0,
        links: 0,
    })
}

struct OpenFile(fs::File);

impl WriteHandle for OpenFile {
    fn write_all(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.0.write_all(bytes)
    }

    fn sync(&mut self) -> io::Result<()> {
        self.0.sync_all()
    }

    fn identity(&self) -> io::Result<FileId> {
        file_id(&self.0.metadata()?)
    }
}
