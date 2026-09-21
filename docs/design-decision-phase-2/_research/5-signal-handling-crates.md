# Signal handling for a lock-owning sync CLI: crate choice

Written 2026-09-21. Version, date and ownership data comes from live queries against the
crates.io and GitHub APIs; API descriptions come from the docs.rs pages, summarized as they
were fetched rather than read byte for byte. Anything not covered that way is listed under
`## Not verified`.

Every version number and publish date in the table below was checked a second time against the
crates.io API, and so was the existence of the functions the recommendation rests on:
`signal_hook::low_level` in 0.4.4 exposes `emulate_default_handler`, `raise`, `register`,
`unregister`, `abort`, `exit` and `signal_name`, and the crate's default features are `channel`
and `iterator`, so `Signals` needs no feature change. The behaviour of any of them was not run.

## 1. Question and constraints

The question: which crate should a synchronous Rust 2024 CLI use to catch `SIGINT` and
`SIGTERM`, so that on receipt it can remove a lock file it created and then terminate *by that
signal* (so a parent process sees "killed by signal", not a normal exit code).

Constraints the candidates are judged against:

- No async runtime anywhere in the core library; the CLI itself is sync only.
- The lock file is created with `O_EXCL`, holding pid/hostname/timestamp; before removal its
  identity is checked with `stat` on the path against `fstat` on the held file descriptor. That
  comparison, and the `unlink`, are not async-signal-safe operations, so they cannot run inside
  the raw OS signal handler — they must run on an ordinary thread, either woken by the handler
  or gated behind a flag the handler sets.
- After cleanup, the process must end by the original signal, not `exit()` with a made-up code.
  This means: restore the signal's default disposition and re-raise it (or equivalent), and the
  crate's API must not make that impossible.
- There is a window between creating the lock file and registering the cleanup handler for it; a
  signal landing in that window must not leak a lock. This is judged as: can the handler be
  registered *before* the lock file is created (independent of which lock exists yet), so that by
  the time `O_EXCL` succeeds, delivery is already being intercepted.
- Target platforms: Linux is supported now; macOS is written for but not verified working;
  Windows must fail to build with a clear message rather than compile into something broken.
- Testing is done by sending a real signal to a shipped binary; no test-only code path in the
  binary is allowed, so a candidate needs to work through the same public API the release binary
  uses.

## 2. Candidate table

