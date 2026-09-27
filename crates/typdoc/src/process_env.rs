//! The environment the shipped binary runs in.

use std::ffi::OsString;
use std::io;
use std::path::PathBuf;

use typdoc_core::{Env, ProcessStatus};

/// The placeholder `Env::hostname` promises when the name cannot be read.
const UNKNOWN_HOST: &str = "unknown-host";

pub struct ProcessEnv;

impl Env for ProcessEnv {
    fn var(&self, name: &str) -> Option<OsString> {
        std::env::var_os(name)
    }

    fn current_dir(&self) -> io::Result<PathBuf> {
        std::env::current_dir()
    }

    fn hostname(&self) -> String {
        hostname().unwrap_or_else(|| UNKNOWN_HOST.to_owned())
    }

    fn process_status(&self, pid: u32) -> ProcessStatus {
        process_status(pid)
    }
}

#[cfg(unix)]
fn hostname() -> Option<String> {
    // Linux names are at most 255 bytes and macOS names 256; one more leaves room for the NUL,
    // and a buffer the call filled without one is a name cut short, not a name.
    let mut buffer = [0u8; 257];
    // SAFETY: the pointer and length describe `buffer`, which outlives the call.
    let result = unsafe { libc::gethostname(buffer.as_mut_ptr().cast(), buffer.len()) };
    if result != 0 {
        return None;
    }
    let end = buffer.iter().position(|&byte| byte == 0)?;
    let name = std::str::from_utf8(&buffer[..end]).ok()?.trim();
    (!name.is_empty()).then(|| name.to_owned())
}

#[cfg(not(unix))]
fn hostname() -> Option<String> {
    None
}

/// `kill(pid, 0)` sends nothing and only asks. `EPERM` is a process that exists and belongs to
/// someone else; only `ESRCH` is "no such process".
#[cfg(unix)]
fn process_status(pid: u32) -> ProcessStatus {
    // `0` would ask about this process's own group and a value past `i32::MAX` turns negative as
    // a `pid_t`, which asks about every process the caller may signal: both answer "running".
    let Ok(pid) = libc::pid_t::try_from(pid) else {
        return ProcessStatus::Unknown;
    };
    if pid == 0 {
        return ProcessStatus::Unknown;
    }
    // SAFETY: signal 0 sends nothing; the call only checks that `pid` could be signalled.
    if unsafe { libc::kill(pid, 0) } == 0 {
        return ProcessStatus::Running;
    }
    match io::Error::last_os_error().raw_os_error() {
        Some(libc::EPERM) => ProcessStatus::Running,
        Some(libc::ESRCH) => ProcessStatus::NotRunning,
        _ => ProcessStatus::Unknown,
    }
}

#[cfg(not(unix))]
fn process_status(_pid: u32) -> ProcessStatus {
    ProcessStatus::Unknown
}
