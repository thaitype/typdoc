//! What `SystemFs` reads and does the Unix way, done the Windows way, as measured on a Windows
//! runner (story 9, TK-12):
//!
//! - A file's identity is its volume serial number and its 128-bit file id, read from a handle.
//!   A handle whose file is waiting to be deleted reports no links, as an unlinked file does on
//!   Unix.
//! - A mode is the read-only flag: `0o444` when it is set, `0o666` when not.
//! - A rename that replaces a file first carries the replaced file's DACL to the file taking its
//!   place, then renames with POSIX semantics, which replaces a read-only file and leaves no
//!   moment where the name is missing. A DACL that cannot be read or carried stops the rename.

use std::ffi::c_void;
use std::fs::{self, File, OpenOptions};
use std::io;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::fs::OpenOptionsExt;
use std::os::windows::io::AsRawHandle;
use std::path::Path;

use typdoc_core::{FileId, Mode};
use windows_sys::Win32::Foundation::{ERROR_SHARING_VIOLATION, HANDLE, LocalFree};
use windows_sys::Win32::Security::Authorization::{
    GetSecurityInfo, SE_FILE_OBJECT, SetSecurityInfo,
};
use windows_sys::Win32::Security::{
    ACL, DACL_SECURITY_INFORMATION, GetSecurityDescriptorControl, GetSecurityDescriptorDacl,
    PROTECTED_DACL_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR, SE_DACL_PROTECTED,
    UNPROTECTED_DACL_SECURITY_INFORMATION,
};
use windows_sys::Win32::Storage::FileSystem::{
    DELETE, FILE_FLAG_BACKUP_SEMANTICS, FILE_ID_INFO, FILE_RENAME_INFO, FILE_SHARE_DELETE,
    FILE_SHARE_READ, FILE_SHARE_WRITE, FILE_STANDARD_INFO, FileIdInfo, FileRenameInfoEx,
    FileStandardInfo, GetFileInformationByHandleEx, READ_CONTROL, SetFileInformationByHandle,
    WRITE_DAC,
};

/// `FILE_RENAME_FLAG_REPLACE_IF_EXISTS`, `FILE_RENAME_FLAG_POSIX_SEMANTICS` and
/// `FILE_RENAME_FLAG_IGNORE_READONLY_ATTRIBUTE`, which `windows-sys` keeps in other modules.
const RENAME_FLAGS: u32 = 0x1 | 0x2 | 0x40;

const SHARE_ALL: u32 = FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE;

/// What a replace that another program's open handle blocks says (os error 32).
const HELD_OPEN: &str = "another program has this file open and does not let it be replaced; close it there and run the command again";

/// A handle to `path` for `access`, which lets everyone else keep reading, writing and deleting.
fn open(path: &Path, access: u32) -> io::Result<File> {
    OpenOptions::new()
        .access_mode(access)
        .share_mode(SHARE_ALL)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
        .open(path)
}

pub(crate) fn rename(from: &Path, to: &Path) -> io::Result<()> {
    carry_dacl(to, from)?;
    posix_rename(from, to)
}

pub(crate) fn mode_at(path: &Path) -> io::Result<Mode> {
    let read_only = fs::metadata(path)?.permissions().readonly();
    Ok(if read_only { 0o444 } else { 0o666 })
}

pub(crate) fn set_mode_at(path: &Path, mode: Mode) -> io::Result<()> {
    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_readonly(mode & 0o222 == 0);
    fs::set_permissions(path, permissions)
}

/// A path that cannot be opened because its file is waiting to be deleted under the older
/// delete semantics answers access denied; it names no file of ours, so it reads as none, and a
/// lock's release then removes nothing.
pub(crate) fn identity_at(path: &Path) -> io::Result<Option<FileId>> {
    match open(path, 0) {
        Ok(file) => identity_of(&file).map(Some),
        Err(e)
            if matches!(
                e.kind(),
                io::ErrorKind::NotFound | io::ErrorKind::PermissionDenied
            ) =>
        {
            Ok(None)
        }
        Err(e) => Err(e),
    }
}

pub(crate) fn identity_of(file: &File) -> io::Result<FileId> {
    let handle = file.as_raw_handle() as HANDLE;
    // SAFETY: both structures are plain data, for which all zeroes is a valid value.
    let (mut id, mut standard): (FILE_ID_INFO, FILE_STANDARD_INFO) =
        unsafe { (std::mem::zeroed(), std::mem::zeroed()) };
    // SAFETY: `handle` is open for as long as `file` is borrowed, and each pointer and size
    // describe the structure the class asks for.
    let read = unsafe {
        GetFileInformationByHandleEx(
            handle,
            FileIdInfo,
            (&raw mut id).cast::<c_void>(),
            size_of::<FILE_ID_INFO>() as u32,
        ) != 0
            && GetFileInformationByHandleEx(
                handle,
                FileStandardInfo,
                (&raw mut standard).cast::<c_void>(),
                size_of::<FILE_STANDARD_INFO>() as u32,
            ) != 0
    };
    if !read {
        return Err(io::Error::last_os_error());
    }
    Ok(FileId {
        device: id.VolumeSerialNumber,
        inode: u128::from_le_bytes(id.FileId.Identifier),
        links: if standard.DeletePending {
            0
        } else {
            u64::from(standard.NumberOfLinks)
        },
    })
}

