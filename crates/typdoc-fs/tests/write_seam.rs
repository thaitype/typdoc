//! One table of scenarios for the write seam, run twice: against the fake, and against a real
//! temporary directory.
//!
//! The fake stages what a real directory will not — no space, a permission refused, a rename
//! across devices, and a run that stops between two operations — and the real directory holds
//! the fake to what a file system does rather than to what it was written to do. A scenario
//! that stages nothing is one a real directory reaches, and the table is the record of which
//! those are.

use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

use tempfile::TempDir;
use typdoc_core::{
    Fs, Mode, NamespaceLock, acquire, create_exclusively, is_temp_name, write_atomically,
};
use typdoc_fs::SystemFs;
use typdoc_testkit::fake::{Failure, FakeFs, FixedClock, On, Stage};

/// An unusual mode, which no umask hands out by itself, so that a mode found on a file after a
/// write was carried there rather than defaulted there.
const UNUSUAL: Mode = 0o100_400;

/// What a scenario looks at afterwards, without going back through the seam it is testing.
trait Disk {
    fn bytes(&self, path: &Path) -> Option<Vec<u8>>;
    fn mode(&self, path: &Path) -> Option<Mode>;
    fn names_in(&self, directory: &Path) -> Vec<String>;
}

/// The file system a scenario runs against, the directory it runs in, and a lock held for the
/// whole of it: `write_atomically` requires one (decision 6), and the scenarios in this table
/// are about what the write seam does to `w.root`, not about locking, so one lock is acquired
/// once, outside `w.root`, and reused for every write the table makes.
struct World<'a> {
    fs: &'a dyn Fs,
    disk: &'a dyn Disk,
    root: PathBuf,
    lock: NamespaceLock<'a>,
}

/// Acquires a lock at `lock_path`, which must sit outside every directory a scenario looks at
/// with [`World::leftovers`] or [`Disk::names_in`], since the lock file is not one of the names
/// a scenario expects to find there.
fn a_lock<'a>(fs: &'a dyn Fs, lock_path: &Path) -> NamespaceLock<'a> {
    acquire(
        fs,
        &FixedClock::new(),
        lock_path.to_path_buf(),
        "write-seam-test-host",
        Duration::from_secs(5),
    )
    .unwrap_or_else(|e| panic!("the scenario's own lock could not be acquired: {e}"))
}

impl World<'_> {
    fn path(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }

    fn bytes(&self, name: &str) -> Option<Vec<u8>> {
        self.disk.bytes(&self.path(name))
    }

    /// Puts a file there through the seam, which is how a scenario sets up on either backend.
    fn given(&self, name: &str, bytes: &[u8]) {
        write_atomically(self.fs, &self.lock, &self.path(name), bytes)
            .unwrap_or_else(|e| panic!("the setup of {name} could not be written: {e}"));
    }

    /// The names in the directory that are not the ones a scenario put there on purpose.
    fn leftovers(&self, expected: &[&str]) -> Vec<String> {
        self.disk
            .names_in(&self.root)
            .into_iter()
            .filter(|name| !expected.contains(&name.as_str()))
            .collect()
    }
}

/// One line of the table.
struct Scenario {
    name: String,
    /// What the fake is told to do. `Stage::Nothing` marks a scenario a real directory reaches.
    stage: Stage,
    setup: fn(&World),
    act: fn(&World) -> io::Result<()>,
    check: fn(&World, io::Result<()>),
}

fn nothing(_: &World) {}

