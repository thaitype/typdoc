//! A file system in memory, and a clock that does not move.
//!
//! The fake is here for the failures a real directory will not produce when a test asks: no
//! space left, a permission refused, a rename across devices, and a run that stops between two
//! operations. The same scenarios run against a real temporary directory, which is what holds
//! the fake to what a file system does rather than to what it was written to do.

use std::collections::{BTreeMap, BTreeSet};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

use chrono::{DateTime, FixedOffset};
use typdoc_core::{Clock, FileId, Fs, Mode, WriteHandle};

/// The mode a file gets from [`FakeFs::create_new`] when nothing sets one: what a umask that
/// takes away nothing from the group and others leaves of a regular file.
const DEFAULT_MODE: Mode = 0o100_644;

/// Which operation a staged failure lands on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum On {
    Create,
    Write,
    SetMode,
    Rename,
    Remove,
    MakeDir,
    ReadMode,
    Exists,
    SameFile,
    IdentityAt,
    HandleIdentity,
}

/// A failure a real directory will not produce on request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Failure {
    /// `ENOSPC`, which a real directory reaches only by being filled.
    NoSpace,
    PermissionDenied,
    /// `EXDEV`: a rename whose two ends are on different file systems.
    CrossesDevices,
}

impl Failure {
    fn error(self) -> io::Error {
        match self {
            Failure::NoSpace => io::Error::from_raw_os_error(28),
            Failure::PermissionDenied => io::Error::from(io::ErrorKind::PermissionDenied),
            Failure::CrossesDevices => io::Error::from_raw_os_error(18),
        }
    }
}

/// What the fake has been told to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    /// Behave as a file system does, which is every scenario a real directory also reaches.
    Nothing,
    /// Every operation of this kind fails, and changes nothing.
    Fail(On, Failure),
    /// The run stops once this many operations have been performed: every operation after
    /// that changes nothing and fails, the way a process that is gone performs no more system
    /// calls and runs no cleanup of its own.
    StopAfter(usize),
}

#[derive(Debug, Clone)]
struct Entry {
    bytes: Vec<u8>,
    mode: Mode,
    /// Which file this is, so that two paths can be asked whether they are one file.
    identity: u64,
}

#[derive(Debug)]
struct State {
    files: BTreeMap<PathBuf, Entry>,
    directories: BTreeSet<PathBuf>,
    stage: Stage,
    /// Operations attempted since the fake was armed, a refused one included.
    attempted: usize,
    next_identity: u64,
}

impl State {
    /// Counts one operation and says whether it may go ahead.
    fn admit(&mut self, on: On) -> io::Result<()> {
        if let Stage::StopAfter(limit) = self.stage
            && self.attempted >= limit
        {
            return Err(io::Error::other("the run stopped"));
        }
        self.attempted += 1;
        if let Stage::Fail(staged, failure) = self.stage
            && staged == on
        {
            return Err(failure.error());
        }
        Ok(())
    }

    /// A number no file of this fake has had, so that two paths can be told apart the way two
    /// inodes are.
    fn fresh_identity(&mut self) -> u64 {
        let identity = self.next_identity;
        self.next_identity += 1;
        identity
    }
}

/// A file system in memory.
///
/// It is cloned by handle rather than by content: a clone reads and changes the same files, so
/// a test can hold one and hand another to the code under test.
#[derive(Clone)]
pub struct FakeFs {
    state: Arc<Mutex<State>>,
}

impl Default for FakeFs {
    fn default() -> Self {
        Self::new()
    }
}

impl FakeFs {
    pub fn new() -> Self {
        FakeFs {
            state: Arc::new(Mutex::new(State {
                files: BTreeMap::new(),
                directories: BTreeSet::new(),
                stage: Stage::Nothing,
                attempted: 0,
                next_identity: 1,
            })),
        }
    }

    /// Tells the fake what to do from here on, and starts the count of operations again, so
    /// that what a scenario set up first is not counted against the failure it stages.
    pub fn arm(&self, stage: Stage) {
        let mut state = self.locked();
        state.stage = stage;
        state.attempted = 0;
    }

    /// What a file holds, for a test to look at without going through the seam.
    pub fn bytes(&self, path: &Path) -> Option<Vec<u8>> {
        self.locked().files.get(path).map(|e| e.bytes.clone())
    }

    /// A file's mode, for a test to look at without going through the seam.
    pub fn mode_of(&self, path: &Path) -> Option<Mode> {
        self.locked().files.get(path).map(|e| e.mode)
    }

    /// The names of the files directly inside a directory, sorted.
    pub fn names_in(&self, directory: &Path) -> Vec<String> {
        self.locked()
            .files
            .keys()
            .filter(|path| path.parent() == Some(directory))
            .filter_map(|path| path.file_name()?.to_str().map(str::to_owned))
            .collect()
    }

    /// Puts a file there without going through the seam, for a scenario's setup.
    pub fn put(&self, path: &Path, bytes: &[u8], mode: Mode) {
        let mut state = self.locked();
        let identity = state.fresh_identity();
        state.files.insert(
            path.to_path_buf(),
            Entry {
                bytes: bytes.to_vec(),
                mode,
                identity,
            },
        );
    }

    fn locked(&self) -> MutexGuard<'_, State> {
        locked(&self.state)
    }
}