/// Puts `from`'s DACL, protected or inheriting as it is there, on `to`. Nothing is carried when
/// `from` does not exist. A DACL that cannot be read or set is an error: a document is never
/// replaced by a file that grants other access than it did.
fn carry_dacl(from: &Path, to: &Path) -> io::Result<()> {
    let source = match open(from, READ_CONTROL) {
        Ok(file) => file,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(not_carried(&e)),
    };
    let mut descriptor: PSECURITY_DESCRIPTOR = std::ptr::null_mut();
    // SAFETY: the handle is open for `READ_CONTROL`, and the descriptor it fills is freed below.
    let status = unsafe {
        GetSecurityInfo(
            source.as_raw_handle() as HANDLE,
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &mut descriptor,
        )
    };
    if status != 0 {
        return Err(not_carried(&io::Error::from_raw_os_error(status as i32)));
    }
    let set = set_dacl(to, descriptor);
    // SAFETY: `descriptor` was allocated by `GetSecurityInfo` and nothing points into it now.
    unsafe { LocalFree(descriptor) };
    set.map_err(|e| not_carried(&e))
}

/// A DACL absent from the descriptor is carried as absent, which is what the old file had.
fn set_dacl(to: &Path, descriptor: PSECURITY_DESCRIPTOR) -> io::Result<()> {
    let (mut present, mut defaulted) = (0, 0);
    let mut dacl: *mut ACL = std::ptr::null_mut();
    let (mut control, mut revision) = (0u16, 0u32);
    // SAFETY: `descriptor` is a valid descriptor from `GetSecurityInfo`, and every out pointer
    // is to a local.
    let read = unsafe {
        GetSecurityDescriptorDacl(descriptor, &mut present, &mut dacl, &mut defaulted) != 0
            && GetSecurityDescriptorControl(descriptor, &mut control, &mut revision) != 0
    };
    if !read {
        return Err(io::Error::last_os_error());
    }
    let inheritance = if control & SE_DACL_PROTECTED != 0 {
        PROTECTED_DACL_SECURITY_INFORMATION
    } else {
        UNPROTECTED_DACL_SECURITY_INFORMATION
    };
    // `SetSecurityInfo` reads the descriptor it changes, so the handle needs both rights.
    let target = open(to, READ_CONTROL | WRITE_DAC)?;
    // SAFETY: the handle is open for both rights, and `dacl` points into `descriptor`, which the
    // caller keeps alive until this returns.
    let status = unsafe {
        SetSecurityInfo(
            target.as_raw_handle() as HANDLE,
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION | inheritance,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            dacl,
            std::ptr::null(),
        )
    };
    if status == 0 {
        Ok(())
    } else {
        Err(io::Error::from_raw_os_error(status as i32))
    }
}

fn not_carried(e: &io::Error) -> io::Error {
    io::Error::new(
        e.kind(),
        format!(
            "its permissions could not be carried to the replacement, so it was not replaced: {e}"
        ),
    )
}

/// Renames `from` over `to` through `from`'s own handle, with POSIX semantics.
fn posix_rename(from: &Path, to: &Path) -> io::Result<()> {
    let source = open(from, DELETE)?;
    let name: Vec<u16> = to.as_os_str().encode_wide().collect();
    let size = std::mem::offset_of!(FILE_RENAME_INFO, FileName) + (name.len() + 1) * 2;
    // Aligned for the structure: its first field is a pointer-sized union.
    let mut buffer = vec![0u64; size.div_ceil(8)];
    let info = buffer.as_mut_ptr().cast::<FILE_RENAME_INFO>();
    // SAFETY: `buffer` is zeroed, aligned and `size` bytes long, which holds the header and the
    // name after it, and `name.len() * 2` fits a `u32` for any path Windows accepts.
    unsafe {
        (*info).Anonymous.Flags = RENAME_FLAGS;
        (*info).FileNameLength = (name.len() * 2) as u32;
        std::ptr::copy_nonoverlapping(
            name.as_ptr(),
            (&raw mut (*info).FileName).cast::<u16>(),
            name.len(),
        );
    }
    // SAFETY: the handle is open for `DELETE`, and `info` describes `size` initialized bytes.
    let renamed = unsafe {
        SetFileInformationByHandle(
            source.as_raw_handle() as HANDLE,
            FileRenameInfoEx,
            info.cast::<c_void>(),
            size as u32,
        ) != 0
    };
    if renamed {
        return Ok(());
    }
    let e = io::Error::last_os_error();
    if e.raw_os_error() == Some(ERROR_SHARING_VIOLATION as i32) {
        return Err(io::Error::new(e.kind(), HELD_OPEN));
    }
    Err(e)
}
