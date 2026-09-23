//! The namespace lock, and the one function that creates or releases one.
//!
//! Not `lock.rs`: that name is taken by the reader of `.typdoc/lock.json`, the pin file, which
//! this module has nothing to do with. This one is a mutex on a namespace (or on the project,
//! for the pins a later story writes), backed by a file made with `O_EXCL`.
//!
//! A held lock is [`NamespaceLock`], a value with no public constructor and no public field.
//! [`acquire`] is the only function that returns one, and it is the only function that creates
//! a lock file. A function that writes takes a `&NamespaceLock` (decision 6): there is nothing
//! to remember, because a write attempted without one does not compile — proved in
//! `tests/namespace_lock_compile_fail.rs`, not merely asserted here.

use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::clock::Clock;
use crate::error::Error;
use crate::fs::{FileId, Fs};

/// `.typdoc/locks/<namespace>.lock`, the `local` mode of the lock table.
pub fn local_namespace_lock_path(project_root: &Path, namespace: &str) -> PathBuf {
    project_root
        .join(".typdoc/locks")
        .join(format!("{namespace}.lock"))
}

/// `.typdoc/locks/.project.lock`: a name that starts with `.`, so no namespace can have it.
pub fn local_project_lock_path(project_root: &Path) -> PathBuf {
    project_root.join(".typdoc/locks/.project.lock")
}

/// `$(git rev-parse --git-common-dir)/typdoc/<project-hash>-<namespace>.lock`. `git_common_dir`
/// is taken already canonicalized: obtaining it, and running `git` at all, is outside this
/// module, which has no way to spawn a process and no need to, since the caller that resolves a
/// project's lock mode is the one place that already knows the worktree it is running in.
pub fn git_common_namespace_lock_path(
    git_common_dir: &Path,
    project_hash: &str,
    namespace: &str,
) -> PathBuf {
    git_common_dir
        .join("typdoc")
        .join(format!("{project_hash}-{namespace}.lock"))
}

/// `<project-hash>.lock`, beside the namespace locks in the same `typdoc/` folder.
pub fn git_common_project_lock_path(git_common_dir: &Path, project_hash: &str) -> PathBuf {
    git_common_dir
        .join("typdoc")
        .join(format!("{project_hash}.lock"))
}

