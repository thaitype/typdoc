# Ticket 01 report: the real owner check

Fixed point: 613c3f4. Standard mode.

## What changed

- `typdoc-core`: `ProcessStatus` and `Env::process_status`, exported beside `Env`. `acquire`
  takes `env: &dyn Env` and reads the hostname through it; `owner_message` picks one of the five
  endings of SPC-10 in order. `pid_alive` and its `/proc` read are gone.
- `typdoc`: `ProcessEnv` moved to `src/process_env.rs`. On Unix, `gethostname` into a 257-byte
  buffer and `kill(pid, 0)`, with `0` and ids past `i32::MAX` answered `Unknown` before any call;
  elsewhere `unknown-host` and `Unknown`. `libc` is an ordinary dependency; `Cargo.lock` is
  unchanged, since it already listed `libc` for this crate.
- `typdoc-testkit`: `HostEnv`, a fake `Env` whose hostname and process status a test chooses,
  `Unknown` by default. Every call of `acquire` in the tests passes one; every other fake `Env`
  answers `Unknown`.

## Tests

- `crates/typdoc-fs/tests/namespace_lock_timeout.rs`: one test per ending, six in all (the host is
  not known has two causes), each comparing the whole ending. The test that passed because one
  ending's text contains another's is gone.
- `crates/typdoc/tests/process_env.rs` (Unix): this process is running; process 1 is running as a
  user that is not root (skipped as root); a spawned and reaped child is not running; `0`,
  `u32::MAX` and `i32::MAX + 1` are unknown; the hostname is not the placeholder.
- 1127 → 1135 tests.

## Planted faults, each red and then restored byte for byte

- `EPERM` read as `NotRunning`: the process-1 test.
- The `pid == 0` guard removed: the test of ids no process can have.
- `ESRCH` read as `Unknown`: the reaped-child test.
- A recorded `unknown-host` no longer checked: the test of the owner's unknown host.
- `Running` given the stale ending: the running and cannot-tell tests.

## Decided where the ticket left it open

- The hostname is read once per `acquire`, not once per command: a command that takes several
  locks asks the system once per lock, a system call each.
- The reaped child is the typdoc binary run with `--version`, through the CLI spawn helper, since
  the lints allow a process to be started only there.

## Known limits

- The reaped-child test assumes the system does not give the child's id to a new process between
  the reaping and the check. Not verified beyond runs on this machine.
