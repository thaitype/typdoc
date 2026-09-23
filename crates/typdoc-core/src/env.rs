use std::ffi::OsString;
use std::io;
use std::path::PathBuf;

use crate::clock::Clock;
use crate::fs::Fs;

/// The process environment, reached only through here.
pub trait Env {
    fn var(&self, name: &str) -> Option<OsString>;
    fn current_dir(&self) -> io::Result<PathBuf>;
    /// The machine's own hostname, stamped into a lock file so a competing lock's message can
    /// say whether it is on this host or another one (design, Concurrency: "A file created with
    /// `O_EXCL`, holding pid, hostname and timestamp"). A name that cannot be read cleanly is not
    /// a reason to refuse taking the lock: the hostname only ever chooses the wording of a
    /// timeout message (design: "The pid check only chooses the wording; it never decides
    /// whether a lock is valid" — read the same way for the host), so this reads infallibly, the
    /// same shape `pid_alive` already uses for the same reason.
    fn hostname(&self) -> String;
}

/// What a command reaches the outside world through.
pub struct Deps<'a> {
    pub env: &'a dyn Env,
    /// The file operations a write is built from. Reading a project does not go through it.
    pub fs: &'a dyn Fs,
    /// The time a write stamps into an `auto` field.
    pub clock: &'a dyn Clock,
}