/// The SHA-256 of `relative`, brought to one form first (`/` separators, no trailing one),
/// truncated to its first 16 hexadecimal characters (decision 14).
pub fn project_hash_of(relative: &str) -> String {
    let normalized = relative.replace('\\', "/");
    let normalized = normalized.trim_end_matches('/');
    let digest = Sha256::digest(normalized.as_bytes());
    digest
        .iter()
        .take(8)
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// `<project-hash>`: the project folder's path relative to the root of the worktree, hashed as
/// [`project_hash_of`] does it. `None` when `project_folder` is not under `worktree_root` at
/// all, which is a caller error rather than a hash this function can produce.
pub fn project_hash(worktree_root: &Path, project_folder: &Path) -> Option<String> {
    let relative = project_folder.strip_prefix(worktree_root).ok()?;
    let joined = relative
        .components()
        .map(|part| part.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/");
    Some(project_hash_of(&joined))
}

/// Orders lock files the way decision 3 fixes: the project lock first when there is one, then
/// every namespace lock in the order of the lock files' own paths, compared byte by byte as
/// absolute paths. Not [`Path`]'s own ordering, which compares components and so does not agree
/// with a byte comparison on every input (two paths differing only by a doubled separator are
/// one component sequence and so equal to `Path`, and two different byte strings to this rule).
pub fn order_locks(
    project_lock: Option<PathBuf>,
    mut namespace_locks: Vec<PathBuf>,
) -> Vec<PathBuf> {
    namespace_locks.sort_by(|a, b| path_bytes(a).cmp(path_bytes(b)));
    let mut ordered = Vec::with_capacity(namespace_locks.len() + 1);
    ordered.extend(project_lock);
    ordered.extend(namespace_locks);
    ordered
}

#[cfg(unix)]
fn path_bytes(path: &Path) -> &[u8] {
    use std::os::unix::ffi::OsStrExt;
    path.as_os_str().as_bytes()
}

/// What a lock file holds, read back from an existing one when this process could not create
/// its own.
#[derive(Debug, Clone, Deserialize)]
struct Owner {
    pid: u32,
    host: String,
    timestamp: String,
}

/// What this process writes into a lock file it creates.
#[derive(Debug, Clone, Serialize)]
struct Stamp<'a> {
    pid: u32,
    host: &'a str,
    timestamp: String,
}

/// What [`acquire`] records for a lock it just created, and what the signal-triggered cleanup
/// thread reads back (see the module doc below the registry's own functions).
#[derive(Debug, Clone)]
struct Registered {
    id: u64,
    path: PathBuf,
    expected: FileId,
}

/// Every lock this process currently holds, by the path it lives at and the identity [`acquire`]
/// read from the handle right after creating it.
///
/// This exists because [`NamespaceLock`] cannot cross a thread: it holds `&dyn Fs` and a boxed
/// `WriteHandle` with no `Send` bound, so a cleanup thread spawned for `SIGINT`/`SIGTERM`
/// (decision 6) cannot call [`release`] on a lock the acquiring thread still owns. What the
/// identity check needs is not the open handle itself, only the two things a `stat` can be
/// compared against — the path, and the device/inode/link-count `acquire` already read from the
/// handle once — so that is what crosses the thread instead, under this mutex.
///
/// Caching the identity at creation rather than reading it fresh at cleanup time loses nothing
/// this check needs: the device and inode of a still-open file do not change for as long as the
/// handle stays open, whatever happens to the path, so a `stat` on the path that still shows the
/// same device and inode at cleanup time is exactly the evidence [`release_checked`] itself
/// would have found from a fresh `fstat`. The one case that only a live `fstat` could catch — the
/// same path relinked back to the exact same, still-open inode with the link count read as zero
/// in between — cannot happen while this process keeps the handle open, because the kernel does
/// not free an inode, and so never reuses its number, while any handle still refers to it.
static REGISTRY: Mutex<Vec<Registered>> = Mutex::new(Vec::new());
static NEXT_ID: AtomicU64 = AtomicU64::new(0);

fn register(path: PathBuf, expected: FileId) -> u64 {
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    let mut guard = REGISTRY
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    guard.push(Registered { id, path, expected });
    id
}

fn deregister(id: u64) {
    let mut guard = REGISTRY
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    guard.retain(|entry| entry.id != id);
}

/// Removes every lock this process still holds, for the cleanup thread `typdoc`'s own binary
/// spawns off the `SIGINT`/`SIGTERM` handler (decisions 5 and 6): the same identity check
/// [`release_checked`] runs, read from the registry above instead of from an open handle, since
/// the handle cannot reach this thread. A lock whose path no longer shows the identity `acquire`
/// recorded — taken away and replaced by something else, or simply gone — is left alone, the
/// same refusal [`release_checked`] already makes: this process never removes a lock it did not
/// create, on the signal path any more than on the ordinary one.
///
/// Nothing here removes an entry from the registry: the process ends by the signal right after
/// this runs (`typdoc`'s own cleanup thread), so there is no later caller left to confuse.
pub fn release_all_for_signal(fs: &dyn Fs) {
    let entries: Vec<Registered> = {
        let guard = REGISTRY
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        guard.clone()
    };
    for entry in entries {
        let current = fs.identity_at(&entry.path).ok().flatten();
        if identity_matches(entry.expected, current) {
            let _ = fs.remove_file(&entry.path);
        }
    }
}

/// The design's own identity check, shared by [`release_checked`] (a live `fstat` on the
/// handle) and [`release_all_for_signal`] (the `FileId` `acquire` cached, since the signal
/// path's cleanup thread cannot reach the handle itself): `held` still names a file with at
/// least one link, and `current` — a `stat` on the lock's path — names that same file.
fn identity_matches(held: FileId, current: Option<FileId>) -> bool {
    held.links > 0 && current.is_some_and(|id| id.device == held.device && id.inode == held.inode)
}

/// A namespace's lock (or the project's), held for as long as this value lives.
///
/// No public constructor and no public field: [`acquire`] is the only function that makes one.
/// It keeps the lock file's handle open from creation to release, which is what lets [`release`]
/// tell its own file from whatever the path leads to by the time it is asked to remove it — the
/// design's identity check, `stat` on the path against `fstat` on this handle.
pub struct NamespaceLock<'a> {
    fs: &'a dyn Fs,
    path: PathBuf,
    handle: Box<dyn crate::fs::WriteHandle>,
    released: bool,
    /// This lock's own entry in [`REGISTRY`], removed on [`release`] or on [`Drop`], whichever
    /// runs first.
    registry_id: u64,
}

