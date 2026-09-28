//! Prints what copying a DACL from one file to another does on Windows, one `RESULT` line each.

#[cfg(not(windows))]
fn main() {
    println!("RESULT platform: not windows, nothing measured");
}

#[cfg(windows)]
fn main() {
    win::run();
}

#[cfg(windows)]
mod win {
    use std::ffi::c_void;
    use std::fs::{self, OpenOptions};
    use std::os::windows::ffi::OsStrExt;
    use std::os::windows::fs::OpenOptionsExt;
    use std::os::windows::io::AsRawHandle;
    use std::path::{Path, PathBuf};

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
    use windows_sys::Win32::Storage::FileSystem::{DELETE, FILE_RENAME_INFO, FileRenameInfoEx, SetFileInformationByHandle};

    const READ_CONTROL: u32 = 0x0002_0000;
    const WRITE_DAC: u32 = 0x0004_0000;
    const RENAME_FLAGS: u32 = 0x1 | 0x2 | 0x40; // replace-if-exists, POSIX semantics, ignore read-only

    fn result(key: &str, value: impl std::fmt::Display) {
        println!("RESULT {key}: {value}");
    }

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(Some(0)).collect()
    }

    fn open(p: &Path, access: u32) -> std::io::Result<std::fs::File> {
        OpenOptions::new().access_mode(access).share_mode(0x7).custom_flags(0x0200_0000).open(p)
    }

    /// The file's DACL as SDDL, read through a handle opened for READ_CONTROL.
    fn sddl(p: &Path) -> String {
        let f = match open(p, READ_CONTROL) {
            Ok(f) => f,
            Err(e) => return format!("open for READ_CONTROL: err os={:?}", e.raw_os_error()),
        };
        let mut sd: PSECURITY_DESCRIPTOR = std::ptr::null_mut();
        let err = unsafe {
            GetSecurityInfo(f.as_raw_handle() as HANDLE, SE_FILE_OBJECT, DACL_SECURITY_INFORMATION, std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut(), &mut sd)
        };
        if err != 0 {
            return format!("GetSecurityInfo err {err}");
        }
        let mut text: *mut u16 = std::ptr::null_mut();
        let mut len = 0u32;
        let ok = unsafe { ConvertSecurityDescriptorToStringSecurityDescriptorW(sd, SDDL_REVISION_1, DACL_SECURITY_INFORMATION, &mut text, &mut len) };
        let out = if ok == 0 {
            format!("convert err {}", std::io::Error::last_os_error())
        } else {
            let s = unsafe { std::slice::from_raw_parts(text, len as usize) };
            String::from_utf16_lossy(s).trim_end_matches('\0').to_owned()
        };
        unsafe {
            LocalFree(text.cast());
            LocalFree(sd);
        }
        out
    }

    /// Puts the DACL of the SDDL string `text` on `p`, protected or not as the SDDL says.
    fn set_sddl(p: &Path, text: &str) -> String {
        let mut sd: PSECURITY_DESCRIPTOR = std::ptr::null_mut();
        let ok = unsafe { ConvertStringSecurityDescriptorToSecurityDescriptorW(wide(text).as_ptr(), SDDL_REVISION_1, &mut sd, std::ptr::null_mut()) };
        if ok == 0 {
            return format!("parse err {}", std::io::Error::last_os_error());
        }
        let r = apply(p, sd);
        unsafe { LocalFree(sd) };
        r
    }

    /// The copy typdoc would do: the old file's DACL, and whether it is protected, onto the temp.
    fn copy_dacl(from: &Path, to: &Path) -> String {
        let f = match open(from, READ_CONTROL) {
            Ok(f) => f,
            Err(e) => return format!("open old for READ_CONTROL: err kind={:?} os={:?}", e.kind(), e.raw_os_error()),
        };
        let mut sd: PSECURITY_DESCRIPTOR = std::ptr::null_mut();
        let err = unsafe {
            GetSecurityInfo(f.as_raw_handle() as HANDLE, SE_FILE_OBJECT, DACL_SECURITY_INFORMATION, std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut(), &mut sd)
        };
        if err != 0 {
            return format!("GetSecurityInfo on old: err {err}");
        }
        let r = apply(to, sd);
        unsafe { LocalFree(sd) };
        r
    }

    fn apply(p: &Path, sd: PSECURITY_DESCRIPTOR) -> String {
        let mut present = 0;
        let mut defaulted = 0;
        let mut dacl: *mut ACL = std::ptr::null_mut();
        let mut control = 0u16;
        let mut revision = 0u32;
        unsafe {
            GetSecurityDescriptorDacl(sd, &mut present, &mut dacl, &mut defaulted);
            GetSecurityDescriptorControl(sd, &mut control, &mut revision);
        }
        if present == 0 || dacl.is_null() {
            return "no DACL (would mean full access to everyone): refused".to_owned();
        }
        let protection = if control & SE_DACL_PROTECTED != 0 { PROTECTED_DACL_SECURITY_INFORMATION } else { UNPROTECTED_DACL_SECURITY_INFORMATION };
        let f = match open(p, WRITE_DAC) {
            Ok(f) => f,
            Err(e) => return format!("open for WRITE_DAC: err kind={:?} os={:?}", e.kind(), e.raw_os_error()),
        };
        let err = unsafe { SetSecurityInfo(f.as_raw_handle() as HANDLE, SE_FILE_OBJECT, DACL_SECURITY_INFORMATION | protection, std::ptr::null_mut(), std::ptr::null_mut(), dacl, std::ptr::null()) };
        if err == 0 { "ok".to_owned() } else { format!("SetSecurityInfo err {err}") }
    }

    fn posix_rename(from: &Path, to: &Path) -> String {
        let source = match open(from, DELETE) {
            Ok(f) => f,
            Err(e) => return format!("open for DELETE: err os={:?}", e.raw_os_error()),
        };
        let name: Vec<u16> = to.as_os_str().encode_wide().collect();
        let header = std::mem::offset_of!(FILE_RENAME_INFO, FileName);
        let size = header + name.len() * 2 + 2;
        let mut buffer = vec![0u64; size.div_ceil(8)];
        let info = buffer.as_mut_ptr().cast::<FILE_RENAME_INFO>();
        unsafe {
            (*info).Anonymous.Flags = RENAME_FLAGS;
            (*info).FileNameLength = (name.len() * 2) as u32;
            std::ptr::copy_nonoverlapping(name.as_ptr(), (&raw mut (*info).FileName).cast::<u16>(), name.len());
        }
        let ok = unsafe { SetFileInformationByHandle(source.as_raw_handle() as HANDLE, FileRenameInfoEx, info.cast::<c_void>(), size as u32) };
        if ok == 0 { format!("err {}", std::io::Error::last_os_error()) } else { "ok".to_owned() }
    }

    fn set_readonly(p: &Path, on: bool) {
        let mut perm = fs::metadata(p).unwrap().permissions();
        perm.set_readonly(on);
        fs::set_permissions(p, perm).unwrap();
    }

    fn scratch() -> PathBuf {
        let d = std::env::temp_dir().join(format!("probe-{}", std::process::id()));
        fs::create_dir_all(&d).unwrap();
        d
    }

    /// One replacement the way typdoc would do it: temp, DACL copied, read-only carried, POSIX rename.
    fn replace(d: &Path, label: &str, old_sddl: Option<&str>, old_readonly: bool) {
        let target = d.join(format!("{label}.md"));
        fs::write(&target, "old").unwrap();
        if let Some(text) = old_sddl {
            result(&format!("{label}: set old DACL {text}"), set_sddl(&target, text));
        }
        let before = sddl(&target);
        result(&format!("{label}: old DACL"), &before);
        if old_readonly {
            set_readonly(&target, true);
        }
        let temp = d.join(format!(".tmp-{label}"));
        fs::write(&temp, "new").unwrap();
        result(&format!("{label}: temp DACL as created"), sddl(&temp));
        let copied = copy_dacl(&target, &temp);
        result(&format!("{label}: copy DACL old -> temp"), &copied);
        if copied != "ok" {
            result(&format!("{label}: target still holds"), format!("{:?}", fs::read_to_string(&target).map_err(|e| e.raw_os_error())));
            let _ = fs::remove_file(&temp);
            return;
        }
        if old_readonly {
            set_readonly(&temp, true);
        }
        result(&format!("{label}: rename"), posix_rename(&temp, &target));
        let after = sddl(&target);
        result(&format!("{label}: result DACL"), &after);
        result(&format!("{label}: DACL kept"), before == after);
        result(&format!("{label}: result read-only"), fs::metadata(&target).map(|m| m.permissions().readonly()).unwrap_or(false));
        result(&format!("{label}: result holds"), format!("{:?}", fs::read_to_string(&target).map_err(|e| e.raw_os_error())));
    }

    pub fn run() {
        let d = scratch();
        result("folder DACL", sddl(&d));
        // Inherited only: what a new file in the folder gets.
        replace(&d, "inherited", None, false);
        // Restrictive and protected: administrators full control, everyone read only, nothing inherited.
        replace(&d, "restrictive", Some("D:P(A;;FA;;;BA)(A;;FA;;;SY)(A;;FR;;;WD)"), false);
        // The same, read-only as well: the DACL goes onto a read-only temp.
        replace(&d, "restrictive-readonly", Some("D:P(A;;FA;;;BA)(A;;FA;;;SY)(A;;FR;;;WD)"), true);
        // The owner is denied reading the DACL: the copy has to fail, and nothing is replaced.
        replace(&d, "owner-denied-read-control", Some("D:P(D;;RC;;;OW)(A;;FA;;;WD)"), false);
        // The owner is denied changing a DACL on the temp: the temp's own DACL is made to deny it first.
        let target = d.join("no-write-dac.md");
        fs::write(&target, "old").unwrap();
        let temp = d.join(".tmp-no-write-dac");
        fs::write(&temp, "new").unwrap();
        result("no-write-dac: deny WRITE_DAC on temp", set_sddl(&temp, "D:P(D;;WD;;;OW)(A;;FA;;;WD)"));
        result("no-write-dac: copy DACL old -> temp", copy_dacl(&target, &temp));
        result("no-write-dac: target still holds", format!("{:?}", fs::read_to_string(&target).map_err(|e| e.raw_os_error())));
    }
}
