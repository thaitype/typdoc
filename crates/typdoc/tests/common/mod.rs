//! Shared by the CLI tests: the fixtures loader and the one place a process is started.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Written out here and never copied from the machine that runs the suite.
const PATH: &str = "/usr/bin:/bin";

/// A fixture project by its path below `fixtures/`, e.g. `valid/minimal`.
pub fn fixture(relative: &str) -> PathBuf {
    typdoc_testkit::fixtures::path(relative)
}

/// What a finished run left behind.
pub struct Ran {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl Ran {
    pub fn stdout_json(&self) -> serde_json::Value {
        serde_json::from_str(&self.stdout)
            .unwrap_or_else(|e| panic!("stdout is not JSON ({e}): {:?}", self.stdout))
    }

    pub fn stderr_json(&self) -> serde_json::Value {
        serde_json::from_str(&self.stderr)
            .unwrap_or_else(|e| panic!("stderr is not JSON ({e}): {:?}", self.stderr))
    }
}

/// One run of the built binary: an empty environment, a constant `PATH`, a fresh `HOME`,
/// and only the variables a test declares.
pub struct Spawn {
    args: Vec<String>,
    vars: Vec<(String, String)>,
    cwd: Option<PathBuf>,
}

impl Spawn {
    pub fn args<I, S>(args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        Spawn {
            args: args.into_iter().map(|a| a.as_ref().to_owned()).collect(),
            vars: Vec::new(),
            cwd: None,
        }
    }

    pub fn var(mut self, name: &str, value: impl AsRef<OsStr>) -> Self {
        assert!(
            name != "PATH" && name != "HOME",
            "{name} is decided by the helper and a test cannot set it"
        );
        let value = value.as_ref().to_str().expect("a UTF-8 value").to_owned();
        self.vars.push((name.to_owned(), value));
        self
    }

    pub fn cwd(mut self, dir: impl AsRef<Path>) -> Self {
        self.cwd = Some(dir.as_ref().to_owned());
        self
    }

    #[allow(
        clippy::disallowed_methods,
        reason = "the one place a test starts a process, so that the environment it gets is decided here"
    )]
    pub fn run(self) -> Ran {
        let home = tempfile::tempdir().expect("a fresh HOME");
        let mut command = Command::new(env!("CARGO_BIN_EXE_typdoc"));
        command
            .env_clear()
            .env("PATH", PATH)
            .env("HOME", home.path())
            .args(&self.args);
        for (name, value) in &self.vars {
            command.env(name, value);
        }
        if let Some(cwd) = &self.cwd {
            command.current_dir(cwd);
        }
        let output = command.output().expect("the typdoc binary starts");
        Ran {
            code: output.status.code().expect("typdoc ended by a signal"),
            stdout: String::from_utf8(output.stdout).expect("UTF-8 on stdout"),
            stderr: String::from_utf8(output.stderr).expect("UTF-8 on stderr"),
        }
    }
}

/// A project made in a temporary folder, for the cases no committed fixture holds.
pub struct Scratch {
    dir: tempfile::TempDir,
}

impl Scratch {
    /// A project with the smallest config and the files given, each as (path, text).
    pub fn project(files: &[(&str, &str)]) -> Scratch {
        let scratch = Scratch {
            dir: tempfile::tempdir().expect("a scratch folder"),
        };
        scratch.file(".typdoc/config.json", r#"{ "version": 1 }"#);
        for (path, text) in files {
            scratch.file(path, text);
        }
        scratch
    }

    /// A folder with nothing in it, for the cases where there is no project at all.
    pub fn empty() -> Scratch {
        Scratch {
            dir: tempfile::tempdir().expect("a scratch folder"),
        }
    }

    pub fn file(&self, path: &str, text: &str) {
        let file = self.dir.path().join(path);
        std::fs::create_dir_all(file.parent().expect("a parent")).expect("a folder");
        std::fs::write(file, text).expect("a file");
    }

    pub fn symlink(&self, link: &str, target: &str) {
        std::os::unix::fs::symlink(target, self.dir.path().join(link)).expect("a symbolic link");
    }

    /// A file whose path, from the project folder, is given as bytes and may not be valid UTF-8.
    pub fn file_named_by_bytes(&self, name: &[u8], text: &str) {
        use std::os::unix::ffi::OsStrExt;
        let file = self.dir.path().join(OsStr::from_bytes(name));
        std::fs::create_dir_all(file.parent().expect("a parent")).expect("a folder");
        std::fs::write(file, text).expect("a file");
    }

    pub fn path(&self) -> &Path {
        self.dir.path()
    }
}

/// The files of a project with one collection of `*.md` and one schema.
pub const NOTES: [(&str, &str); 2] = [
    (
        ".typdoc/collections/notes.json",
        r#"{ "match": "*.md", "schema": "note.json" }"#,
    ),
    ("note.json", r#"{ "name": "note", "fields": {} }"#),
];
