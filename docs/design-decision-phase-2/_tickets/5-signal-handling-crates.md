# 5: Which crate catches interrupt signals, and what does it guarantee?

Type: wayfinder:research
Status: claimed
Blocked by: None (can start immediately)

## Question

The design says: "typdoc removes its own lock on normal completion, on error, and on the system's interrupt signals (on POSIX, SIGINT and SIGTERM)", and, under Exit codes: "When typdoc is interrupted (SIGINT or SIGTERM on POSIX) it removes its own locks and then ends by that signal, so the caller sees a process killed by a signal and no code from this table, and no error object is written."

No crate is chosen for it. Gather the facts a choice needs, without making the choice:

- The candidates, with their latest versions and publish dates and what maintenance signals are visible. Cover at least `signal-hook`, `ctrlc`, `signal-hook-registry`, `nix` used for raw `sigaction`, and `libc` used directly.
- For each: what runs inside the handler and what runs on a normal thread. This matters because the cleanup is not async-signal-safe work — before removing a lock, typdoc compares the file identity the system provides, `stat` on the path against `fstat` on the handle it has held since creation.
- For each: whether the default handler can be restored and the signal re-raised, so the caller sees a signal death rather than an exit code. That is a requirement, not a preference; a crate that can only exit is unsuitable.
- For each: whether the handler can be registered before the lock file is created, so that a signal in the gap between creation and registration cannot leak a lock.
- For each: whether it needs an async runtime. `typdoc-core` stays runtime-free.
- Platform support: Linux is supported, macOS is written for and not yet supported, and the Windows build must fail with a message rather than silently do nothing.
- What the standard library alone gives and does not give here.
- What unsafe code each candidate exposes the program to, since `typdoc-core` bans panics in non-test code and any unsafe here is in the one path that runs while the program is being torn down.

Record what was not run separately from what was read.

## Answer

<filled in on resolve>
