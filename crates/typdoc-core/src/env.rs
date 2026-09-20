use std::ffi::OsString;
use std::io;
use std::path::PathBuf;

/// The process environment, reached only through here.
pub trait Env {
    fn var(&self, name: &str) -> Option<OsString>;
    fn current_dir(&self) -> io::Result<PathBuf>;
}

/// What a command reaches the outside world through.
pub struct Deps<'a> {
    pub env: &'a dyn Env,
}
