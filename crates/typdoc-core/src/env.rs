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
    /// `O_EXCL`, holding pid, hostname and timestamp"). Nothing in `typdoc-core` reads it any
    /// other way; the first write command to take a lock is the first caller.
    fn hostname(&self) -> io::Result<String>;
}

/// What a command reaches the outside world through.
pub struct Deps<'a> {
    pub env: &'a dyn Env,
    /// The file operations a write is built from. Reading a project does not go through it.
    pub fs: &'a dyn Fs,
    /// The time a write stamps into an `auto` field.
    pub clock: &'a dyn Clock,
}