impl NamespaceLock<'_> {
    /// The lock file's path, for a caller that needs to name it (a message, a second lock in
    /// the order decision 3 fixes).
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Named by its path alone: `fs` and `handle` are trait objects with no `Debug` of their own,
/// and the path is what a test failure or a log needs to say which lock this was.
impl std::fmt::Debug for NamespaceLock<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NamespaceLock")
            .field("path", &self.path)
            .finish_non_exhaustive()
    }
}

/// What [`release`] (or a lock dropped without it) found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Released {
    /// The path was still this handle's own file, and it is gone now.
    ByUs,
    /// The path no longer names this handle's file, or the handle's own link count was zero:
    /// removing anything on that evidence would be exactly the takeover the design forbids, so
    /// nothing was removed. The lock this process thought it held was taken away during the
    /// operation.
    TakenByAnother,
}

impl Drop for NamespaceLock<'_> {
    fn drop(&mut self) {
        if !self.released {
            // A command that forgets to call `release` still cannot hold the lock past its own
            // scope (decision 6): dropping does the same identity-checked release, silently,
            // since there is no channel left to report through by the time this runs.
            let _ = release_checked(self.fs, &self.path, self.handle.as_ref());
        }
        // Runs whichever way this value's life ended, so the registry the signal-triggered
        // cleanup thread reads never outlives the lock it describes.
        deregister(self.registry_id);
    }
}

/// Releases `lock`, reporting whether the file removed was still this process's own.
///
/// This is the one place a lock file is ever removed: [`acquire`] never removes one it did not
/// just create, and a lock dropped without calling this runs the same check, silently.
pub fn release(mut lock: NamespaceLock<'_>) -> io::Result<Released> {
    let outcome = release_checked(lock.fs, &lock.path, lock.handle.as_ref())?;
    lock.released = true;
    Ok(outcome)
}

fn release_checked(
    fs: &dyn Fs,
    path: &Path,
    handle: &dyn crate::fs::WriteHandle,
) -> io::Result<Released> {
    let held = handle.identity()?;
    let current = fs.identity_at(path)?;
    if identity_matches(held, current) {
        fs.remove_file(path)?;
        Ok(Released::ByUs)
    } else {
        Ok(Released::TakenByAnother)
    }
}

/// Creates `path` with `O_EXCL`, retrying with backoff until `timeout` elapses, then returns
/// [`Error::LockTimeout`]. Never deletes or takes over a lock another process created, whatever
/// its age (design, Concurrency: "no takeover ... whatever its age").
///
/// `host` is what this process would stamp into the lock file, and what a timeout message
/// compares a competing lock's recorded host against to say whether it can tell the owner has
/// stopped. Nothing in this module reads the machine's own hostname; the caller that first
/// wires a command to this function has to source one (`Env` reaches the environment for
/// everything else this crate reads, and has no such method yet).
///
/// The directory that will hold `path` is created first (decision 3: "the directory is created
/// before the first lock is taken"), so the first lock a project ever takes does not fail with
/// a plain "not found" for a `.typdoc/locks/` nobody has made yet.
pub fn acquire<'a>(
    fs: &'a dyn Fs,
    clock: &dyn Clock,
    path: PathBuf,
    host: &str,
    timeout: Duration,
) -> Result<NamespaceLock<'a>, Error> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs.create_dir_all(parent).map_err(|source| Error::Io {
            file: parent.to_owned(),
            source,
        })?;
    }
    let start = Instant::now();
    let mut backoff = Duration::from_millis(10);
    loop {
        match fs.create_new(&path) {
            Ok(mut handle) => {
                // Registered as soon as the handle can say what it is, so the window between
                // the kernel's own creation of the file and this process recording that it
                // holds it is as small as it can be made. It is not closed: a signal delivered
                // before this line finds a lock file no list in the process yet names, exactly
                // the residual window decision 6 writes down rather than promises away.
                let identity = handle.identity().map_err(|source| Error::Io {
                    file: path.clone(),
                    source,
                })?;
                let registry_id = register(path.clone(), identity);
                let stamp = Stamp {
                    pid: std::process::id(),
                    host,
                    timestamp: clock.now().to_rfc3339(),
                };
                let bytes = serde_json::to_vec(&stamp).unwrap_or_else(|_| b"{}".to_vec());
                handle.write_all(&bytes).map_err(|source| Error::Io {
                    file: path.clone(),
                    source,
                })?;
                return Ok(NamespaceLock {
                    fs,
                    path,
                    handle,
                    released: false,
                    registry_id,
                });
            }
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                let elapsed = start.elapsed();
                if elapsed >= timeout {
                    return Err(timeout_error(&path, host, clock.now()));
                }
                let remaining = timeout - elapsed;
                std::thread::sleep(backoff.min(remaining));
                backoff = (backoff * 2).min(Duration::from_millis(200));
            }
            Err(source) => {
                return Err(Error::Io {
                    file: path.clone(),
                    source,
                });
            }
        }
    }
}