| Crate | Latest version / publish date | Maintenance signals | In the raw OS handler | On a normal thread | Restore-default + re-raise | Runtime-free | Platforms | Unsafe exposure |
|---|---|---|---|---|---|---|---|---|
| `signal-hook` | `0.4.4`, published 2026-04-04 | 2 owners on crates.io (`vorner`, `rust-bus:maintainers` team); GitHub `vorner/signal-hook` last push 2026-04-04, last repo activity 2026-09-14, 868 stars, 26 open issues, not archived | Only an atomic flag flip (`flag` module) or a byte written down a self-pipe (`iterator`/`Signals`) | Everything else: reading the flag or draining the `Signals` iterator on a thread you own or control, including all of the stat/fstat/unlink work | Yes — `signal_hook::low_level::emulate_default_handler()` resets the signal to `SIG_DFL` and re-invokes it, and `low_level::raise()` is available for a manual reset-then-raise | Yes, no runtime dependency | Unix (POSIX.1-2001) fully; Windows compiles with reduced signal coverage (a handful of signals only, documented race caveats) | Safe public API (`flag`, `iterator`); `low_level` module carries the few remaining unsafe/advanced entry points |
| `ctrlc` | `3.5.2`, published 2026-02-10 | 1 owner on crates.io (`Detegr`); GitHub `Detegr/rust-ctrlc` last push 2026-07-22, last repo activity 2026-09-17, 669 stars, 15 open issues, not archived | Nothing application-level — the crate's own internal handler only signals a dedicated background thread it spawns | The registered closure itself runs on that dedicated thread, not in raw handler context, so the stat/fstat/unlink work is fine there | Not provided by the crate. No `emulate_default_handler`/re-raise helper exists in its public API. A caller can still do it manually (`libc`/`nix` `SIG_DFL` + `raise()`) from inside the closure, since the closure already runs off a normal thread — but that means pulling in a second crate for the one piece of behavior this task actually needs | Yes, no runtime dependency | Linux, macOS, Windows all supported (Windows path uses `windows-sys`, Unix path is layered on `nix`); `SIGTERM`/`SIGHUP` require the `termination` Cargo feature | Public API itself exposes no unsafe items |
| `nix` (`nix::sys::signal::sigaction`) | `0.31.3`, published 2026-05-11 | 2 owners on crates.io (`carllerche`, `nix-rust:nix-maintainers` team); GitHub `nix-rust/nix` last push 2026-05-19, last repo activity 2026-09-20, 3077 stars, 359 open issues, not archived | Whatever the caller writes into the `extern "C" fn(c_int)` installed via `SigHandler::Handler` — entirely the caller's own code, entirely subject to async-signal-safety rules the crate does not enforce | Everything outside that raw handler function — same pattern as the other candidates: flag/pipe in the handler, real work elsewhere | Yes — `SigHandler::SigDfl` reinstalls the default disposition via `sigaction()`, and `raise()` sends the signal to the current thread; both are exposed directly | Yes, no runtime dependency | Documented "Available on Unix only" — the `sys::signal` module does not compile on Windows at all | `sigaction()` and `signal()` are both `unsafe fn`; the handler itself must be a raw `extern "C"` function pointer (no closures), so all signal-safety bookkeeping is manual |
| `signal-hook-registry` | `1.4.8`, published 2025-12-25 | Same ownership as `signal-hook` (2 owners: `vorner`, `rust-bus:maintainers`); same repository (`vorner/signal-hook` is a Cargo workspace covering both crates), same activity dates as above | The registered closure (`Fn() + Sync + Send + 'static`) runs directly inside the real OS handler, dispatched through the crate's internal trampoline; it is fully subject to async-signal-safety rules, which the docs spell out explicitly (no non-async-signal-safe OS calls, no unsynchronized globals beyond atomics, must not panic) | Nothing built in — this crate only manages *which signal fires which registered closure*; anything past that (like "later, on a real thread, do the fstat/unlink") is the caller's own architecture, same as raw `sigaction` | Not provided directly. This crate is the low-level engine `signal-hook` itself is built on top of (`emulate_default_handler` lives one layer up, in `signal-hook`, not here); a caller could still hand-roll reset+raise via `libc` | Yes, no runtime dependency | Same repository/support envelope as `signal-hook`: Unix fully, Windows via CRT `signal`/`raise` with documented races and no `siginfo_t` support | `register()`/`unregister()` are `unsafe fn`; soundness is the caller's responsibility for whatever closure is registered |
| `libc` (raw `sigaction` FFI) | stable line `0.2.189`, published 2026-07-21; also publishing a `1.0.0-alpha.4` prerelease, published 2026-07-21 | 1 owner shown on crates.io (`rust-lang-owner`, the org's publishing account — actual maintenance is the rust-lang Libs team); GitHub `rust-lang/libc` pushed 2026-09-21 (same day as this note), 2609 stars, 162 open issues, not archived — the most actively pushed repository of the group | Whatever raw C code the caller writes and installs via a hand-built `libc::sigaction` struct — no Rust-side help at all | Everything else, entirely the caller's own design | Possible, entirely manual: reinstall `SIG_DFL` via `sigaction()`, then call `libc::raise()` | Yes, no runtime dependency (it is the bindings layer everything else above is ultimately built on) | Cross-platform bindings crate; the POSIX-only items (`sigaction`, `SIGTERM`, etc.) are only present under Unix `cfg`s, so code calling them simply does not compile on a Windows target | `sigaction()` is `pub unsafe extern "C" fn`; the caller constructs and interprets the raw C struct directly — the largest unsafe surface of any candidate here |

An additional crate turned up in the same space, `simple-signal`: crates.io shows its latest
version as `1.1.1`, last published 2018-03-21. That is old enough (no publish in roughly eight
years relative to this note's date) to set aside without a deeper look — it is not treated as a
serious candidate.

## 3. Registration-before-creation and re-raise, per candidate

- **`signal-hook`**: registration (`flag::register(...)` or `Signals::new(...)`) is a plain
  function call that can run as the very first thing in `main`, before any lock file exists —
  nothing in the API ties registration to any later state. Re-raise: yes, directly supported via
  `low_level::emulate_default_handler()` (reset-to-default-and-reinvoke, described as
  async-signal-safe with a documented short race window on terminating signals, falling back to
  `abort` if the race is lost) or a manual `low_level::raise()` after `unregister`ing.

- **`ctrlc`**: `set_handler()` can also run first, before the lock file exists, since it takes no
  lock-related state — the closure just needs to consult whatever shared cell later holds the
  lock's cleanup information. Re-raise: not offered by the crate's own API; achievable only by
  reaching for another crate's raw signal call inside the closure.

- **`nix`**: `sigaction()` can be called first thing in `main`, same reasoning. Re-raise: yes,
  directly — install `SigHandler::SigDfl` then call `nix::sys::signal::raise()`, both exposed
  functions in the same module.

- **`signal-hook-registry`**: `register()` can be called first thing in `main`. Re-raise: not
  provided by this crate directly (that convenience lives in `signal-hook` itself); would need a
  manual `SIG_DFL` + `raise()` step via `libc` or `nix`, which most projects would rather get from
  `signal-hook`'s `low_level` module for free since it is built on this crate anyway.

- **`libc`**: `sigaction()` can be called first thing in `main`. Re-raise: yes, manually —
  `sigaction()` to install `SIG_DFL`, then `libc::raise()`.

For every candidate the "handler registered before lock creation" property held, so none of them
force the leak-in-the-gap failure mode by themselves; the gap is closed by *ordering the calls*
in the CLI's own `main`, not by anything a particular crate does or does not provide.

## 4. What the standard library alone gives

Rust's standard library does not expose `sigaction`, `signal()`, or any other POSIX signal
registration API. There is no built-in way, using only `std`, to intercept `SIGINT`/`SIGTERM`
before the process dies, and therefore no way with `std` alone to run cleanup first.

What `std` does give, without any extra crate, is the end state this task wants by default: an
unhandled `SIGINT`/`SIGTERM` already terminates a process by that signal (the OS default
disposition applies), which is exactly the "caller sees killed by signal, not an exit code"
outcome — the entire reason a crate is needed here is only to get a chance to run cleanup code
*before* that default termination happens, not to get the termination semantics themselves.
`std::os::unix::process::ExitStatusExt::signal()` exists on the *parent* side (reading how a
*child* process died), which is unrelated to a process determining its own termination.

## 5. Recommendation

Recommended: `signal-hook` (default features — `channel` and `iterator` are both on by default,
so no feature-flag changes are needed to use `Signals`), used as: register a `Signals` iterator
for `SIGINT`/`SIGTERM` before creating any lock file, hand it to a dedicated thread that blocks
in the iterator (an ordinary thread — the fstat/stat/unlink identity check and removal run there,
not inside handler context), and on receipt of a signal call `low_level::emulate_default_handler`
(or an explicit `SIG_DFL`-then-raise sequence via the same `low_level` module) after cleanup
finishes, so the process ends by the original signal.

Reasoning: it is the only candidate that supplies, in one dependency, (a) an async-signal-safe
handler body with zero user-written unsafe code in the common path, (b) a documented, off-the-
shelf way to move the real work to a normal thread (`Signals` iterator over a self-pipe-style
mechanism, not something reimplemented by hand), and (c) a documented, purpose-built primitive
for the exact "restore default and re-raise" requirement (`emulate_default_handler`), instead of
that behavior being left as an exercise via raw FFI. Ownership is spread across an individual and
a team account rather than a single maintainer, and the repository is actively pushed to
(2026-04-04) and has recent overall repository activity (2026-09-14) relative to this note's
date. On Windows, `signal-hook` compiles with a reduced, documented signal set rather than
failing outright, so the "fail to build with a clear message" requirement for Windows needs to be
enforced by the project itself (an explicit `#[cfg(windows)] compile_error!(...)` at the crate
root, or an equivalent build-time check) — that is true of every candidate here except `nix` and
raw `libc`, which fail to build on Windows on their own by lacking the POSIX symbols entirely,
just not with a message a CLI's own users would find clear without the project adding one anyway.

Runner-up: `nix` (`nix::sys::signal`, using `sigaction()` directly). It gives the same
registration-before-creation freedom and the same explicit `SigDfl` + `raise()` re-raise path,
with comparable maintenance signals (team + individual ownership, active repository). The
difference from the recommendation is architectural rather than a maintenance gap: `nix` hands
back a raw `extern "C" fn(c_int)` slot with no multiplexing help and no built-in path to move work
off the handler (that "flag now, work later" structure has to be built by hand, the same way it
would with raw `libc`), and its default-disposition-then-re-raise sequence has to be assembled
from two separate calls rather than used as a single named primitive. It is a reasonable choice
if the project already depends on `nix` for other OS-level work and would rather avoid adding
`signal-hook` as a second signal-handling dependency; `nix::sys::signal` does not compile on
Windows at all, which — unlike `signal-hook` — needs no project-side gate to keep Windows off the
POSIX code path, though a project-added `compile_error!` is still worth it for a readable message
instead of a raw "item not found" error.

`ctrlc` is not recommended for this task: its closure already runs off a normal thread (a genuine
strength for the cleanup work), but the crate supplies no restore-default/re-raise primitive at
all, which the constraints treat as disqualifying on its own — using it would mean reaching for
`libc` or `nix` regardless, at which point that crate may as well be the primary dependency.
`signal-hook-registry` and raw `libc` are the layers `signal-hook` and `nix` are respectively
built on; either is usable directly but means re-deriving, by hand, safety and ordering guarantees
that the two recommended candidates already give as part of their public API.

## Not verified

- The internal delivery mechanism of `signal-hook`'s `Signals` iterator (self-pipe vs. `signalfd`
  vs. some other primitive) was not confirmed against source; docs.rs fetches for that specific
  page did not return the implementation detail, only that registration happens through `Signals::new`
  and that iteration happens on whatever thread calls it.
- The exact wording and full text of `signal-hook`'s crate-level README (self-pipe trick,
  async-signal-safety discussion) was not retrieved in full; only a partial fetch came back.
