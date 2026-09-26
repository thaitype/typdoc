use std::ffi::OsString;
use std::io;
use std::path::PathBuf;

use crate::clock::Clock;
use crate::fs::Fs;

/// The process environment, reached only through here.
pub trait Env {
    fn var(&self, name: &str) -> Option<OsString>;
    fn current_dir(&self) -> io::Result<PathBuf>;
    /// The machine's hostname, stamped into a lock file. It only words a timeout message and
    /// never decides whether a lock is valid (SPC-10), so a name that cannot be read is no reason
    /// to refuse a lock, and this cannot fail.
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
