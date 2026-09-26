//! Shared by the tests that race two writers in one project on disk.

use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};

use typdoc_core::{Env, Fs};

pub const WF_SCHEMA: &str = r#"{
  "name": "ticket",
  "code": "WF",
  "fields": {
    "title": { "type": "string", "required": true }
  }
}"#;

/// Writes through the real [`Fs`] seam: `clippy.toml`'s write ban covers test code too.
pub fn write_file(path: &Path, text: &str) {
    let fs = typdoc_fs::SystemFs;
    if let Some(parent) = path.parent() {
        fs.create_dir_all(parent).expect("a folder");
    }
    let mut handle = fs.create_new(path).expect("the file does not exist yet");
    handle
        .write_all(text.as_bytes())
        .expect("the write succeeds");
}

/// An [`Env`] that reads nothing from the machine that runs the tests.
pub struct FixedEnv {
    pub cwd: PathBuf,
}

impl Env for FixedEnv {
    fn var(&self, _name: &str) -> Option<OsString> {
        None
    }

    fn current_dir(&self) -> io::Result<PathBuf> {
        Ok(self.cwd.clone())
    }

    fn hostname(&self) -> String {
        "fixed-host".to_owned()
    }
}
