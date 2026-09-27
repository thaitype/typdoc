# Testing Decisions

Mode: standard. Tests check behaviour from outside the seam: the message `acquire` returns, and
the shipped `Env`'s answers.

- **All five endings on every OS, through a fake `Env`.** `crates/typdoc-fs/tests/namespace_lock_timeout.rs`
  (it needs a real directory, see its header) drives `acquire` with a fake `Env` whose hostname
  and `process_status` the test sets. One test per ending. Each asserts the whole ending text,
  or a part no other ending contains, and asserts that "stale" is absent from endings 1 to 4.
  This replaces the test that passes by accident (`contains("running on this machine")` also
  matches ending 5).
- **The shipped `Env`, for real, under `#[cfg(unix)]`**, in `crates/typdoc` (it owns `libc`):
  - own pid → `Running`;
  - pid 1 as a user that is not root → `Running` (`EPERM`); skipped when running as root;
  - a child that was spawned, waited for and reaped → `NotRunning`;
  - `0` and `u32::MAX` → `Unknown`;
  - hostname is not `unknown-host` and not empty.
  These run on both CI jobs, ubuntu and macOS; macOS is where the bug lived.
- **Planted fault** (verification, not a committed test): map `EPERM` to `NotRunning`, and drop
  the `pid == 0` guard; each must turn a test red.
