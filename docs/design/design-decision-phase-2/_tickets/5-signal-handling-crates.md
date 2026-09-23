# 5: Which crate catches interrupt signals, and what does it guarantee?

Type: wayfinder:research
Status: resolved
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

Full table, per-candidate detail and sources: [research 5](../_research/5-signal-handling-crates.md).

**The facts, as of 2026-09-21.** Five candidates were surveyed: `signal-hook` 0.4.4 (2026-04-04),
`ctrlc` 3.5.2 (2026-02-10), `nix` 0.31.3 (2026-05-11), `signal-hook-registry` 1.4.8 (2025-12-25)
and `libc` 0.2.189 (2026-07-21, with a `1.0.0-alpha.4` prerelease the same day). `simple-signal`
was set aside: its last release is 1.1.1, 2018-03-21. Every version and date here was checked
against the crates.io API a second time after the survey, and they match.

**Against the requirement that the process ends by the signal**, which is the one that decides:

- `signal-hook` provides it as a named function. `low_level::emulate_default_handler` restores the
  default disposition and re-invokes it; `low_level::raise` is there for an explicit reset-then-raise.
  Both exist in 0.4.4, checked on the module's own page, together with `register`, `unregister`,
  `abort`, `exit` and `signal_name`.
- `nix` provides it as two calls: install `SigHandler::SigDfl`, then `raise()`.
- `libc` provides it the same way, one layer lower and entirely by hand.
- `signal-hook-registry` does not; the convenience lives one layer up, in `signal-hook`, which is
  built on it.
- `ctrlc` does not, at any level. Using it means adding `nix` or `libc` for the one behaviour that
  matters here.

**Against the requirement that the cleanup runs off the handler**, because comparing file identity
and unlinking are not async-signal-safe: `signal-hook`'s `Signals` iterator is built for it — the
handler writes a byte, an ordinary thread does the work — and its `flag` module does the same with
an atomic. `ctrlc` runs its closure on a thread it spawns, which is the same property by a
different route. `nix`, `signal-hook-registry` and raw `libc` hand back a raw handler slot and
leave the structure to the caller.

**Against the gap between creating the lock and registering the handler:** every candidate can
register before any lock file exists, because registration takes no lock-related state. The gap is
closed by the order of the calls in the program, not by the choice of crate. That answers the part
of ticket 6 that was about the crate; the rest of that gap is still ticket 6's.

**Platforms.** `nix`'s signal module and the POSIX items of `libc` do not compile on Windows at
all. `signal-hook` and `ctrlc` do compile there, with a reduced signal set. Either way the clear
Windows build failure the design asks for has to come from the program's own `compile_error!`,
not from the dependency.

**Runtime.** None of the candidates needs an async runtime.

**What the standard library gives:** no way to intercept either signal, so a crate is needed for
the interception alone. The termination semantics the design wants are already the default
disposition; the crate buys the chance to run cleanup before it, nothing more.

**Shape of the answer for ticket 6.** On these facts `signal-hook` is the candidate that covers all
three requirements in one dependency, with `nix` the runner-up for a program that already depends
on it, and `ctrlc` ruled out by the re-raise requirement. The choice itself is ticket 6's, together
with the questions this ticket does not answer: what checks that every command reaches a lock
through one path, what the cleanup does when a second signal arrives while it is running, and what
an interrupt leaves behind at each point of a multi-file write.

## Not verified

Nothing was compiled and no signal was sent to any binary. The delivery mechanism behind `Signals`,
the race that `emulate_default_handler` documents and its fall back to `abort`, `ctrlc`'s internal
thread, `nix`'s restriction to a bare `extern "C" fn`, and `libc`'s Windows gating were all read
rather than run. No macOS behaviour was checked at all; macOS support is inferred from each crate's
own claim of POSIX coverage. The crates.io owner lists record publish rights, not who reviews code,
and GitHub's open-issue counts include pull requests; both are used as rough activity signals only.
