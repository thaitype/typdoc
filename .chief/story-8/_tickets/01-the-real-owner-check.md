# 01: The real owner check

Type: implementation
Status: open
Blocked by: none

## What this delivers

A lock timeout on Linux or macOS names the owner's state as the system reports it, and says a
lock is stale only when the system answered that no process has its id.

## Scope (contract: The seam; Choosing the ending; The shipped `Env`; What does not change)

- `ProcessStatus` and `Env::process_status` in `typdoc-core`, exported beside `Env`.
- `acquire` takes `env: &dyn Env`; `owner_message` picks one of the five endings in contract
  order; `pid_alive` and the `/proc` reads go.
- `ProcessEnv` in `crates/typdoc/src/process_env.rs`: `gethostname`, `kill(pid, 0)` with the
  `0` / `> i32::MAX` guard, `cfg(not(unix))` fallback; `libc` into `[dependencies]`.
- Every existing `impl Env` gains `process_status`.

## Checks

Gates; the tests in `testing-decisions.md`; planted faults (`EPERM` read as `NotRunning`, the
`pid == 0` guard removed) each turn a test red.
