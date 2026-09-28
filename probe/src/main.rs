//! Prints what a POSIX-semantics rename (`FileRenameInfoEx`) does on Windows, one `RESULT` line each.

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
    use std::fs::{self, File, OpenOptions};
    use std::os::windows::ffi::OsStrExt;
    use std::os::windows::fs::OpenOptionsExt;
    use std::os::windows::io::AsRawHandle;
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

    use windows_sys::Win32::Foundation::HANDLE;
    use windows_sys::Win32::Storage::FileSystem::{
        DELETE, FILE_ATTRIBUTE_READONLY, FILE_RENAME_INFO, FILE_SHARE_READ, FILE_SHARE_WRITE,
        FileRenameInfoEx, GetFileAttributesW, SetFileInformationByHandle,
    };

    // FILE_RENAME_FLAG_REPLACE_IF_EXISTS, FILE_RENAME_FLAG_POSIX_SEMANTICS,
    // FILE_RENAME_FLAG_IGNORE_READONLY_ATTRIBUTE (winbase.h / ntifs.h).
    const REPLACE_IF_EXISTS: u32 = 0x1;
    const POSIX_SEMANTICS: u32 = 0x2;
    const IGNORE_READONLY_ATTRIBUTE: u32 = 0x40;

    fn result(key: &str, value: impl std::fmt::Display) {
        println!("RESULT {key}: {value}");
    }

    fn io<T: std::fmt::Debug>(r: &std::io::Result<T>) -> String {
        match r {
            Ok(v) => format!("ok {v:?}"),
            Err(e) => format!("err kind={:?} os={:?} ({e})", e.kind(), e.raw_os_error()),
        }
    }

    fn wide(p: &Path) -> Vec<u16> {
        p.as_os_str().encode_wide().chain(Some(0)).collect()
    }

    fn readonly(p: &Path) -> String {
        let a = unsafe { GetFileAttributesW(wide(p).as_ptr()) };
        if a == u32::MAX {
            return format!("attributes err {}", std::io::Error::last_os_error());
        }
        format!("readonly={}", a & FILE_ATTRIBUTE_READONLY != 0)
    }

    /// Renames `from` over `to` through its own handle, with `flags`.
    fn posix_rename(from: &Path, to: &Path, flags: u32) -> std::io::Result<()> {
        let source = OpenOptions::new()
            .access_mode(DELETE)
            .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | 0x4)
            .open(from)?;
        let name: Vec<u16> = to.as_os_str().encode_wide().collect();
        let header = std::mem::offset_of!(FILE_RENAME_INFO, FileName);
        let size = header + name.len() * 2 + 2;
        let mut buffer = vec![0u64; size.div_ceil(8)];
        let info = buffer.as_mut_ptr().cast::<FILE_RENAME_INFO>();
        unsafe {
            (*info).Anonymous.Flags = flags;
            (*info).RootDirectory = std::ptr::null_mut();
            (*info).FileNameLength = (name.len() * 2) as u32;
            let dst = (&raw mut (*info).FileName).cast::<u16>();
            std::ptr::copy_nonoverlapping(name.as_ptr(), dst, name.len());
        }
        let ok = unsafe {
            SetFileInformationByHandle(source.as_raw_handle() as HANDLE, FileRenameInfoEx, info.cast::<c_void>(), size as u32)
        };
        if ok == 0 { Err(std::io::Error::last_os_error()) } else { Ok(()) }
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

    const ALL: u32 = REPLACE_IF_EXISTS | POSIX_SEMANTICS | IGNORE_READONLY_ATTRIBUTE;

    pub fn run() {
        let d = scratch();

        // 1. Over a read-only target, the temp not read-only.
        let target = d.join("ro-target.md");
        fs::write(&target, "old").unwrap();
        set_readonly(&target, true);
        let temp = d.join(".tmp-1");
        fs::write(&temp, "new").unwrap();
        let r = posix_rename(&temp, &target, ALL);
        result("1 read-only target, plain temp: rename", io(&r));
        result("1 target holds", io(&fs::read_to_string(&target)));
        result("1 target attributes", readonly(&target));

        // 2. Over a read-only target, the temp made read-only first (the mode carried before the rename).
        let target = d.join("ro-carried.md");
        fs::write(&target, "old").unwrap();
        set_readonly(&target, true);
        let temp = d.join(".tmp-2");
        fs::write(&temp, "new").unwrap();
        set_readonly(&temp, true);
        let r = posix_rename(&temp, &target, ALL);
        result("2 read-only target, read-only temp: rename", io(&r));
        result("2 target holds", io(&fs::read_to_string(&target)));
        result("2 target attributes", readonly(&target));
        result("2 temp still there", temp.exists());

        // 3. Without IGNORE_READONLY_ATTRIBUTE, for contrast.
        let target = d.join("ro-noflag.md");
        fs::write(&target, "old").unwrap();
        set_readonly(&target, true);
        let temp = d.join(".tmp-3");
        fs::write(&temp, "new").unwrap();
        result("3 read-only target without the ignore flag", io(&posix_rename(&temp, &target, REPLACE_IF_EXISTS | POSIX_SEMANTICS)));

        // 4. Over a target another handle holds open without share-delete.
        let target = d.join("open-nsd.md");
        fs::write(&target, "old").unwrap();
        let reader = OpenOptions::new().read(true).share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE).open(&target).unwrap();
        let temp = d.join(".tmp-4");
        fs::write(&temp, "new").unwrap();
        let r = posix_rename(&temp, &target, ALL);
        result("4 target open without share-delete: rename", io(&r));
        result("4 target holds", io(&fs::read_to_string(&target)));
        drop(reader);

        // 5. Over a target another handle holds open with share-delete.
        let target = d.join("open-sd.md");
        fs::write(&target, "old").unwrap();
        let reader = File::open(&target).unwrap();
        let temp = d.join(".tmp-5");
        fs::write(&temp, "new").unwrap();
        result("5 target open with share-delete: rename", io(&posix_rename(&temp, &target, ALL)));
        result("5 target holds", io(&fs::read_to_string(&target)));
        drop(reader);

        // 6. Atomicity: a reader opens the target in a loop while it is replaced many times.
        for (label, use_posix) in [("posix rename", true), ("std::fs::rename", false)] {
            let target = d.join(format!("atomic-{use_posix}.md"));
            fs::write(&target, "0").unwrap();
            let stop = Arc::new(AtomicBool::new(false));
            let missing = Arc::new(AtomicU64::new(0));
            let other = Arc::new(AtomicU64::new(0));
            let opens = Arc::new(AtomicU64::new(0));
            let reader = {
                let (t, stop, missing, other, opens) = (target.clone(), stop.clone(), missing.clone(), other.clone(), opens.clone());
                std::thread::spawn(move || {
                    while !stop.load(Ordering::Relaxed) {
                        match File::open(&t) {
                            Ok(_) => { opens.fetch_add(1, Ordering::Relaxed); }
                            Err(e) if e.kind() == std::io::ErrorKind::NotFound => { missing.fetch_add(1, Ordering::Relaxed); }
                            Err(_) => { other.fetch_add(1, Ordering::Relaxed); }
                        }
                    }
                })
            };
            let mut failures = 0u32;
            let mut first_failure = String::new();
            for i in 0..2000 {
                let temp = d.join(format!(".tmp-a-{use_posix}-{i}"));
                fs::write(&temp, i.to_string()).unwrap();
                let r = if use_posix { posix_rename(&temp, &target, ALL) } else { fs::rename(&temp, &target) };
                if let Err(e) = r {
                    failures += 1;
                    if first_failure.is_empty() { first_failure = format!("{:?} os={:?}", e.kind(), e.raw_os_error()); }
                    let _ = fs::remove_file(&temp);
                }
            }
            stop.store(true, Ordering::Relaxed);
            reader.join().unwrap();
            result(&format!("6 {label}: 2000 replacements"), format!(
                "rename failures={failures} {first_failure} | reader opens={} not-found={} other-errors={}",
                opens.load(Ordering::Relaxed), missing.load(Ordering::Relaxed), other.load(Ordering::Relaxed)
            ));
        }
    }
}