/// A test that panicked while holding the fake has already failed; going on with what it left
/// is better than a second failure that hides the first.
fn locked(state: &Arc<Mutex<State>>) -> MutexGuard<'_, State> {
    match state.lock() {
        Ok(state) => state,
        Err(poisoned) => poisoned.into_inner(),
    }
}

impl Fs for FakeFs {
    fn create_new(&self, path: &Path) -> io::Result<Box<dyn WriteHandle>> {
        let mut state = self.locked();
        state.admit(On::Create)?;
        if state.files.contains_key(path) {
            return Err(io::Error::from(io::ErrorKind::AlreadyExists));
        }
        let identity = state.fresh_identity();
        state.files.insert(
            path.to_path_buf(),
            Entry {
                bytes: Vec::new(),
                mode: DEFAULT_MODE,
                identity,
            },
        );
        Ok(Box::new(FakeHandle {
            state: Arc::clone(&self.state),
            path: path.to_path_buf(),
            identity,
        }))
    }

    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        let mut state = self.locked();
        state.admit(On::Rename)?;
        let Some(entry) = state.files.remove(from) else {
            return Err(io::Error::from(io::ErrorKind::NotFound));
        };
        state.files.insert(to.to_path_buf(), entry);
        Ok(())
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
        let mut state = self.locked();
        state.admit(On::Remove)?;
        match state.files.remove(path) {
            Some(_) => Ok(()),
            None => Err(io::Error::from(io::ErrorKind::NotFound)),
        }
    }

    fn create_dir_all(&self, path: &Path) -> io::Result<()> {
        let mut state = self.locked();
        state.admit(On::MakeDir)?;
        let mut here = PathBuf::new();
        for part in path {
            here.push(part);
            state.directories.insert(here.clone());
        }
        Ok(())
    }

    fn mode(&self, path: &Path) -> io::Result<Mode> {
        let mut state = self.locked();
        state.admit(On::ReadMode)?;
        match state.files.get(path) {
            Some(entry) => Ok(entry.mode),
            None => Err(io::Error::from(io::ErrorKind::NotFound)),
        }
    }

    fn set_mode(&self, path: &Path, mode: Mode) -> io::Result<()> {
        let mut state = self.locked();
        state.admit(On::SetMode)?;
        match state.files.get_mut(path) {
            Some(entry) => {
                entry.mode = mode;
                Ok(())
            }
            None => Err(io::Error::from(io::ErrorKind::NotFound)),
        }
    }

    fn exists(&self, path: &Path) -> io::Result<bool> {
        let mut state = self.locked();
        state.admit(On::Exists)?;
        Ok(state.files.contains_key(path) || state.directories.contains(path))
    }

    fn same_file(&self, a: &Path, b: &Path) -> io::Result<bool> {
        let mut state = self.locked();
        state.admit(On::SameFile)?;
        let (Some(a), Some(b)) = (state.files.get(a), state.files.get(b)) else {
            return Ok(false);
        };
        Ok(a.identity == b.identity)
    }

    fn identity_at(&self, path: &Path) -> io::Result<Option<FileId>> {
        let mut state = self.locked();
        state.admit(On::IdentityAt)?;
        Ok(state.files.get(path).map(|entry| FileId {
            device: 0,
            inode: entry.identity,
            links: 1,
        }))
    }
}

struct FakeHandle {
    state: Arc<Mutex<State>>,
    path: PathBuf,
    /// Captured when the handle was made, and never re-read from `path`: a real `fstat` on an
    /// open descriptor answers for the file the descriptor was opened on, whatever a later
    /// writer does to the name.
    identity: u64,
}

impl WriteHandle for FakeHandle {
    fn write_all(&mut self, bytes: &[u8]) -> io::Result<()> {
        let mut state = locked(&self.state);
        state.admit(On::Write)?;
        match state.files.get_mut(&self.path) {
            Some(entry) => {
                entry.bytes.extend_from_slice(bytes);
                Ok(())
            }
            None => Err(io::Error::from(io::ErrorKind::NotFound)),
        }
    }

    fn sync(&mut self) -> io::Result<()> {
        Ok(())
    }

    fn identity(&self) -> io::Result<FileId> {
        let mut state = locked(&self.state);
        state.admit(On::HandleIdentity)?;
        // The link count is whether any entry in the fake still holds this identity: the fake
        // never reuses an identity (`fresh_identity` only counts up), so a `remove_file` that
        // drops the one entry that had it, and nothing else taking its place, is what "the open
        // file's link count is zero" means here.
        let links = u64::from(
            state
                .files
                .values()
                .any(|entry| entry.identity == self.identity),
        );
        Ok(FileId {
            device: 0,
            inode: self.identity,
            links,
        })
    }
}

/// The instant the fixed clock reports: obviously not now, with every part of it different
/// from every other, so that a formatting that puts one where another belongs is visible, and
/// with an offset that is not UTC, so that an offset dropped on the way is visible too.
pub const FIXED_INSTANT: &str = "2001-02-03T04:05:06+07:00";

/// A clock that does not move, so that a golden file can hold a time.
pub struct FixedClock(DateTime<FixedOffset>);

impl Default for FixedClock {
    fn default() -> Self {
        Self::new()
    }
}

impl FixedClock {
    pub fn new() -> Self {
        FixedClock(
            FIXED_INSTANT
                .parse()
                .unwrap_or_else(|e| panic!("{FIXED_INSTANT} is an instant with an offset: {e}")),
        )
    }
}

impl Clock for FixedClock {
    fn now(&self) -> DateTime<FixedOffset> {
        self.0
    }
}