/// Builds the exit-4 error: the path always, and pid/host/age when the competing lock file can
/// still be read and parsed (it may have gone, or been replaced, in the instant between the
/// last failed create and this read, which this function does not treat as a reason to fail).
fn timeout_error(path: &Path, our_host: &str, now: DateTime<FixedOffset>) -> Error {
    let message = match read_owner(path) {
        Some(owner) => owner_message(path, &owner, our_host, now),
        None => format!(
            "lock not acquired: {} (its owner could not be read)",
            path.display()
        ),
    };
    Error::LockTimeout {
        path: path.to_owned(),
        message,
    }
}

fn read_owner(path: &Path) -> Option<Owner> {
    let bytes = std::fs::read(path).ok()?;
    serde_json::from_slice(&bytes).ok()
}

fn owner_message(path: &Path, owner: &Owner, our_host: &str, now: DateTime<FixedOffset>) -> String {
    let age = owner
        .timestamp
        .parse::<DateTime<FixedOffset>>()
        .map(|stamped| now.signed_duration_since(stamped))
        .unwrap_or_default();
    let status = if owner.host != our_host {
        "it is on another host and cannot be checked from here: delete it only once you know \
         that process has stopped"
            .to_owned()
    } else if pid_alive(owner.pid) {
        "it is running on this machine: wait, or run again with a longer --lock-timeout".to_owned()
    } else {
        format!(
            "it is not running on this machine: the lock is stale, delete {}",
            path.display()
        )
    };
    format!(
        "lock not acquired: {} is held by pid {} on {} (age {}); {status}",
        path.display(),
        owner.pid,
        owner.host,
        format_age(age)
    )
}

/// Whether `pid` names a process this machine still has, checked the way that costs nothing to
/// ask and never decides whether a lock is valid, only the wording of a message about it
/// (design: "The pid check only chooses the wording; it never decides whether a lock is
/// valid"). `/proc` is Linux's, which is the platform this is run and claimed on; on a platform
/// without it the answer is always "not running", the same conservative wording an unreadable
/// `/proc` already gets below.
fn pid_alive(pid: u32) -> bool {
    Path::new("/proc").join(pid.to_string()).exists()
}

fn format_age(age: chrono::TimeDelta) -> String {
    let total = age.num_seconds().max(0);
    let (hours, rest) = (total / 3600, total % 3600);
    let (minutes, seconds) = (rest / 60, rest % 60);
    match (hours, minutes) {
        (0, 0) => format!("{seconds}s"),
        (0, _) => format!("{minutes}m{seconds}s"),
        _ => format!("{hours}h{minutes}m{seconds}s"),
    }
}

