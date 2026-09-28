//! A file's DACL, read and set as SDDL text, for the Windows tests that a write carries it.

use std::fs::{File, OpenOptions};
use std::io;
use std::os::windows::fs::OpenOptionsExt;
use std::os::windows::io::AsRawHandle;
use std::path::Path;

use windows_sys::Win32::Foundation::{HANDLE, LocalFree};
use windows_sys::Win32::Security::Authorization::{
    ConvertSecurityDescriptorToStringSecurityDescriptorW,
    ConvertStringSecurityDescriptorToSecurityDescriptorW, GetSecurityInfo, SDDL_REVISION_1,
    SE_FILE_OBJECT, SetSecurityInfo,
};
use windows_sys::Win32::Security::{
    ACL, DACL_SECURITY_INFORMATION, GetSecurityDescriptorControl, GetSecurityDescriptorDacl,
    PROTECTED_DACL_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR, SE_DACL_PROTECTED,
    UNPROTECTED_DACL_SECURITY_INFORMATION,
};
use windows_sys::Win32::Storage::FileSystem::{
    FILE_FLAG_BACKUP_SEMANTICS, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, READ_CONTROL,
    WRITE_DAC,
};

fn open(path: &Path, access: u32) -> io::Result<File> {
    OpenOptions::new()
        .access_mode(access)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
        .open(path)
}

/// The DACL of `path` as SDDL, such as `D:PAI(A;;FA;;;BA)`.
pub fn sddl(path: &Path) -> io::Result<String> {
    let file = open(path, READ_CONTROL)?;
    let mut descriptor: PSECURITY_DESCRIPTOR = std::ptr::null_mut();
    // SAFETY: the handle is open for `READ_CONTROL`; the descriptor is freed below.
    let status = unsafe {
        GetSecurityInfo(
            file.as_raw_handle() as HANDLE,
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
        return Err(io::Error::from_raw_os_error(status as i32));
    }
    let mut text: *mut u16 = std::ptr::null_mut();
    let mut length = 0u32;
    // SAFETY: `descriptor` is valid; `text` is allocated by the call and freed below.
    let converted = unsafe {
        ConvertSecurityDescriptorToStringSecurityDescriptorW(
            descriptor,
            SDDL_REVISION_1,
            DACL_SECURITY_INFORMATION,
            &mut text,
            &mut length,
        )
    };
    let result = if converted == 0 {
        Err(io::Error::last_os_error())
    } else {
        // SAFETY: the call wrote `length` UTF-16 units at `text`.
        let units = unsafe { std::slice::from_raw_parts(text, length as usize) };
        Ok(String::from_utf16_lossy(units)
            .trim_end_matches('\0')
            .to_owned())
    };
    // SAFETY: both were allocated by the calls above and are not used after this.
    unsafe {
        LocalFree(text.cast());
        LocalFree(descriptor);
    }
    result
}

/// Gives `path` the DACL written as SDDL in `text`, protected when the text says `D:P`.
pub fn set_sddl(path: &Path, text: &str) -> io::Result<()> {
    let wide: Vec<u16> = text.encode_utf16().chain(Some(0)).collect();
    let mut descriptor: PSECURITY_DESCRIPTOR = std::ptr::null_mut();
    // SAFETY: `wide` is NUL-terminated; the descriptor is freed below.
    let parsed = unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            wide.as_ptr(),
            SDDL_REVISION_1,
            &mut descriptor,
            std::ptr::null_mut(),
        )
    };
    if parsed == 0 {
        return Err(io::Error::last_os_error());
    }
    let result = apply(path, descriptor);
    // SAFETY: allocated by the parse above and not used after this.
    unsafe { LocalFree(descriptor) };
    result
}

fn apply(path: &Path, descriptor: PSECURITY_DESCRIPTOR) -> io::Result<()> {
    let (mut present, mut defaulted) = (0, 0);
    let mut dacl: *mut ACL = std::ptr::null_mut();
    let (mut control, mut revision) = (0u16, 0u32);
    // SAFETY: `descriptor` is valid and every out pointer is to a local.
    unsafe {
        GetSecurityDescriptorDacl(descriptor, &mut present, &mut dacl, &mut defaulted);
        GetSecurityDescriptorControl(descriptor, &mut control, &mut revision);
    }
    let inheritance = if control & SE_DACL_PROTECTED != 0 {
        PROTECTED_DACL_SECURITY_INFORMATION
    } else {
        UNPROTECTED_DACL_SECURITY_INFORMATION
    };
    let file = open(path, READ_CONTROL | WRITE_DAC)?;
    // SAFETY: the handle is open for both rights; `dacl` points into `descriptor`, alive here.
    let status = unsafe {
        SetSecurityInfo(
            file.as_raw_handle() as HANDLE,
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
