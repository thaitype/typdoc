//! Covers SPC-10.
//!
//! Two allocations racing for one namespace lock never issue one key: the second waits for the
//! first's whole read-decide-write window before it can read `last`, and this is forced, not
//! left to timing.
//!
//! Real disk, not the in-memory fake: `state::read` reads with `std::fs::read`, not through the
//! seam, so a state write made through the fake would be invisible to it.

#[allow(dead_code, reason = "each test file uses part of the shared helper")]
mod common;

use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Duration;

use typdoc_core::{Deps, FileId, Fs, Mode, NewTarget, Project, WriteHandle};
use typdoc_testkit::fake::FixedClock;

use common::{FixedEnv, WF_SCHEMA, write_file};

fn scratch_project() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("a scratch folder");
    write_file(
        &dir.path().join(".typdoc/config.json"),
        r#"{ "version": 1 }"#,
    );
    write_file(
        &dir.path().join(".typdoc/collections/tickets.json"),
        r#"{ "match": "tickets/{key}.md", "schema": "wf.json" }"#,
    );
    write_file(&dir.path().join("wf.json"), WF_SCHEMA);
    dir
}

/// One rendezvous: the call that draws it parks until told to continue, and tells the test it
/// arrived first.
struct Gate {
    reached: Sender<()>,
    release: Receiver<()>,
}

/// The real file system, pausing each allocation at its `exists` call, which
/// `Project::new_coded` makes between reading `last` and writing it, and reporting the first
/// `create_new` of the lock file that is refused: evidence that a second attempt met the first
/// still holding the lock.
struct RaceFs {
    inner: typdoc_fs::SystemFs,
    lock_path: PathBuf,
    /// Drawn in order, one per `exists` call; a further call passes straight through.
    gates: Mutex<Vec<Gate>>,
    /// Sends once, on the first refused `create_new` of `lock_path`.
    contention: Mutex<Option<Sender<()>>>,
}

impl Fs for RaceFs {
    fn create_new(&self, path: &Path) -> io::Result<Box<dyn WriteHandle>> {
        let result = self.inner.create_new(path);
        if path == self.lock_path
            && result.is_err()
            && let Some(tx) = self
                .contention
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .take()
        {
            let _ = tx.send(());
        }
        result
    }

    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        self.inner.rename(from, to)
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
        self.inner.remove_file(path)
    }

    fn create_dir_all(&self, path: &Path) -> io::Result<()> {
        self.inner.create_dir_all(path)
    }

    fn mode(&self, path: &Path) -> io::Result<Mode> {
        self.inner.mode(path)
    }

    fn set_mode(&self, path: &Path, mode: Mode) -> io::Result<()> {
        self.inner.set_mode(path, mode)
    }

    fn exists(&self, path: &Path) -> io::Result<bool> {
        let gate = {
            let mut gates = self
                .gates
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            (!gates.is_empty()).then(|| gates.remove(0))
        };
        if let Some(gate) = gate {
            gate.reached.send(()).expect("the test is still listening");
            gate.release
                .recv()
                .expect("the test still holds the release end");
        }
        self.inner.exists(path)
    }

    fn same_file(&self, a: &Path, b: &Path) -> io::Result<bool> {
        self.inner.same_file(a, b)
    }

    fn identity_at(&self, path: &Path) -> io::Result<Option<FileId>> {
        self.inner.identity_at(path)
    }
}

fn target(title: &str) -> NewTarget {
    NewTarget::Coded {
        code: "WF".to_owned(),
        title: title.to_owned(),
    }
}

/// Concrete types, because [`Deps`] is not `Sync`: each thread builds its own `Deps` from this.
struct Ctx<'a> {
    fs: &'a RaceFs,
    env: &'a FixedEnv,
    clock: &'a FixedClock,
}