fn scenarios() -> Vec<Scenario> {
    let mut table = vec![
        Scenario {
            name: "a file that was not there is created holding the bytes".to_owned(),
            stage: Stage::Nothing,
            setup: nothing,
            act: |w| write_atomically(w.fs, &w.lock, &w.path("note.md"), b"new"),
            check: |w, outcome| {
                outcome.expect("the write succeeds");
                assert_eq!(w.bytes("note.md").as_deref(), Some(&b"new"[..]));
            },
        },
        Scenario {
            name: "an existing file is replaced whole".to_owned(),
            stage: Stage::Nothing,
            setup: |w| w.given("note.md", b"old and longer than what replaces it"),
            act: |w| write_atomically(w.fs, &w.lock, &w.path("note.md"), b"new"),
            check: |w, outcome| {
                outcome.expect("the write succeeds");
                assert_eq!(w.bytes("note.md").as_deref(), Some(&b"new"[..]));
            },
        },
        Scenario {
            name: "the mode of the file that was there is carried across the rename".to_owned(),
            stage: Stage::Nothing,
            setup: |w| {
                w.given("note.md", b"old");
                w.fs.set_mode(&w.path("note.md"), UNUSUAL)
                    .expect("the setup can set a mode");
            },
            act: |w| write_atomically(w.fs, &w.lock, &w.path("note.md"), b"new"),
            check: |w, outcome| {
                outcome.expect("the write succeeds");
                assert_eq!(w.bytes("note.md").as_deref(), Some(&b"new"[..]));
                assert_eq!(
                    w.disk.mode(&w.path("note.md")),
                    Some(UNUSUAL),
                    "the mode of the file that was replaced was not carried to the new one"
                );
            },
        },
        Scenario {
            name: "a file that was not there gets the default mode and not a neighbour's"
                .to_owned(),
            stage: Stage::Nothing,
            setup: |w| {
                w.given("neighbour.md", b"beside it");
                w.fs.set_mode(&w.path("neighbour.md"), UNUSUAL)
                    .expect("the setup can set a mode");
            },
            act: |w| write_atomically(w.fs, &w.lock, &w.path("note.md"), b"new"),
            check: |w, outcome| {
                outcome.expect("the write succeeds");
                assert_ne!(
                    w.disk.mode(&w.path("note.md")),
                    Some(UNUSUAL),
                    "a file that was not there has no mode to carry"
                );
            },
        },
        Scenario {
            name: "a write that finished leaves no file of the reserved shape".to_owned(),
            stage: Stage::Nothing,
            setup: |w| w.given("note.md", b"old"),
            act: |w| write_atomically(w.fs, &w.lock, &w.path("note.md"), b"new"),
            check: |w, outcome| {
                outcome.expect("the write succeeds");
                assert_eq!(
                    w.leftovers(&["note.md"]),
                    Vec::<String>::new(),
                    "a write that reached its rename left something behind"
                );
            },
        },
        Scenario {
            name: "a create that must not replace is refused, and what is there is untouched"
                .to_owned(),
            stage: Stage::Nothing,
            setup: |w| w.given("note.md", b"old"),
            act: |w| w.fs.create_new(&w.path("note.md")).map(|_| ()),
            check: |w, outcome| {
                let error = outcome.expect_err("a file that is already there is refused");
                assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
                assert_eq!(w.bytes("note.md").as_deref(), Some(&b"old"[..]));
            },
        },
        Scenario {
            name: "a rename replaces what is at the destination".to_owned(),
            stage: Stage::Nothing,
            setup: |w| {
                w.given("from.md", b"from");
                w.given("to.md", b"to");
            },
            act: |w| w.fs.rename(&w.path("from.md"), &w.path("to.md")),
            check: |w, outcome| {
                outcome.expect("the rename succeeds");
                assert_eq!(w.bytes("to.md").as_deref(), Some(&b"from"[..]));
                assert_eq!(w.bytes("from.md"), None);
            },
        },
        Scenario {
            name: "a file is removed, and removing what is not there is not found".to_owned(),
            stage: Stage::Nothing,
            setup: |w| w.given("note.md", b"here"),
            act: |w| w.fs.remove_file(&w.path("note.md")),
            check: |w, outcome| {
                outcome.expect("the removal succeeds");
                assert_eq!(w.bytes("note.md"), None);
                let again =
                    w.fs.remove_file(&w.path("note.md"))
                        .expect_err("removing what is not there fails");
                assert_eq!(again.kind(), io::ErrorKind::NotFound);
            },
        },
        Scenario {
            name: "a directory and its parents are made, and making them again is no failure"
                .to_owned(),
            stage: Stage::Nothing,
            setup: nothing,
            act: |w| {
                w.fs.create_dir_all(&w.path("one/two"))?;
                w.fs.create_dir_all(&w.path("one/two"))
            },
            check: |w, outcome| {
                outcome.expect("the directory is made");
                assert_eq!(w.fs.exists(&w.path("one/two")).ok(), Some(true));
            },
        },
        Scenario {
            name: "a path is one file with itself, and not with another or with nothing".to_owned(),
            stage: Stage::Nothing,
            setup: |w| {
                w.given("one.md", b"one");
                w.given("two.md", b"two");
            },
            act: |_| Ok(()),
            check: |w, _| {
                let (one, two) = (w.path("one.md"), w.path("two.md"));
                assert_eq!(w.fs.same_file(&one, &one).ok(), Some(true));
                assert_eq!(w.fs.same_file(&one, &two).ok(), Some(false));
                assert_eq!(w.fs.same_file(&one, &w.path("gone.md")).ok(), Some(false));
            },
        },
        Scenario {
            name: "what is there exists and what is not there does not".to_owned(),
            stage: Stage::Nothing,
            setup: |w| w.given("note.md", b"here"),
            act: |_| Ok(()),
            check: |w, _| {
                assert_eq!(w.fs.exists(&w.path("note.md")).ok(), Some(true));
                assert_eq!(w.fs.exists(&w.path("gone.md")).ok(), Some(false));
            },
        },
        Scenario {
            name: "an open handle takes bytes and can be asked to hold them".to_owned(),
            stage: Stage::Nothing,
            setup: nothing,
            act: |w| {
                let mut handle = w.fs.create_new(&w.path("note.md"))?;
                handle.write_all(b"held")?;
                handle.sync()
            },
            check: |w, outcome| {
                outcome.expect("the handle takes the bytes");
                assert_eq!(w.bytes("note.md").as_deref(), Some(&b"held"[..]));
            },
        },
        Scenario {
            name: "no space left stops the write and leaves the file that was there".to_owned(),
            stage: Stage::Fail(On::Write, Failure::NoSpace),
            setup: |w| w.given("note.md", b"old"),
            act: |w| write_atomically(w.fs, &w.lock, &w.path("note.md"), b"new"),
            check: |w, outcome| {
                let error = outcome.expect_err("a write with no space left fails");
                assert_eq!(error.raw_os_error(), Some(28));
                assert_eq!(w.bytes("note.md").as_deref(), Some(&b"old"[..]));
                assert_eq!(w.leftovers(&["note.md"]), Vec::<String>::new());
            },
        },
        Scenario {
            name: "a permission refused on the create stops the write before anything is made"
                .to_owned(),
            stage: Stage::Fail(On::Create, Failure::PermissionDenied),
            setup: |w| w.given("note.md", b"old"),
            act: |w| write_atomically(w.fs, &w.lock, &w.path("note.md"), b"new"),
            check: |w, outcome| {
                let error = outcome.expect_err("a create without the permission fails");
                assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
                assert_eq!(w.bytes("note.md").as_deref(), Some(&b"old"[..]));
                assert_eq!(w.leftovers(&["note.md"]), Vec::<String>::new());
            },
        },
        Scenario {
            name: "a rename across devices stops the write and leaves the file that was there"
                .to_owned(),
            stage: Stage::Fail(On::Rename, Failure::CrossesDevices),
            setup: |w| w.given("note.md", b"old"),
            act: |w| write_atomically(w.fs, &w.lock, &w.path("note.md"), b"new"),
            check: |w, outcome| {
                let error = outcome.expect_err("a rename across devices fails");
                assert_eq!(error.raw_os_error(), Some(18));
                assert_eq!(w.bytes("note.md").as_deref(), Some(&b"old"[..]));
                assert_eq!(w.leftovers(&["note.md"]), Vec::<String>::new());
            },
        },
        // `create_exclusively` is `new`'s own half of the seam: no temp file and no rename, a
        // file created once and never replaced (decision 7).
        Scenario {
            name: "create_exclusively makes a file that was not there, holding the bytes"
                .to_owned(),
            stage: Stage::Nothing,
            setup: nothing,
            act: |w| create_exclusively(w.fs, &w.lock, &w.path("note.md"), b"new"),
            check: |w, outcome| {
                outcome.expect("the create succeeds");
                assert_eq!(w.bytes("note.md").as_deref(), Some(&b"new"[..]));
            },
        },
        Scenario {
            name:
                "create_exclusively refuses a file that is already there, and leaves it untouched"
                    .to_owned(),
            stage: Stage::Nothing,
            setup: |w| w.given("note.md", b"old"),
            act: |w| create_exclusively(w.fs, &w.lock, &w.path("note.md"), b"new"),
            check: |w, outcome| {
                let error = outcome.expect_err("a destination that exists is refused");
                assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
                assert_eq!(w.bytes("note.md").as_deref(), Some(&b"old"[..]));
            },
        },
        Scenario {
            name: "create_exclusively makes the folder that will hold the file first".to_owned(),
            stage: Stage::Nothing,
            setup: nothing,
            act: |w| create_exclusively(w.fs, &w.lock, &w.path("tickets/WF-3.md"), b"new"),
            check: |w, outcome| {
                outcome.expect("the create succeeds, folder included");
                assert_eq!(w.bytes("tickets/WF-3.md").as_deref(), Some(&b"new"[..]));
            },
        },
        Scenario {
            name: "create_exclusively leaves nothing behind when the write itself fails".to_owned(),
            stage: Stage::Fail(On::Write, Failure::NoSpace),
            setup: nothing,
            act: |w| create_exclusively(w.fs, &w.lock, &w.path("note.md"), b"new"),
            check: |w, outcome| {
                let error = outcome.expect_err("a write with no space left fails");
                assert_eq!(error.raw_os_error(), Some(28));
                assert_eq!(
                    w.bytes("note.md"),
                    None,
                    "a create that never finished writing must not leave a half-written document \
                     at the final path, since there is no temp file to have held it instead"
                );
            },
        },
    ];

    table.push(Scenario {
        name: "a run stopped before the bytes are written has already carried the mode".to_owned(),
        // The read of the mode, the create and the setting of the mode, and then nothing.
        stage: Stage::StopAfter(3),
        setup: |w| {
            w.given("note.md", b"old");
            w.fs.set_mode(&w.path("note.md"), UNUSUAL)
                .expect("the setup can set a mode");
        },
        act: |w| write_atomically(w.fs, &w.lock, &w.path("note.md"), b"new"),
        check: |w, outcome| {
            outcome.expect_err("a run that stopped does not report success");
            let left = w.leftovers(&["note.md"]);
            assert_eq!(
                left.len(),
                1,
                "the one file the run had made should still be there: {left:?}"
            );
            for name in &left {
                assert_eq!(
                    w.disk.mode(&w.path(name)),
                    Some(UNUSUAL),
                    "the bytes of a document nobody else may read were about to be written \
                     into a file under a looser mode"
                );
            }
        },
    });

    // A write makes five operations, so a stop after each of nought to five of them falls
    // between every two of them and after the last.
    for stopped_after in 0..=5 {
        table.push(Scenario {
            name: format!("a run stopped after {stopped_after} operations damages no document"),
            stage: Stage::StopAfter(stopped_after),
            setup: |w| w.given("note.md", b"old"),
            act: |w| write_atomically(w.fs, &w.lock, &w.path("note.md"), b"new"),
            check: |w, _| {
                let held = w.bytes("note.md");
                assert!(
                    held.as_deref() == Some(&b"old"[..]) || held.as_deref() == Some(&b"new"[..]),
                    "a reader saw neither the old file nor the new one whole: {held:?}"
                );
                for name in w.leftovers(&["note.md"]) {
                    assert!(
                        is_temp_name(&name),
                        "a run that stopped left {name}, which no walker would skip"
                    );
                }
            },
        });
    }

    table
}

