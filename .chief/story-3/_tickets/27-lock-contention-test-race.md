# 27: `lock_contention.rs`'s own test races the holder's pid-write, not a product bug

Type: implementation
Status: resolved
Blocked by: None (can start immediately)

**Found 2026-09-24, Aria — verified by reading the real CI failure from the story's own
red-before-green proof run (`35954561280`, the clippy-break run), not assumed from the
"known flake" label this session had given it.** The failure
(`lock_contention.rs:104`, `n_processes_racing_one_lock_issue_n_distinct_keys_with_no_document_overwritten`,
`"the lock file the holder just created must name its own pid" — left: None`) had been treated
earlier in this story as an environmental flake sensitive to machine load. Aria's read: it is not
that, and not a duplicate-key bug either — it's a race in the **test itself**, inside the
design's own documented window (lock file created, pid not yet written into it):

```rust
assert!(
    wait_for_file(&lock, LOCK_APPEARS_WITHIN),
    "the holder's own lock file never appeared"
);
let holder_pid = holder.pid();
assert_eq!(
    lock_owner_pid(&lock),
    Some(holder_pid),
    "the lock file the holder just created must name its own pid"
);
```

`wait_for_file` only waits for the lock file's *existence* — it says nothing about whether the
holder process has finished writing its pid into that file yet. The very next line reads the pid
back out immediately, so on an unlucky scheduling the file exists (empty, or partially written)
before the write completes, and `lock_owner_pid` returns `None`. This is exactly the same
distinction the test's own `wait_until_all_contenders_are_observed_waiting` helper (lines
54-75, same file) already gets right a few lines later: it polls until the lock *names a pid*,
not until the file merely exists.

## The work

Fix the test, not the product: replace the `wait_for_file` + immediate `lock_owner_pid` check
(lines 99-108) with a bounded poll that waits until `lock_owner_pid(&lock)` returns `Some(_)` (or
specifically `Some(holder_pid)`, matching what's actually being asserted), the same pattern
`wait_until_all_contenders_are_observed_waiting` already uses — reuse that same polling shape (a
small `Duration::from_millis` sleep in a loop, bounded by a generous timeout, panicking with a
clear message on timeout) rather than inventing a second one. A small, clearly-named constant for
this bound is fine (e.g. reuse `LOCK_APPEARS_WITHIN` if its bound is already generous enough for
this too, or add a new one if not — check `common::LOCK_APPEARS_WITHIN`'s value and doc comment
before deciding).

## Tests

- The fixed test passes normally (single run).
- Run the full gate (`TMPDIR=/home/thw-home/.cache/typdoc-tmp scripts/test.sh`) a handful of
  times in a row (at least 5) to build confidence the race is actually closed, not just less
  likely — record each run's result in the report. This is Aria's own instruction: "run the full
  gate a handful of times to show it holds."
- No other test in the file changes behavior; `wait_until_all_contenders_are_observed_waiting`
  itself is untouched (it already has the right shape).

## Done

- The pid-read race in `n_processes_racing_one_lock_issue_n_distinct_keys_with_no_document_overwritten`
  is closed: the test waits until the lock names a pid before asserting which one, not just until
  the file exists.
- At least 5 consecutive full `scripts/test.sh` runs pass with no `lock_contention` failure.
- All three gates green.
