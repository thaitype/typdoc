//! Prints what Windows does in the cases typdoc's writes depend on, one `RESULT` line each.

#[cfg(not(windows))]
fn main() {
    println!("RESULT platform: not windows, nothing measured");
}

#[cfg(windows)]
fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("child-handler") => win::child(true, &args[2]),
        Some("child-default") => win::child(false, &args[2]),
        _ => win::run(),
    }
}

#[cfg(windows)]
mod win {
    use std::ffi::c_void;
    use std::fs::{self, File, OpenOptions};
    use std::io::Write;
    use std::os::windows::fs::OpenOptionsExt;
    use std::os::windows::io::AsRawHandle;
    use std::os::windows::process::CommandExt;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::time::{Duration, Instant};

    use windows_sys::Win32::Foundation::{HANDLE, STATUS_CONTROL_C_EXIT};
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_ID_INFO, FILE_SHARE_READ, FILE_SHARE_WRITE, FILE_STANDARD_INFO, FileIdInfo,
        FileStandardInfo, GetFileInformationByHandleEx,
    };
    use windows_sys::Win32::System::Console::{
        AllocConsole, CTRL_BREAK_EVENT, GenerateConsoleCtrlEvent, SetConsoleCtrlHandler,
    };
    use windows_sys::Win32::System::Threading::ExitProcess;

    fn result(key: &str, value: impl std::fmt::Display) {
        println!("RESULT {key}: {value}");
    }

    fn io(r: std::io::Result<impl std::fmt::Debug>) -> String {
        match r {
            Ok(v) => format!("ok {v:?}"),
            Err(e) => format!("err kind={:?} os={:?} ({e})", e.kind(), e.raw_os_error()),
        }
    }

    fn id(h: HANDLE) -> String {
        let mut info: FILE_ID_INFO = unsafe { std::mem::zeroed() };
        let ok = unsafe {
            GetFileInformationByHandleEx(h, FileIdInfo, (&raw mut info).cast::<c_void>(), size_of::<FILE_ID_INFO>() as u32)
        };
        if ok == 0 {
            return format!("err {}", std::io::Error::last_os_error());
        }
        let hex: String = info.FileId.Identifier.iter().rev().map(|b| format!("{b:02x}")).collect();
        format!("volume={:016x} id={hex}", info.VolumeSerialNumber)
    }

    fn standard(h: HANDLE) -> String {
        let mut info: FILE_STANDARD_INFO = unsafe { std::mem::zeroed() };
        let ok = unsafe {
            GetFileInformationByHandleEx(h, FileStandardInfo, (&raw mut info).cast::<c_void>(), size_of::<FILE_STANDARD_INFO>() as u32)
        };
        if ok == 0 {
            return format!("err {}", std::io::Error::last_os_error());
        }
        format!("links={} delete_pending={}", info.NumberOfLinks, info.DeletePending)
    }

    fn path_id(p: &Path) -> String {
        // Access 0, share everything, backup semantics: how std's `fs::metadata` opens a path.
        match OpenOptions::new().access_mode(0).custom_flags(0x0200_0000).open(p) {
            Ok(f) => id(f.as_raw_handle() as HANDLE),
            Err(e) => format!("open err kind={:?} os={:?}", e.kind(), e.raw_os_error()),
        }
    }

    fn scratch() -> PathBuf {
        let d = std::env::temp_dir().join(format!("probe-{}", std::process::id()));
        fs::create_dir_all(&d).unwrap();
        d
    }

    pub fn run() {
        let d = scratch();
        result("temp dir", d.display());
        lock_deleted_while_held(&d);
        rename_cases(&d);
        ctrl_break(&d);
    }

    fn lock_deleted_while_held(d: &Path) {
        let lock = d.join("default.lock");
        let mut held = File::create_new(&lock).unwrap();
        held.write_all(b"{}").unwrap();
        let h = held.as_raw_handle() as HANDLE;
        result("lock held: handle id", id(h));
        result("lock held: path id", path_id(&lock));
        result("lock held: handle standard", standard(h));
        result("lock deleted by std remove_file", io(fs::remove_file(&lock)));
        result("after delete: handle standard", standard(h));
        result("after delete: handle id", id(h));
        result("after delete: path metadata", io(fs::metadata(&lock).map(|m| m.len())));
        result("after delete: path id", path_id(&lock));
        let second = File::create_new(&lock);
        let second_ok = second.is_ok();
        result("after delete: create_new at path", match &second { Ok(_) => "ok".to_owned(), Err(e) => format!("err kind={:?} os={:?} ({e})", e.kind(), e.raw_os_error()) });
        if let Ok(s) = &second {
            result("after delete: new file id", id(s.as_raw_handle() as HANDLE));
        }
        drop(second);
        drop(held);
        result("after close: path metadata", io(fs::metadata(&lock).map(|m| m.len())));
        if !second_ok {
            result("after close: create_new at path", io(File::create_new(&lock).map(|_| ())));
        }
        let _ = fs::remove_file(&lock);
    }

    fn rename_cases(d: &Path) {
        // Over a read-only target.
        let target = d.join("ro.md");
        fs::write(&target, "old").unwrap();
        let mut p = fs::metadata(&target).unwrap().permissions();
        p.set_readonly(true);
        fs::set_permissions(&target, p).unwrap();
        let temp = d.join(".tmp-ro");
        fs::write(&temp, "new").unwrap();
        result("rename over read-only target", io(fs::rename(&temp, &target)));
        result("read-only target now holds", io(fs::read_to_string(&target)));

        // Over a target another handle holds open, without share-delete.
        let target = d.join("open-no-share-delete.md");
        fs::write(&target, "old").unwrap();
        let reader = OpenOptions::new().read(true).share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE).open(&target).unwrap();
        let temp = d.join(".tmp-nsd");
        fs::write(&temp, "new").unwrap();
        result("rename over target open without share-delete", io(fs::rename(&temp, &target)));
        result("target (no share-delete) now holds", io(fs::read_to_string(&target)));
        drop(reader);

        // Over a target another handle holds open with std's default sharing (share-delete).
        let target = d.join("open-share-delete.md");
        fs::write(&target, "old").unwrap();
        let reader = File::open(&target).unwrap();
        let temp = d.join(".tmp-sd");
        fs::write(&temp, "new").unwrap();
        result("rename over target open with share-delete", io(fs::rename(&temp, &target)));
        result("target (share-delete) now holds", io(fs::read_to_string(&target)));
        let mut old = String::new();
        result("reader still reads", io(std::io::Read::read_to_string(&mut &reader, &mut old).map(|_| old.clone())));
        drop(reader);

        // Plain rename over a closed target: identity of the result.
        let target = d.join("plain.md");
        fs::write(&target, "old").unwrap();
        let before = path_id(&target);
        let temp = d.join(".tmp-plain");
        fs::write(&temp, "new").unwrap();
        let temp_id = path_id(&temp);
        result("plain rename", io(fs::rename(&temp, &target)));
        result("plain rename: target id before / temp id / after", format!("{before} / {temp_id} / {}", path_id(&target)));
    }

    unsafe extern "system" fn handler(_ctrl: u32) -> windows_sys::core::BOOL {
        let marker = std::env::var("PROBE_MARKER").unwrap();
        let _ = fs::write(marker, "cleaned");
        unsafe { ExitProcess(STATUS_CONTROL_C_EXIT as u32) };
    }

    pub fn child(with_handler: bool, ready: &str) {
        if with_handler {
            unsafe { SetConsoleCtrlHandler(Some(handler), 1) };
        }
        fs::write(ready, "ready").unwrap();
        std::thread::sleep(Duration::from_secs(30));
        println!("child was not interrupted");
    }

    fn ctrl_break(d: &Path) {
        let alloc = unsafe { AllocConsole() };
        result("AllocConsole (0 = already had one or failed)", format!("{alloc} {}", if alloc == 0 { std::io::Error::last_os_error().to_string() } else { String::new() }));
        let me = std::env::current_exe().unwrap();
        for mode in ["child-handler", "child-default"] {
            let ready = d.join(format!("{mode}.ready"));
            let marker = d.join(format!("{mode}.marker"));
            let mut child = Command::new(&me)
                .args([mode, ready.to_str().unwrap()])
                .env("PROBE_MARKER", &marker)
                .creation_flags(0x0000_0200)
                .spawn()
                .unwrap();
            let start = Instant::now();
            while !ready.exists() && start.elapsed() < Duration::from_secs(10) {
                std::thread::sleep(Duration::from_millis(50));
            }
            let sent = unsafe { GenerateConsoleCtrlEvent(CTRL_BREAK_EVENT, child.id()) };
            result(&format!("{mode}: GenerateConsoleCtrlEvent"), format!("{sent} {}", if sent == 0 { std::io::Error::last_os_error().to_string() } else { String::new() }));
            let start = Instant::now();
            let status = loop {
                if let Some(s) = child.try_wait().unwrap() {
                    break Some(s);
                }
                if start.elapsed() > Duration::from_secs(15) {
                    let _ = child.kill();
                    break None;
                }
                std::thread::sleep(Duration::from_millis(50));
            };
            result(&format!("{mode}: exit"), format!("{status:?} code={:?} (STATUS_CONTROL_C_EXIT as i32 = {})", status.and_then(|s| s.code()), STATUS_CONTROL_C_EXIT));
            result(&format!("{mode}: cleanup marker written"), marker.exists());
        }
    }
}