/// The fake's own directory, which is a key rather than a place.
fn fake_root() -> PathBuf {
    PathBuf::from("/project")
}

impl Disk for FakeFs {
    fn bytes(&self, path: &Path) -> Option<Vec<u8>> {
        FakeFs::bytes(self, path)
    }

    fn mode(&self, path: &Path) -> Option<Mode> {
        self.mode_of(path)
    }

    fn names_in(&self, directory: &Path) -> Vec<String> {
        FakeFs::names_in(self, directory)
    }
}

/// A real temporary directory, read with the standard library rather than through the seam.
struct RealDisk;

impl Disk for RealDisk {
    fn bytes(&self, path: &Path) -> Option<Vec<u8>> {
        std::fs::read(path).ok()
    }

    fn mode(&self, path: &Path) -> Option<Mode> {
        use std::os::unix::fs::PermissionsExt;
        std::fs::metadata(path).ok().map(|m| m.permissions().mode())
    }

    fn names_in(&self, directory: &Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(directory)
            .into_iter()
            .flatten()
            .flatten()
            .filter(|entry| entry.path().is_file())
            .filter_map(|entry| entry.file_name().to_str().map(str::to_owned))
            .collect();
        names.sort();
        names
    }
}

/// Sets a scenario up, arms whatever the backend can stage, acts, and checks. Both backends
/// go through here, so the two runs of the table cannot drift apart.
fn run(scenario: &Scenario, world: &World, arm: impl Fn(Stage)) {
    (scenario.setup)(world);
    arm(scenario.stage);
    let outcome = (scenario.act)(world);
    (scenario.check)(world, outcome);
}

