//! The goal's second criterion, deterministic half (contract, testing decision 2): two
//! allocations, on two threads, racing for the same namespace lock, proved to never issue one
//! key — not because the runs happened not to overlap, but because the second is forced to wait
//! for the first's whole read-decide-write window before it can read `last` at all.
//!
//! `state::read` (`crates/typdoc-core/src/state.rs`) reads with a raw `std::fs::read`, not
//! through the injected [`Fs`]: reading a project deliberately does not go through the seam
//! (`crates/typdoc-core/src/fs.rs`'s own doc comment, "Reading a project does not go through
//! here"). A purely in-memory fake cannot stand in for the project root here, because a write
//! `state::write` makes through it would be invisible to `state::read`'s own real-disk read. So
//! this test runs against a real temporary directory, backed by [`typdoc_fs::SystemFs`], wrapped
//! only enough to pause a thread at the one seam call inside the allocation window
//! (`Project::new_coded`'s own `deps.fs.exists(&file)`, between reading `last` and writing it
//! back) and to notice the instant a second, real, concurrent attempt at the same lock file is
//! turned away.

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

/// A project on real disk with one coded collection (`WF`, `tickets/{key}.md`) and nothing in
/// it yet: the smallest project `new_coded`'s own allocation window can be raced over.
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

/// Wraps the real file system to (a) pause each of the two allocations' `exists` calls, in turn,
/// at the point `Project::new_coded` reaches it inside its allocation window, and (b) notice the
/// moment a `create_new` on the namespace's own lock file is turned away because another thread
/// already holds it — hard evidence that a real, concurrent attempt happened while the first
/// thread was paused, not merely that it was scheduled.
struct RaceFs {
    inner: typdoc_fs::SystemFs,
    lock_path: PathBuf,
    /// One [`Gate`] per `exists` call this test cares about, drawn in order: the first call to
    /// `exists` takes the first, the second call takes the second. Any further call (there is
    /// none in this test's own path) passes straight through.
    gates: Mutex<Vec<Gate>>,
    /// Sends once, the first time a `create_new` on `lock_path` fails because it is already
    /// there: proof that a second attempt at the same lock genuinely met the first one still
    /// holding it, not a fact this test would otherwise have to assume from timing.
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

/// What both threads share to build their own [`Deps`] from: bundled into one type, rather than
/// three references passed and rebuilt separately, the same reason `Deps` itself exists.
/// Concrete types, not `Deps`'s own `&dyn Trait` fields, which is what lets this cross the
/// thread boundary at all — `Deps<'_>` is not `Sync` (its fields carry no such bound), so a
/// value built from it cannot be shared by reference the way this one is; each thread instead
/// builds its own `Deps` from this bundle, on its own side of the boundary.
struct Ctx<'a> {
    fs: &'a RaceFs,
    env: &'a FixedEnv,
    clock: &'a FixedClock,
}

/// Allocates one document through the real seam, on the current thread, returning the key it
/// was given.
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

/// **Contract testing decision 2, "Deterministically, through the seam."** Two allocations, on
/// two threads, against one real project: the first is paused, by the fake, at the instant
/// between reading `last` and writing it back; the second is spawned while the first is still
/// paused there, and is proved — not assumed — to have been turned away by the real lock at
/// least once before the first is let go. Only then is the first released, and the second is
/// tracked to the very same seam a second time before it, too, is released. If the lock did not
/// cover this whole window, both could compute the same `next` from the same stale `last`.
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

    // Two separate `Project` values, both loaded before either allocates, standing for two
    // callers that each read the project once and then raced for the lock — the same shape a
    // real second process would have.
    let ctx = Ctx {
        fs: &fs,
        env: &env,
        clock: &clock,
    };
    let project_a = Project::load(&root, &env).expect("the scratch project loads");
    let project_b = Project::load(&root, &env).expect("the scratch project loads");

    let (key_a, key_b) = std::thread::scope(|scope| {
        let handle_a = scope.spawn(|| allocate(&project_a, &ctx, "First"));

        // Thread A has reached (and is now parked inside) its own `exists` call, the instant
        // between reading `last` and writing it back.
        reached_a_rx.recv().expect("thread A reaches the pause");

        let handle_b = scope.spawn(|| allocate(&project_b, &ctx, "Second"));

        // Real, hard evidence that thread B's own attempt at the same lock file met thread A's
        // still standing there and was refused — not a timing guess: this recv blocks until it
        // happens, and it is guaranteed to happen, because thread A has not been released yet,
        // so its lock file cannot have gone anywhere in the meantime.
        contention_rx
            .recv()
            .expect("thread B's own attempt at the lock is turned away while A still holds it");

        // Now, and only now, let thread A finish: write its state, create its document, and
        // drop its lock.
        release_a_tx.send(()).expect("thread A is still parked");

        // Thread B's own retry succeeds once A's lock is gone; it reads a fresh `last` (A's own
        // write), computes its own `next`, and reaches the very same seam a second time — proof
        // the second allocation really did go through the identical, locked path, not around it.
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
