//! The write half of the seam: the file system itself.
//!
//! This is the only module of this crate that calls a function which changes a file. Every
//! other module reaches a file system through [`Fs`], which is why the expectation below is
//! module-wide here and nowhere else: a write that appears in any other module of this crate
//! is caught by the lint rather than by anyone remembering to look.
#![expect(
    clippy::disallowed_methods,
    reason = "this module is the write half of the seam `Fs` names: every function of the \
              implementation below is one operation of that trait and calls exactly the \
              standard-library function that performs it, and `Deps` hands the rest of the \
              program the trait rather than this type, so a write from any other module has \
              no way through here and is caught where it stands"
)]

use std::fs;
use std::io::{self, Write};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::Path;

use super::{Fs, Mode, WriteHandle};

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