#[test]
fn every_scenario_holds_against_the_fake() {
    let table = scenarios();
    assert!(
        !table.is_empty(),
        "the table is empty, so it checks nothing"
    );

    for scenario in &table {
        let fake = FakeFs::new();
        let lock = a_lock(&fake, Path::new("/locks/scenario.lock"));
        let world = World {
            fs: &fake,
            disk: &fake,
            root: fake_root(),
            lock,
        };
        run(scenario, &world, |stage| fake.arm(stage));
        println!("fake: {}", scenario.name);
    }
}

#[test]
fn every_scenario_a_real_directory_reaches_holds_against_a_real_temporary_directory() {
    let table = scenarios();
    let reachable: Vec<&Scenario> = table.iter().filter(|s| s.stage == Stage::Nothing).collect();
    assert!(
        reachable.len() >= 8,
        "a real directory reaches {} of the scenarios, which is too few to hold the fake to \
         what a file system does",
        reachable.len()
    );

    let lock_dir = TempDir::new().expect("a temporary directory for locks can be made");
    let mut ran = 0;
    for scenario in reachable {
        let directory = TempDir::new().expect("a temporary directory can be made");
        let lock = a_lock(&SystemFs, &lock_dir.path().join("scenario.lock"));
        let world = World {
            fs: &SystemFs,
            disk: &RealDisk,
            root: directory.path().to_path_buf(),
            lock,
        };
        // A real directory stages nothing, which is what puts a scenario in this half.
        run(scenario, &world, |_| {});
        ran += 1;
        println!("real: {}", scenario.name);
    }
    assert_eq!(
        ran,
        scenarios()
            .iter()
            .filter(|s| s.stage == Stage::Nothing)
            .count(),
        "a scenario a real directory reaches was not run"
    );
}