- `ctrlc`'s internal use of a dedicated background thread, and its layering on `nix` for the Unix
  backend, is taken from its docs.rs page and Cargo dependency listing as fetched, not from
  reading its source directly.
- The precise race-condition mechanics of `signal_hook::low_level::emulate_default_handler`
  (exactly when it falls back to `abort`, and under what conditions the "short race condition on
  terminating signals" manifests) is taken from the function's own doc text as fetched and
  summarized, not from reading the implementation.
- `libc`'s cross-platform `cfg` gating (that `sigaction`/`SIGTERM`/etc. are unix-only items so
  that code referencing them fails to compile, rather than compiling to something broken, on a
  Windows target) is stated from general knowledge of the crate's structure, not confirmed here
  by inspecting its source or by an actual Windows build attempt.
- No crate in this note was added as a dependency, compiled, or run. No signal was actually sent
  to a test binary. No macOS-specific behavior was checked; macOS support is inferred from POSIX
  API coverage claims in each crate's own docs, not tested.
- Owner listings from the crates.io `/owners` endpoint reflect publish rights, not necessarily
  everyone who reviews or contributes code; they are used here only as a maintenance signal, not
  as a complete contributor count.
- GitHub `open_issues_count` includes pull requests in GitHubs's counting; it is used only as a
  rough activity signal, not a precise defect count.
- Whether `nix`'s `SigHandler::Handler` variant truly forbids closures (accepting only a bare
  `extern "C" fn(c_int)`, with no captured state) is taken from the fetched docs.rs summary of
  that module, not from reading the type definition directly.