// The tests of `acquire`, `release` and the timeout message live in
// `tests/namespace_lock.rs`, not here: they need `typdoc_testkit::fake`, a dev-dependency that
// itself depends on this crate, and a `#[cfg(test)]` module inside `src/` is compiled as part
// of this crate's own build, which is a different instantiation of `typdoc_core` from the one
// `typdoc_testkit` was built against — the two `Fs`/`Clock` traits then do not unify. An
// integration test in `tests/` depends on the finished crate from the outside and has no such
// conflict, which is the same reason every other fake-backed test in this workspace lives
// there and not beside the code it tests.

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    // ---- ticket 4: the registry itself, proven directly here rather than through a fake file
    // system's file removal: `typdoc_testkit::fake`'s own `fresh_identity` never repeats a
    // value, so no fake-backed test can tell a leaked registry entry apart from a properly
    // deregistered one by which files end up removed — both look the same from there, since the
    // identity check alone already refuses a leaked entry's mismatched device/inode. This is a
    // unit test of `register`/`deregister` against `REGISTRY` itself, which is why it lives here
    // and not in `tests/namespace_lock.rs` (no `Fs` or `Clock` needed).

    fn registry_len() -> usize {
        REGISTRY
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .len()
    }

    /// `cargo test` runs this module's tests concurrently on several threads by default, and
    /// `REGISTRY` is one static shared by the whole process: every test below that reads a
    /// count relative to its own `before` holds this for its length, so two of them can never
    /// interleave their register/deregister calls and see each other's.
    static REGISTRY_TEST_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn deregister_removes_exactly_the_entry_register_returned_the_id_for() {
        let _exclusive = REGISTRY_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let before = registry_len();
        let id = register(
            PathBuf::from("/does/not/matter"),
            FileId {
                device: 1,
                inode: 2,
                links: 1,
            },
        );
        assert_eq!(registry_len(), before + 1, "register did not add an entry");

        deregister(id);

        assert_eq!(
            registry_len(),
            before,
            "deregister did not remove the entry it was given the id for"
        );
    }

    /// A minimal `Fs`/`WriteHandle` pair for the one test below that needs a real
    /// [`NamespaceLock`] value to drop — every method but the two `identity` calls is
    /// unreachable, since a plain drop touches nothing else.
    struct NoopHandle;

    impl crate::fs::WriteHandle for NoopHandle {
        fn write_all(&mut self, _bytes: &[u8]) -> io::Result<()> {
            unreachable!("not called by a plain drop")
        }

        fn sync(&mut self) -> io::Result<()> {
            unreachable!("not called by a plain drop")
        }

        fn identity(&self) -> io::Result<FileId> {
            Ok(FileId {
                device: 1,
                inode: 2,
                links: 1,
            })
        }
    }

    struct NoopFs;

    impl Fs for NoopFs {
        fn create_new(&self, _path: &Path) -> io::Result<Box<dyn crate::fs::WriteHandle>> {
            unreachable!("not called by a plain drop")
        }

        fn rename(&self, _from: &Path, _to: &Path) -> io::Result<()> {
            unreachable!("not called by a plain drop")
        }

        fn remove_file(&self, _path: &Path) -> io::Result<()> {
            // The drop below finds its own identity still matching (`identity_at` returns the
            // same device/inode `NoopHandle::identity` does), so this is reached and only needs
            // to succeed.
            Ok(())
        }

        fn create_dir_all(&self, _path: &Path) -> io::Result<()> {
            unreachable!("not called by a plain drop")
        }

        fn mode(&self, _path: &Path) -> io::Result<crate::fs::Mode> {
            unreachable!("not called by a plain drop")
        }

        fn set_mode(&self, _path: &Path, _mode: crate::fs::Mode) -> io::Result<()> {
            unreachable!("not called by a plain drop")
        }

        fn exists(&self, _path: &Path) -> io::Result<bool> {
            unreachable!("not called by a plain drop")
        }

        fn same_file(&self, _a: &Path, _b: &Path) -> io::Result<bool> {
            unreachable!("not called by a plain drop")
        }

        fn identity_at(&self, _path: &Path) -> io::Result<Option<FileId>> {
            Ok(Some(FileId {
                device: 1,
                inode: 2,
                links: 1,
            }))
        }
    }

    /// The one property `deregister_removes_exactly_the_entry_register_returned_the_id_for`
    /// does not reach: that [`NamespaceLock`]'s own `Drop` actually calls `deregister`, not
    /// only that `deregister` works when called directly. Built by hand rather than through
    /// [`acquire`], which this module's own doc already explains is not a way around decision
    /// 6's "no public constructor" — nothing outside this module can do the same, since these
    /// fields are private to it.
    #[test]
    fn dropping_a_namespace_lock_deregisters_it_even_when_release_is_never_called() {
        let _exclusive = REGISTRY_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let fs = NoopFs;
        let before = registry_len();
        let id = register(
            PathBuf::from("/does/not/matter"),
            FileId {
                device: 1,
                inode: 2,
                links: 1,
            },
        );
        let lock = NamespaceLock {
            fs: &fs,
            path: PathBuf::from("/does/not/matter"),
            handle: Box::new(NoopHandle),
            released: false,
            registry_id: id,
        };

        drop(lock);

        assert_eq!(
            registry_len(),
            before,
            "Drop must deregister even when release() is never called"
        );
    }

    // ---- decision 3: one order for taking more than one lock ----

    #[test]
    fn the_project_lock_sorts_first_whatever_the_namespace_paths_are() {
        let ordered = order_locks(
            Some(PathBuf::from("/project/.typdoc/locks/.project.lock")),
            vec![
                PathBuf::from("/project/.typdoc/locks/aaa.lock"),
                PathBuf::from("/project/.typdoc/locks/000.lock"),
            ],
        );

        assert_eq!(
            ordered,
            vec![
                PathBuf::from("/project/.typdoc/locks/.project.lock"),
                PathBuf::from("/project/.typdoc/locks/000.lock"),
                PathBuf::from("/project/.typdoc/locks/aaa.lock"),
            ]
        );
    }

    #[test]
    fn namespace_locks_sort_by_the_raw_bytes_of_the_path_and_not_by_path_s_own_ordering() {
        // `Path`'s own `Ord` walks components, which treats a doubled separator as the same
        // component sequence as a single one; a byte comparison does not. If this ever sorted
        // by `Path::cmp` instead of raw bytes, this assertion would still pass by accident on
        // most inputs, which is exactly why it is built to tell the two apart.
        let doubled = PathBuf::from("/project/.typdoc/locks//a.lock");
        let single = PathBuf::from("/project/.typdoc/locks/a.lock");
        assert_eq!(
            doubled.cmp(&single),
            std::cmp::Ordering::Equal,
            "this fixture only proves the point if Path::cmp treats the two as equal"
        );

        let ordered = order_locks(None, vec![doubled.clone(), single.clone()]);

        // A byte comparison sees `/` (0x2f) repeat, so `//` sorts before `/a`: the doubled
        // path's next byte after the shared prefix is `/`, the single path's is `a`.
        assert_eq!(ordered, vec![doubled, single]);
    }

    #[test]
    fn with_no_project_lock_the_namespace_locks_alone_are_ordered() {
        let ordered = order_locks(
            None,
            vec![
                PathBuf::from("/project/.typdoc/locks/z.lock"),
                PathBuf::from("/project/.typdoc/locks/a.lock"),
            ],
        );

        assert_eq!(
            ordered,
            vec![
                PathBuf::from("/project/.typdoc/locks/a.lock"),
                PathBuf::from("/project/.typdoc/locks/z.lock"),
            ]
        );
    }

    // ---- done when (c): the git-common project hash ----

    #[test]
    fn two_worktrees_holding_the_project_at_the_same_relative_path_hash_the_same() {
        let project_in_a = Path::new("/home/user/worktree-a/docs/project");
        let project_in_b = Path::new("/srv/checkouts/worktree-b/docs/project");

        let hash_a = project_hash(Path::new("/home/user/worktree-a"), project_in_a)
            .expect("the project is under the worktree root");
        let hash_b = project_hash(Path::new("/srv/checkouts/worktree-b"), project_in_b)
            .expect("the project is under the worktree root");

        assert_eq!(
            hash_a, hash_b,
            "two worktrees of one repository, with the project at the same relative place, \
             must agree on the lock they take even though their absolute paths differ"
        );
    }

    #[test]
    fn a_worktree_holding_the_project_somewhere_else_is_a_different_project_on_purpose() {
        let same_relative_hash = project_hash(
            Path::new("/home/user/worktree-a"),
            Path::new("/home/user/worktree-a/docs/project"),
        )
        .expect("under the root");
        let elsewhere_hash = project_hash(
            Path::new("/home/user/worktree-c"),
            Path::new("/home/user/worktree-c/somewhere/else/project"),
        )
        .expect("under the root");

        assert_ne!(
            same_relative_hash, elsewhere_hash,
            "a worktree keeping the project at a different relative path is a different \
             project under decision 14, and must take a different lock"
        );
    }

    #[test]
    fn a_project_outside_the_worktree_root_has_no_hash() {
        assert_eq!(
            project_hash(Path::new("/a/worktree"), Path::new("/somewhere/else")),
            None
        );
    }

    #[test]
    fn the_hash_is_sixteen_lowercase_hex_characters() {
        let hash = project_hash_of("docs/project");

        assert_eq!(hash.len(), 16);
        assert!(
            hash.chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
        );
    }

    #[test]
    fn the_hash_ignores_a_trailing_separator() {
        assert_eq!(
            project_hash_of("docs/project"),
            project_hash_of("docs/project/")
        );
    }

    #[test]
    fn the_hash_normalizes_backslashes_to_forward_slashes() {
        assert_eq!(
            project_hash_of("docs/project"),
            project_hash_of("docs\\project")
        );
    }

    #[test]
    fn the_hash_is_a_known_value_pinned_by_hand() {
        // SHA-256("docs/project") = 830d630585793d774fa445bae677f79f4dd3fe14102f750678d495feb1ccb6ff,
        // from `sha256sum` (not from this crate's own output); the first 16 hex characters are
        // what decision 14 keeps.
        assert_eq!(project_hash_of("docs/project"), "830d630585793d77");
    }
}
