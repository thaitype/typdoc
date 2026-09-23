//! Shared by ticket 14's own tests: the smallest coded schema they race allocations against, a
//! real-disk write helper that stays on the [`Fs`] seam, and a fixed [`Env`] — so the write ban
//! and the scratch-project shape are written once, not once per test file.

use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};

use typdoc_core::{Env, Fs};

/// The schema a coded allocation is raced against: minimal, one code, one required field.
pub const WF_SCHEMA: &str = r#"{
  "name": "ticket",
  "code": "WF",
  "fields": {
    "title": { "type": "string", "required": true }
  }
}"#;

/// Writes `text` to `path` through the real [`Fs`] seam (`typdoc_fs::SystemFs`), not through a
/// raw `std::fs` call: `crates/typdoc-core/clippy.toml`'s write ban reaches this crate's test
/// code too, on purpose ("a test that reaches around the seam is exactly the test that stops
/// proving anything about the code that ships"), so even a scratch project's own setup goes
/// through it.
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

/// An [`Env`] with a fixed current directory and hostname and no environment variables: what
/// these tests need to load and race a project, with nothing read from the machine that runs
/// them.
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
        "ticket-14-test".to_owned()
    }
}
