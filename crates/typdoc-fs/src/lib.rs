//! The write half of the seam: the file system itself.
//!
//! This crate exists to be the one place a file is changed. Everything else reaches a file
//! system through [`typdoc_core::Fs`], and `typdoc-core`'s own lint list refuses the calls that
//! would go round it, with no exception in it anywhere. Which code may write is therefore
//! settled by the dependency graph: a crate that can write has to say so in its manifest, where
//! it can be searched for and seen in a review, instead of by an attribute that a later one
//! could be added beside without anyone noticing.
//!
//! Every function below is one operation of that trait and calls exactly the standard-library
//! function that performs it. The rules about temp files, modes and renames are not here: they
//! sit above the seam, in `typdoc_core`, so that they are the same whichever implementation is
//! underneath.

use std::fs;
use std::io::{self, Write};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::Path;

use typdoc_core::{Fs, Mode, WriteHandle};

/// The file system this process is running on. Linux is the platform that is run and claimed,
/// and the mode and the identity of a file are read the way Unix reports them.
pub struct SystemFs;

impl Fs for SystemFs {
    fn create_new(&self, path: &Path) -> io::Result<Box<dyn WriteHandle>> {
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
        Ok(fs::metadata(path)?.permissions().mode())
    }

    fn set_mode(&self, path: &Path, mode: Mode) -> io::Result<()> {
        fs::set_permissions(path, fs::Permissions::from_mode(mode))
    }

    fn exists(&self, path: &Path) -> io::Result<bool> {
        match fs::metadata(path) {
            Ok(_) => Ok(true),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(e),
        }
    }

    fn same_file(&self, a: &Path, b: &Path) -> io::Result<bool> {
        let (Some(a), Some(b)) = (identity(a)?, identity(b)?) else {
            return Ok(false);
        };
        Ok(a == b)
    }
}

/// The device and inode a path leads to, or nothing when the path leads nowhere.
fn identity(path: &Path) -> io::Result<Option<(u64, u64)>> {
    match fs::metadata(path) {
        Ok(data) => Ok(Some((data.dev(), data.ino()))),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}

/// A file this process opened for writing.
struct OpenFile(fs::File);

impl WriteHandle for OpenFile {
    fn write_all(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.0.write_all(bytes)
    }

    fn sync(&mut self) -> io::Result<()> {
        self.0.sync_all()
    }
}
