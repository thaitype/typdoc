//! Namespace and project locks. Not `lock.rs`, which reads `.typdoc/lock.json`, the pin file.
//!
//! [`acquire`] is the only way to get a [`NamespaceLock`], and every function that writes takes
//! one, so a write without a lock does not compile (SPC-10; proved in
//! `tests/namespace_lock_compile_fail.rs`).

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

/// A namespace's lock file in the `local` lock mode.
pub fn local_namespace_lock_path(project_root: &Path, namespace: &str) -> PathBuf {
    project_root
        .join(".typdoc/locks")
        .join(format!("{namespace}.lock"))
}

/// `.typdoc/locks/.project.lock`: a name that starts with `.`, so no namespace can have it.
pub fn local_project_lock_path(project_root: &Path) -> PathBuf {
    project_root.join(".typdoc/locks/.project.lock")
}

/// A namespace's lock file in the `git-common` lock mode. `git_common_dir` must already be
/// canonicalized: this crate cannot run `git`.
pub fn git_common_namespace_lock_path(
    git_common_dir: &Path,
    project_hash: &str,
    namespace: &str,
) -> PathBuf {
    git_common_dir
        .join("typdoc")
        .join(format!("{project_hash}-{namespace}.lock"))
}

/// The project lock file in the `git-common` lock mode.
pub fn git_common_project_lock_path(git_common_dir: &Path, project_hash: &str) -> PathBuf {
    git_common_dir
        .join("typdoc")
        .join(format!("{project_hash}.lock"))
}

/// The `<project-hash>` of a project folder's path relative to its worktree root. Changing how
/// it is computed is a breaking change: a new binary would not see a lock an old one holds.
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

/// The [`project_hash_of`] `project_folder`'s path relative to `worktree_root`, or `None` when
/// it is not under it.
pub fn project_hash(worktree_root: &Path, project_folder: &Path) -> Option<String> {
    let relative = project_folder.strip_prefix(worktree_root).ok()?;
    let joined = relative
        .components()
        .map(|part| part.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/");
    Some(project_hash_of(&joined))
}

/// Orders lock files as SPC-10 requires: the project lock first, then the namespace locks by
/// the bytes of their paths. Not [`Path`]'s own ordering, which compares components: `a//b` and
/// `a/b` are equal to it and different here.
pub fn order_locks(
    project_lock: Option<PathBuf>,
    mut namespace_locks: Vec<PathBuf>,
) -> Vec<PathBuf> {
    namespace_locks.sort_by_key(|a| path_bytes(a));
    let mut ordered = Vec::with_capacity(namespace_locks.len() + 1);
    ordered.extend(project_lock);
    ordered.extend(namespace_locks);
    ordered
}

/// Owned: Windows has no byte view of a path, so one has to be built.
fn path_bytes(path: &Path) -> Vec<u8> {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        path.as_os_str().as_bytes().to_vec()
    }
    #[cfg(not(unix))]
    {
        path.as_os_str().to_string_lossy().into_owned().into_bytes()
    }
}

#[derive(Debug, Clone, Deserialize)]
struct Owner {
    pid: u32,
    host: String,
    timestamp: String,
}

#[derive(Debug, Clone, Serialize)]
struct Stamp<'a> {
    pid: u32,
    host: &'a str,
    timestamp: String,
}

#[derive(Debug, Clone)]
struct Registered {
    id: u64,
    path: PathBuf,
    expected: FileId,
}

/// Every lock this process holds, for the cleanup an interrupt starts on another thread.
/// [`NamespaceLock`] is not `Send` (it holds `&dyn Fs` and a boxed handle), so that thread
/// cannot call [`release`]; it gets the path and the identity [`acquire`] read from the handle.
///
/// The cached identity is as good as a fresh `fstat`: while this process keeps the handle open,
/// the kernel does not free the inode, so its device and inode cannot change or be reused.
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

