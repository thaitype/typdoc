//! The mode and the identity of a file, read the way Unix reports them.

use std::fs;
use std::io;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::Path;

use typdoc_core::{FileId, Mode};

pub(crate) fn rename(from: &Path, to: &Path) -> io::Result<()> {
    fs::rename(from, to)
}

pub(crate) fn mode_at(path: &Path) -> io::Result<Mode> {
    Ok(fs::metadata(path)?.permissions().mode())
}

pub(crate) fn set_mode_at(path: &Path, mode: Mode) -> io::Result<()> {
    fs::set_permissions(path, fs::Permissions::from_mode(mode))
}

pub(crate) fn identity_at(path: &Path) -> io::Result<Option<FileId>> {
    match fs::metadata(path) {
        Ok(data) => Ok(Some(file_id(&data))),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}

pub(crate) fn identity_of(file: &fs::File) -> io::Result<FileId> {
    Ok(file_id(&file.metadata()?))
}

fn file_id(data: &fs::Metadata) -> FileId {
    FileId {
        device: data.dev(),
        inode: u128::from(data.ino()),
        links: data.nlink(),
    }
}
