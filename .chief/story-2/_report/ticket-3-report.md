# Ticket 3: the namespace lock, and one path to it

Resolved. Commit `c494adf`. Four gates green on the committed tree, each re-run after the commit:
`cargo fmt --check` clean, `cargo clippy --workspace --all-targets -- -D warnings` clean,
`scripts/test.sh` 799 passed / 0 failed / 1 ignored (775 / 0 / 1 before), public-text check clean.

## Outcome

done

## What it does

`crates/typdoc-core/src/namespace_lock.rs` holds the namespace lock. It is not `lock.rs`, which is
`lock.json`, the pin file, and the name keeps the two apart.

`acquire` creates the lock file with `O_EXCL`, making the directory that holds it first, and
retries with backoff until the caller's timeout, then fails with the message naming the path, the
pid, the host and the age. It never takes a lock over, at any age. It returns `NamespaceLock`, a
value with private fields and no public constructor, returned by no other function; `release`
does the identity check — `stat` on the path against `fstat` on the handle held since creation —
before removing, and refuses when they do not match. A `Drop` releases the same way if a caller
forgets, so a lock cannot outlive the scope that took it.

`write_atomically` now takes `&NamespaceLock`, so a write cannot happen under a lock the caller did
not acquire that way. `Fs` and `WriteHandle` gained `identity_at` and `identity`, returning a
`FileId` of device, inode and link count, implemented in `typdoc-fs` and in the fake.

`order_locks` sorts lock paths by their own bytes with the project lock first, which is not the
same as `Path`'s ordering — a fixture with a doubled separator shows the two disagree.
`project_hash` is SHA-256 of the project folder's path relative to the worktree root, normalized to
`/` with no trailing separator, truncated to 16 hexadecimal characters.

## Checked by running

- The compile-fail proof is `trybuild` with four fixtures: constructing the lock outside its module
  does not compile and the recorded reason is the private fields; calling `write_atomically`
  without a lock does not compile and the recorded reason is the missing argument; each has a twin
  that does compile, so a fixture failing for an unrelated reason cannot pass unnoticed. The
  harness was shown able to fail by making the fields public, which turned the failing case into a
  compiling one and the test red; both files were restored.
- The existing write ban reaches the new module: `std::fs::remove_file` planted inside
  `release_checked` was refused as a disallowed method, and removed.
- The directory creation was shown to matter by removing it, which turned the real-filesystem test
  red with `NotFound`, and restoring it.
- A lock another process created is never removed, tested at a fresh age and at a 1990 age; a lock
  taken away and replaced is refused by the identity check, which leaves the replacement untouched.
- The timeout message is tested against a real temporary directory rather than the fake, because
  `acquire` reads a competing lock's content with an ordinary read and a file that exists only in
  the fake is invisible to it.
- The project hash is checked against a worktree holding the project at the same relative path and
  against one holding it elsewhere, which are a different project on purpose.

## Decision

- **Issue:** the design names `--lock-timeout` once, with a default of 5 s, and never says which
  commands accept it or what shape the backoff has; no command takes a lock yet.
- **Options considered:** build the flag and its wiring now; take a `Duration` and leave the flag to
  the first write command; promise a backoff shape in the design.
- **Chosen:** `acquire` takes a `Duration` and nothing wires a flag yet, because which commands
  accept the flag is a question about commands that do not exist. The schedule is 10 ms doubling to
  a 200 ms cap, further capped by the time left to the deadline, and it is not promised anywhere. A
  caller cannot tell a timeout from a lock that was never contended: both end in the same error,
  and nothing reports how long it was held.

## Notes

Threads this ticket opened that a later ticket has to close, none of them left implicit:

- **A hostname has no source yet.** `acquire` takes `host` from its caller and nothing reads the
  machine's own name. The first ticket that acquires a lock for a real command has to get one, and
  `Env` is where the environment and the home directory are already reached.
- **`order_locks` does not canonicalize.** It sorts paths it is given. The first ticket that takes
  two locks at once has to bring each lock's directory to its canonical form before calling it.
- **`NamespaceLock` is not `Send`.** It holds `&dyn Fs` and a boxed handle with no bound, so the
  cleanup thread of ticket 4 cannot call `release` on a lock another thread owns. Ticket 4 chooses
  between adding the bounds and keeping a small registry of what the identity check needs — the
  path and the handle's `FileId` — that the cleanup thread reads under a mutex.
- **Exit 4 has no test through a command** and stays in `UNPRODUCED_EXIT_CODES`; the mapping exists
  in `cli.rs`. Ticket 15 is where it leaves that list.
- **`--lock-timeout` is not wired to any command**, per the decision above.
- `trybuild` is in the stack list. Its recorded errors are the compiler's own words, so a toolchain
  that words them differently needs the two `.stderr` files regenerated; that is what they are for.
- `pid_alive` reads `/proc/<pid>` and answers "not running" where that does not exist. Nothing
  decides a lock's validity by it — it only chooses the wording — so the answer is the safe one.
