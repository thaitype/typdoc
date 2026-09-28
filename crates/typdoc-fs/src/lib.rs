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
use std::path::Path;

use typdoc_core::{FileId, Fs, Mode, WriteHandle};

#[cfg(unix)]
mod unix;
#[cfg(unix)]
use unix as platform;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
use windows as platform;

/// The file system this process is running on: Unix, or Windows (`windows.rs` says how each
/// Unix reading is made there).
pub struct SystemFs;

impl Fs for SystemFs {
    fn create_new(&self, path: &Path) -> io::Result<Box<dyn WriteHandle>> {
        Ok(Box::new(OpenFile(fs::File::create_new(path)?)))
    }

    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        platform::rename(from, to)
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
        fs::remove_file(path)
    }

    fn create_dir_all(&self, path: &Path) -> io::Result<()> {
        fs::create_dir_all(path)
    }

    fn mode(&self, path: &Path) -> io::Result<Mode> {
        platform::mode_at(path)
    }

    fn set_mode(&self, path: &Path, mode: Mode) -> io::Result<()> {
        platform::set_mode_at(path, mode)
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
        platform::identity_at(path)
    }
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
        platform::identity_of(&self.0)
    }
}