fn allocate(project: &Project, ctx: &Ctx, title: &str) -> String {
    let deps = Deps {
        env: ctx.env,
        fs: ctx.fs,
        clock: ctx.clock,
    };
    let document = project
        .new_document(&target(title), &deps, &[], Duration::from_secs(30), None)
        .unwrap_or_else(|e| panic!("allocating {title:?} failed: {e}"));
    document
        .key
        .expect("a coded allocation always returns a key")
}

/// If the lock did not cover the whole window, both threads could compute the same next number
/// from the same `last`.
#[test]
fn two_threads_racing_the_same_lock_never_issue_the_same_key() {
    let scratch = scratch_project();
    let root = scratch.path().to_path_buf();
    let env = FixedEnv { cwd: root.clone() };
    let clock = FixedClock::new();

    let lock_path = typdoc_core::local_namespace_lock_path(&root, "default");
    let (reached_a_tx, reached_a_rx) = mpsc::channel();
    let (release_a_tx, release_a_rx) = mpsc::channel();
    let (reached_b_tx, reached_b_rx) = mpsc::channel();
    let (release_b_tx, release_b_rx) = mpsc::channel();
    let (contention_tx, contention_rx) = mpsc::channel();
    let fs = RaceFs {
        inner: typdoc_fs::SystemFs,
        lock_path: lock_path.clone(),
        gates: Mutex::new(vec![
            Gate {
                reached: reached_a_tx,
                release: release_a_rx,
            },
            Gate {
                reached: reached_b_tx,
                release: release_b_rx,
            },
        ]),
        contention: Mutex::new(Some(contention_tx)),
    };

    // Two `Project`s, both loaded before either allocates, as two processes would be.
    let ctx = Ctx {
        fs: &fs,
        env: &env,
        clock: &clock,
    };
    let project_a = Project::load(&root, &env).expect("the scratch project loads");
    let project_b = Project::load(&root, &env).expect("the scratch project loads");

    let (key_a, key_b) = std::thread::scope(|scope| {
        let handle_a = scope.spawn(|| allocate(&project_a, &ctx, "First"));

        // A is parked between reading `last` and writing it.
        reached_a_rx.recv().expect("thread A reaches the pause");

        let handle_b = scope.spawn(|| allocate(&project_b, &ctx, "Second"));

        // Blocks until B's attempt at the lock is refused, which must happen while A is parked.
        contention_rx
            .recv()
            .expect("thread B's own attempt at the lock is turned away while A still holds it");

        // Only now does A finish and drop its lock.
        release_a_tx.send(()).expect("thread A is still parked");

        // B reaches the same pause, so it too went through the locked path, reading A's `last`.
        reached_b_rx
            .recv()
            .expect("thread B reaches the same pause");
        release_b_tx.send(()).expect("thread B is still parked");

        let key_a = handle_a.join().expect("thread A does not panic");
        let key_b = handle_b.join().expect("thread B does not panic");
        (key_a, key_b)
    });

    assert_ne!(
        key_a, key_b,
        "two allocations racing the same lock must never issue the same key"
    );
    let keys = {
        let mut k = [key_a.clone(), key_b.clone()];
        k.sort();
        k
    };
    assert_eq!(keys, ["WF-1", "WF-2"], "the two keys allocated, in order");

    let state_text =
        std::fs::read_to_string(root.join(".typdoc/state/default.json")).expect("the state file");
    let state: serde_json::Value = serde_json::from_str(&state_text).expect("valid JSON");
    assert_eq!(
        state["tickets"]["last"],
        serde_json::json!(2),
        "`last` is the higher of the two keys, not whichever writer happened to write it second"
    );

    assert!(root.join("tickets/WF-1.md").is_file());
    assert!(root.join("tickets/WF-2.md").is_file());
    let text_1 = std::fs::read_to_string(root.join("tickets/WF-1.md")).expect("WF-1.md");
    let text_2 = std::fs::read_to_string(root.join("tickets/WF-2.md")).expect("WF-2.md");
    assert_ne!(
        text_1, text_2,
        "each document keeps its own writer's title; neither was overwritten by the other"
    );
}