/// What neither the fake nor a real directory covers, written down rather than assumed.
///
/// A power cut and a `SIGKILL` land between two system calls, where no code of this program
/// runs: no error comes back, no cleanup happens, and neither backend can produce that.
/// `Stage::StopAfter` is as close as the fake comes, and it is not the same thing — it stops
/// the program between two of its own operations, which is a point the program chose.
const COVERED_BY_NEITHER: [&str; 2] = ["a power cut", "a SIGKILL between two system calls"];

#[test]
fn nothing_in_the_table_claims_to_cover_a_power_cut_or_a_kill() {
    for gap in COVERED_BY_NEITHER {
        for scenario in scenarios() {
            assert!(
                !scenario.name.contains(gap),
                "the table has a scenario named for {gap}, which neither backend can stage"
            );
        }
    }
}

#[test]
fn what_a_run_that_stopped_leaves_is_a_leftover_and_not_a_promise_that_there_is_none() {
    let fake = FakeFs::new();
    let note = PathBuf::from("/project/note.md");
    let lock = a_lock(&fake, Path::new("/locks/note.lock"));
    write_atomically(&fake, &lock, &note, b"old").expect("the setup succeeds");

    // Everything up to and including the write of the bytes, and then nothing: no rename, and
    // no removal of the temp file either, because the program that would have done it is gone.
    fake.arm(Stage::StopAfter(4));
    let outcome = write_atomically(&fake, &lock, &note, b"new");

    outcome.expect_err("a run that stopped does not report success");
    assert_eq!(
        fake.bytes(&note).as_deref(),
        Some(&b"old"[..]),
        "the document was damaged by a run that stopped"
    );
    let left: Vec<String> = FakeFs::names_in(&fake, Path::new("/project"))
        .into_iter()
        .filter(|name| name != "note.md")
        .collect();
    assert_eq!(
        left.len(),
        1,
        "a run stopped before its rename leaves the one file it had made: {left:?}"
    );
    for name in &left {
        assert!(
            is_temp_name(name),
            "{name} was left behind and is not of the shape a walker skips"
        );
    }
}
