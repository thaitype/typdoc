//! Shared by the CLI tests: the fixtures loader and the one place a process is started.

use std::ffi::OsStr;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

/// Written out here and never copied from the machine that runs the suite.
const PATH: &str = "/usr/bin:/bin";

/// A fixture project by its path below `fixtures/`, e.g. `valid/minimal`.
pub fn fixture(relative: &str) -> PathBuf {
    typdoc_testkit::fixtures::path(relative)
}

/// Loads the fixture for `rule` in `dir`, stages it (a write runs on a copy; a read runs in
/// `dir` unchanged), and spawns its declared command with the environment it declares. The spec
/// is returned alongside the run, since a caller checking the rules tripped needs the declared
/// `trips` too.
pub fn spawn_fixture(
    dir: &Path,
    rule: &str,
) -> Result<(typdoc_testkit::spec::FixtureSpec, Ran), String> {
    let spec = typdoc_testkit::spec::FixtureSpec::load(dir, rule)?;
    let staged = typdoc_testkit::staging::stage(dir, &spec)?;
    let mut spawn = Spawn::args(&spec.command).cwd(staged.dir());
    for (name, value) in &spec.env {
        spawn = spawn.var(name, value);
    }
    let ran = spawn.run();
    Ok((spec, ran))
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

    /// The one place the CLI tests' helper calls `Command::new`; `clippy.toml` allows it only
    /// here and in the shell examples harness.
    #[allow(
        clippy::disallowed_methods,
        reason = "the one place a test starts a process, so that the environment it gets is decided here"
    )]
    fn command(&self, home: &Path) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_typdoc"));
        command
            .env_clear()
            .env("PATH", PATH)
            .env("HOME", home)
            .args(&self.args);
        for (name, value) in &self.vars {
            command.env(name, value);
        }
        if let Some(cwd) = &self.cwd {
            command.current_dir(cwd);
        }
        command
    }

    pub fn run(self) -> Ran {
        let home = tempfile::tempdir().expect("a fresh HOME");
        let mut command = self.command(home.path());
        let output = command.output().expect("the typdoc binary starts");
        Ran {
            code: output.status.code().expect("typdoc ended by a signal"),
            stdout: String::from_utf8(output.stdout).expect("UTF-8 on stdout"),
            stderr: String::from_utf8(output.stderr).expect("UTF-8 on stderr"),
        }
    }

    /// Starts the process without waiting, for a test that acts on it while it runs.
    pub fn spawn(self) -> RunningChild {
        let home = tempfile::tempdir().expect("a fresh HOME");
        let mut command = self.command(home.path());
        command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let child = command.spawn().expect("the typdoc binary starts");
        RunningChild { child, _home: home }
    }
}

/// A process started through [`Spawn::spawn`], while it is still running (or has just ended,
/// for a caller that raced it and lost).
pub struct RunningChild {
    child: Child,
    // Kept alive so `HOME` is not removed out from under a process still running against it.
    _home: tempfile::TempDir,
}

impl RunningChild {
    /// Sends `signal` (a POSIX signal number — `libc::SIGINT`, `libc::SIGTERM`) to this
    /// process, through `kill(2)`, the one real way to deliver anything past `SIGKILL`: the
    /// standard library's own [`Child::kill`] reaches no further than that.
    pub fn signal(&self, signal: libc::c_int) {
        // SAFETY: `kill(2)` takes a pid and a signal number and has no memory of its own to
        // corrupt; the pid is this value's own child, read from the `Child` the standard
        // library already gave it.
        let sent = unsafe { libc::kill(self.child.id() as libc::pid_t, signal) };
        assert_eq!(sent, 0, "kill(2) failed: {}", io::Error::last_os_error());
    }

    pub fn pid(&self) -> u32 {
        self.child.id()
    }

    /// Checked without blocking (`Child::try_wait`); a caller still calls
    /// [`wait`](RunningChild::wait) for how it ended.
    pub fn is_alive(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }

    pub fn wait(self) -> Ended {
        let output = self
            .child
            .wait_with_output()
            .expect("the process can be waited on");
        Ended {
            code: output.status.code(),
            signal: std::os::unix::process::ExitStatusExt::signal(&output.status),
            stdout: String::from_utf8(output.stdout).expect("UTF-8 on stdout"),
            stderr: String::from_utf8(output.stderr).expect("UTF-8 on stderr"),
        }
    }
}

/// `code` is `None` exactly when the process ended by a signal, which is
/// [`std::process::ExitStatus`]'s own rule on POSIX.
pub struct Ended {
    pub code: Option<i32>,
    pub signal: Option<i32>,
    pub stdout: String,
    pub stderr: String,
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

    pub fn read(&self, path: &str) -> String {
        std::fs::read_to_string(self.dir.path().join(path))
            .unwrap_or_else(|e| panic!("{path}: {e}"))
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

/// The coded schema `WF`, for the tests that hold a namespace lock through [`large_project`].
pub const WF_SCHEMA: &str = r#"{
  "name": "ticket",
  "code": "WF",
  "fields": {
    "title": { "type": "string", "required": true },
    "status": { "type": "enum", "values": ["open", "claimed"], "default": "open" },
    "kind": { "type": "enum", "values": ["research", "task"], "required": true }
  }
}"#;

/// The files of a project with one coded collection, `tickets/{key}.md`, matching
/// [`WF_SCHEMA`].
pub const WF_COLLECTION: [(&str, &str); 2] = [
    (
        ".typdoc/collections/tickets.json",
        r#"{ "match": "tickets/{key}.md", "schema": "wf.json" }"#,
    ),
    ("wf.json", WF_SCHEMA),
];

/// The frontmatter every filler document of [`large_project`] carries: `WF_SCHEMA` requires no
/// more than this to be valid.
pub const FILLER: &str = "---\ntitle: Filler\nstatus: open\nkind: research\n---\n";

/// A project with `document_count` documents in the `WF` collection and its state file caught up
/// to them, built for the test since a fixture this size does not belong in the repository. A
/// write reads every document already there from disk under the lock (`Project::prescan_refs`,
/// for `refs.acyclic`), so enough documents hold the lock long enough to be observed, signalled or
/// contended, with no code in the binary for it.
pub fn large_project(document_count: u32) -> Scratch {
    let state_text = format!("{{ \"tickets\": {{ \"last\": {document_count} }} }}");
    let mut files: Vec<(&str, &str)> = WF_COLLECTION.to_vec();
    files.push((".typdoc/state/default.json", state_text.as_str()));
    let project = Scratch::project(&files);
    for n in 1..=document_count {
        project.file(&format!("tickets/WF-{n}.md"), FILLER);
    }
    project
}

/// `.typdoc/locks/default.lock`, the lock every write against [`large_project`]'s single
/// namespace takes.
pub fn lock_path(project: &Path) -> PathBuf {
    project.join(".typdoc/locks/default.lock")
}

/// Polls for `path` to exist, up to `timeout`; the last check's own answer is the return value,
/// so a caller that gets `false` back knows the wait, not a stale read, is what failed.
pub fn wait_for_file(path: &Path, timeout: Duration) -> bool {
    let start = Instant::now();
    loop {
        if path.is_file() {
            return true;
        }
        if start.elapsed() >= timeout {
            return path.is_file();
        }
        std::thread::sleep(Duration::from_millis(2));
    }
}

/// How long a test waits for the lock file to appear before giving up: generous next to how
/// quickly it actually shows up (`acquire` creates it before any of the holding command's own
/// work runs), so this is headroom for a loaded machine, not the ordinary case.
pub const LOCK_APPEARS_WITHIN: Duration = Duration::from_secs(20);
