# Ticket 14: two writers that take the same lock

Resolved. Commit `3a6444c`, merged `ec9b4c2` with no conflicts. `cargo fmt --check` clean,
`cargo clippy --workspace --all-targets -- -D warnings` clean, public-text check clean (after a
wording fix to an earlier report, caught on this merge and committed separately), all re-run on the
merged tree. `scripts/test.sh` with `TMPDIR` on a disk-backed folder: 979 passed / 0 failed / 1
ignored (975 before this ticket).

## Outcome

done

## What it does

The goal's second criterion: the collision is made to happen, not hoped absent.

- **Deterministic, through the seam** (`crates/typdoc-core/tests/allocation_race.rs`): two threads
  call `new_document` against one real temporary project, backed by `typdoc_fs::SystemFs` — needed
  because `state::read` bypasses the injected `Fs` entirely, so a pure in-memory fake would be
  invisible to it. A wrapper `Fs` pauses each allocation's one `exists()` call inside the allocation
  window via channel gates, with a second channel firing the instant one thread's own lock creation
  is refused because the other still holds it — hard evidence of a real mid-window meeting, no sleeps
  anywhere.
- **Across real processes** (`crates/typdoc/tests/lock_contention.rs`): a separate uncoded `new`
  holds the lock via ticket 4's own scan-under-lock mechanism, chosen deliberately so it allocates no
  key and cannot be confused with the `n` contenders. Three `new` processes are then started with a
  120 s `--lock-timeout`; the holder is released only once every contender is confirmed both alive
  and still meeting a lock file that names the holder's own pid — a fail-fast ceiling bounds the wait,
  never proves it.
- **The `git-common` carve-out** (`crates/typdoc-core/tests/git_common_duplicate.rs`): `LockMode::
  GitCommon` is refused outright by every write command today, verified by reading the code rather
  than assumed, so no write command can exercise it. The narrower, buildable form of the design's own
  claim is built instead — two ordinary projects allocate the same key with nothing coordinating
  them, and a third, hand-assembled project demonstrates `validate` reporting `keys.unique` once both
  are visible together — stated plainly as narrower, not silently passed off as the wider claim.

## Checked by running

- All four new tests pass on their own, including the real-process one at its actual ~7.7s
  wall-clock time.
- The story's own correctness claim was shown provable, not merely asserted: `new_coded`'s fresh
  `state::read` was replaced with the pre-lock, potentially stale snapshot the function's own comment
  names as the danger, planted once. Both the deterministic and the real-process test went red — the
  deterministic one panicking with the exact collision (`AlreadyExists`, "the key `WF-1` was just
  allocated and should not be reachable"), the real-process one tripping its own "every contender must
  succeed" assertion the same way. Verified independently by re-planting and re-observing the same red
  panic text before this report was written, then reverted; `git diff` confirmed the revert was exact.
- The existing write ban reaches the new test code: `std::fs::write` planted inside a test helper was
  refused; the same call inside `typdoc-fs` was not, confirming the crate boundary. The original setup
  helpers were also caught this way before the deliberate plant, and fixed to go through `Fs`-trait
  methods instead of a raw concrete path.

## Notes

- `typdoc-fs` became a dev-dependency of `typdoc-core`, needed for the two tests that require a real
  `Fs` backed by real disk rather than the in-memory fake.
- `crates/typdoc/tests/signals.rs`'s large-project/lock-polling helpers moved into the shared
  `crates/typdoc/tests/common/mod.rs`, exactly the reuse ticket 4's own report anticipated.
- The `git-common` test's narrower scope is a real, stated limit, not a gap silently left: nothing on
  this branch wires a write command to that lock mode yet, so the collision the design describes
  cannot be produced through one. Whichever ticket first wires `git-common` inherits proving the wider
  claim.
