# Contract

The behaviour is `docs/design/spec/SPC-10.md` (When a lock is not acquired). This contract fixes
the shapes the code takes and the choices the spec leaves open. When the two disagree, the build
stops and the disagreement is reported; neither is bent to fit the other.

## The seam

- `typdoc-core` gets no `libc` and no `cfg` on the operating system. Liveness joins the hostname
  behind `Env` (`crates/typdoc-core/src/env.rs`):

  ```rust
  pub enum ProcessStatus { Running, NotRunning, Unknown }
  fn process_status(&self, pid: u32) -> ProcessStatus;
  ```

  `ProcessStatus` is exported from `typdoc-core` beside `Env`.
- `hostname()` keeps its signature and its placeholder: a name that cannot be read is
  `unknown-host`. The lock file's shape (`pid`, `host`, `timestamp`) does not change.
- `namespace_lock::acquire` takes `env: &dyn Env` in place of `host: &str`, and reads the
  hostname and the owner's status through it. Its two callers in `project.rs` pass `deps.env`.
  `pid_alive` and every read of `/proc` in the lock path are removed.
- Every existing `impl Env` (test fakes in `typdoc-core`, `typdoc`) gains `process_status`.
  A fake that does not care returns `Unknown`, never `NotRunning`.

## Choosing the ending (`owner_message`)

In this order, the first that applies (`SPC-10` cases 1 to 5):

1. our hostname is `unknown-host`, or the recorded host is `unknown-host` → host not known;
2. the recorded host differs from ours → another host;
3. `process_status(pid)` is `Unknown` → cannot be checked here;
4. `Running` → wait;
5. `NotRunning` → stale, and the path is shown.

The ending is one of these fixed texts (the part after `(age …); `). Endings 2, 4 and 5 are
today's, unchanged:

1. `its host is not known, so it cannot be checked from here: delete it only once you know that
   process has stopped`
2. `it is on another host and cannot be checked from here: delete it only once you know that
   process has stopped`
3. `it is on this machine, but whether it is running cannot be checked here: delete it only once
   you know that process has stopped`
4. `it is running on this machine: wait, or run again with a longer --lock-timeout`
5. `it is not running on this machine: the lock is stale, delete <path>`

The lead (`lock not acquired: <path> is held by pid <pid> on <host> (age <age>); `) and the
message when the owner cannot be read do not change.

## The shipped `Env` (`crates/typdoc/src/main.rs`)

One code path for every Unix, behind `#[cfg(unix)]`; `#[cfg(not(unix))]` returns
`unknown-host` and `Unknown`.

`ProcessEnv` moves out of `main.rs` into the library, `crates/typdoc/src/process_env.rs`
(`pub struct ProcessEnv`), so that tests in `crates/typdoc/tests/` reach the shipped answers;
`main.rs` uses it from there.

- **Hostname**: `libc::gethostname` into a 257-byte buffer (the longest name Linux or macOS
  gives is 255 or 256 bytes). A call that fails, a buffer with no NUL, an empty name or a name
  that is not UTF-8 is `unknown-host`. The name is trimmed of surrounding whitespace, as today.
- **Liveness**: `libc::kill(pid, 0)`.
  - `pid` is `0` or larger than `i32::MAX` → `Unknown`, without calling `kill`. Measured on
    Linux: `kill(0, 0)` signals the caller's own process group and `kill(-1, 0)` every process
    it may signal, and both succeed, so `u32::MAX` cast to `pid_t` would read as `Running`.
  - returns 0 → `Running`; fails with `EPERM` → `Running` (another user's process);
    fails with `ESRCH` → `NotRunning`; any other failure → `Unknown`.
- `libc` moves from `[dev-dependencies]` to `[dependencies]` of `crates/typdoc`, same version
  (0.2.189). No new crate enters `Cargo.lock`.
- The comments that say `/proc` or "Linux only" (`namespace_lock.rs`, `main.rs`) go.

## What does not change

`acquire`'s retry, backoff and timeout; the lock file; release; the interrupt registry; lock
order. A test that is not about the message is not edited except to pass `env` to `acquire`.

## Documentation in the same PR

- `CHANGELOG.md` `## [Unreleased]` (the heading is added above `[0.4.0]`):
  - Fixed: on macOS a lock held by a running process was reported as stale; the two new endings.
  - Changed, for the `typdoc-core` library: `acquire` takes `env: &dyn Env` in place of
    `host: &str`, and `Env` has a new required method, `process_status`, with its return type
    `ProcessStatus`. A caller of `acquire` or an implementer of `Env` must change.
- `templates/skills/typdoc/references/exit-codes.md` (then `scripts/render_skills.py`) and any
  user doc that lists the endings: all five.
- `.chief/project.md`: `libc` is an ordinary dependency of the binary crate, and why.

## Decided

- **`follows` on SPC-10.** None of PRN-1..10 is about never guessing the unsafe answer where the
  answer cannot be known. Decided: SPC-10 carries no `follows` in this story, and the gap is
  recorded here; a principle for it is a separate decision.
- **The `unknown-host` placeholder.** A machine actually named `unknown-host` always gets ending 1.
  Decided: accepted, since it errs to the cautious side and keeps the lock file's shape.