/// Removes every lock this process still holds, for the cleanup that runs on `SIGINT` or
/// `SIGTERM` (SPC-10). A lock whose path no longer shows the identity [`acquire`] recorded is
/// left alone, as [`release`] leaves it.
///
/// Entries stay in the registry: the process ends by the signal right after this runs.
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

fn identity_matches(held: FileId, current: Option<FileId>) -> bool {
    held.links > 0 && current.is_some_and(|id| id.device == held.device && id.inode == held.inode)
}

/// A held lock, released when this value is dropped (SPC-10).
///
/// It keeps the lock file open until release, so [`release`] can tell its own file from
/// whatever the path leads to by then: `stat` on the path against `fstat` on the handle.
pub struct NamespaceLock<'a> {
    fs: &'a dyn Fs,
    path: PathBuf,
    handle: Box<dyn crate::fs::WriteHandle>,
    released: bool,
    registry_id: u64,
}

impl NamespaceLock<'_> {
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// `fs` and `handle` have no `Debug`, and the path says which lock this is.
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
    /// The path no longer names this handle's file, or that file has no link left, so nothing
    /// was removed: the lock was taken away during the operation, and removing what is there now
    /// would take over another process's lock (SPC-10).
    TakenByAnother,
}

impl Drop for NamespaceLock<'_> {
    fn drop(&mut self) {
        if !self.released {
            // The same check as `release`, silently: a drop has nowhere to report to (SPC-10).
            let _ = release_checked(self.fs, &self.path, self.handle.as_ref());
        }
        // Always, so the signal cleanup never sees a lock that is gone.
        deregister(self.registry_id);
    }
}

/// Releases `lock`, removing the lock file only if it is still this lock's own file.
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

/// Creates `path` with `O_EXCL`, retrying with backoff until `timeout`, then fails with
/// [`Error::LockTimeout`]. A lock another process created is never removed or taken over,
/// whatever its age (SPC-10).
///
/// `host` is stamped into the lock file, and compared with a competing lock's host to word the
/// timeout message.
///
/// The folder that holds `path` is created first (SPC-10), so the first lock a project takes
/// does not fail on a missing `.typdoc/locks/`.
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
                // Registered at once: an interrupt before this line leaves a lock file nothing
                // names, the window SPC-10 leaves open.
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

/// The competing lock may have gone by the time it is read; the message then names only the
/// path.
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

/// This only chooses the wording of the timeout message; it never decides whether a lock is
/// valid (SPC-10). `/proc` is Linux's: elsewhere every process reads as not running.
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

// `acquire`, `release` and the timeout message are tested in `tests/namespace_lock.rs`: they
// need `typdoc_testkit::fake`, which depends on this crate, and in a `#[cfg(test)]` module here
// its `Fs` and `Clock` would come from a different build of `typdoc_core` and not unify.

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    // The registry is tested here, not through the fake: the fake never repeats an identity, so
    // from outside a leaked entry and a removed one leave the same files behind.

    fn registry_len() -> usize {
        REGISTRY
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .len()
    }

    /// `REGISTRY` is shared by tests running at the same time; each test that counts entries
    /// holds this.
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
            // Reached: the drop finds its identity still matching.
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

    /// Built by hand, which only this module can do: the fields are private to it (SPC-10).
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
        let doubled = PathBuf::from("/project/.typdoc/locks//a.lock");
        let single = PathBuf::from("/project/.typdoc/locks/a.lock");
        assert_eq!(
            doubled.cmp(&single),
            std::cmp::Ordering::Equal,
            "this fixture only proves the point if Path::cmp treats the two as equal"
        );

        let ordered = order_locks(None, vec![doubled.clone(), single.clone()]);

        // `//` sorts before `/a`: `/` (0x2f) is below `a`.
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
        // From `sha256sum`, not from this crate: SHA-256("docs/project") =
        // 830d630585793d774fa445bae677f79f4dd3fe14102f750678d495feb1ccb6ff.
        assert_eq!(project_hash_of("docs/project"), "830d630585793d77");
    }
}
