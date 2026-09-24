# Ticket 27 Report

## Ticket
`lock_contention.rs`'s `n_processes_racing_one_lock_issue_n_distinct_keys_with_no_document_overwritten`
raced its own holder-pid assertion — `wait_for_file` only proved the lock file existed, not that
the holder had finished writing its pid into it — causing an occasional false CI failure that had
previously been misattributed to machine load.

## Outcome
done

## The fix
Added `wait_until_lock_names_pid(lock, pid)` in `crates/typdoc/tests/lock_contention.rs`: a
bounded poll (2ms sleep in a loop, same shape as the file's existing
`wait_until_all_contenders_are_observed_waiting`) that keeps calling `lock_owner_pid(&lock)` until
it returns `Some(holder_pid)`, panicking with a clear message (naming the lock path, the pid
looked for, and the last value read) if `LOCK_APPEARS_WITHIN` (20s — already generous next to how
fast a real pid write finishes right after file creation) elapses first.

Replaced the racy `wait_for_file(&lock, ...)` immediately followed by a one-shot
`assert_eq!(lock_owner_pid(&lock), Some(holder_pid), ...)` with a call to this new helper. The
`wait_for_file` existence check right before it is untouched — it still proves the file appeared
at all, distinctly from the new poll proving its *contents* are fully written.

This closes the race without weakening the assertion: the test still requires the lock to name the
holder's own pid, and still fails outright (not silently, not by retrying forever) if that never
becomes true within a generous, bounded timeout — a genuine regression (wrong pid, lock never
written, etc.) still fails the test, just no longer on an unlucky scheduling of the observation
itself.

`wait_until_all_contenders_are_observed_waiting` was not touched, per the ticket's instruction —
it already had the correct polling shape. No product code
(`crates/typdoc-core`, `crates/typdoc` non-test code) was changed.

## Verification
- Isolated test runs: `TMPDIR=/home/thw-home/.cache/typdoc-tmp cargo test -p typdoc --test
  lock_contention` run 5 times in a row (1 initial + 4 more). All 5 passed:
  `test result: ok. 1 passed; 0 failed;` each time (~7.7s each).
- Full `scripts/test.sh` runs, 5 in a row (`TMPDIR=/home/thw-home/.cache/typdoc-tmp
  scripts/test.sh`):
  - Run 1: exit 0 — `1000 test(s) passed across 56 suite(s) on linux (ceiling 6144 MB)`
  - Run 2: exit 0 — `1000 test(s) passed across 56 suite(s) on linux (ceiling 6144 MB)`
  - Run 3: exit 0 — `1000 test(s) passed across 56 suite(s) on linux (ceiling 6144 MB)`
  - Run 4: exit 0 — `1000 test(s) passed across 56 suite(s) on linux (ceiling 6144 MB)`
  - Run 5: exit 0 — `1000 test(s) passed across 56 suite(s) on linux (ceiling 6144 MB)`
  All 5 runs identical: exit 0, 1000 tests passed, 56 suites, no failures.
- `cargo fmt --check`: clean, no output, exit 0.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean, no warnings, exit 0.

## Notes
- The fix is purely additive to the test file's existing vocabulary: `wait_until_lock_names_pid`
  reuses `lock_owner_pid` and the same generous-bounded-poll-with-clear-panic pattern already
  established by `wait_until_all_contenders_are_observed_waiting`, rather than introducing a new
  idiom.
- Reused `LOCK_APPEARS_WITHIN` (20s) as the bound rather than adding a new constant — it was
  already documented as generous headroom for a loaded machine, and a pid write finishing after
  file creation is an even smaller window than the file appearing at all, so the same bound
  comfortably covers it without inventing a redundant constant.
- No flakiness observed in any of the 10 total runs (5 isolated + 5 full-gate) on this machine.
