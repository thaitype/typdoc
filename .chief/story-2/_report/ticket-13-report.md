# Ticket 13: temp files and what a run leaves behind

Resolved. Commit `05c0ea3` (built in an isolated worktree, merged fast-forward with no conflict).
`cargo fmt --check` clean, `cargo clippy --workspace --all-targets -- -D warnings` clean, public-text
check clean, all re-run on the merged tree. Test counts: 828 passed / 0 failed of this ticket's own
and every pre-existing test / 1 ignored, plus 4 failures unrelated to this ticket (below).

## Outcome

done

## What it does

The reserved temp-file shape already lived in `crates/typdoc-core/src/fs.rs` from ticket 1, placed
there so both the writer and a walker could read it from one place; nothing new was added for it.
The per-collection walk now skips that shape before trusting a `match` to have excluded it, and so
do the two other places a file becomes visible: the independent `.md` walk behind `uncollected`, and
the stray-file candidate list behind `filename.pattern`. A skipped leftover is `files.leftover` at
`warn`, fixed rather than merged through `--strict`, the same as `files.unreadable`. `AuditReport`
gained `not_read`, populated under `--audit` from both the unreadable entries and the leftovers, so
the accounting invariant now has one category for everything a `match` or `namespaces` glob met and
did not read.

`find_leftovers` and `remove_leftovers` are new, general-purpose primitives in `typdoc-core`:
discovery over a directory, and removal of a given path list through the write seam, taking
`&NamespaceLock` only as proof of holding one. Which directory a real command hands to
`find_leftovers` is not decided here — no write command exists yet to define a lock's scope on disk.

## Checked by running

- A leftover is skipped under `match: "*"` and under `match: "*.md"`.
- The accounting invariant, extended rather than duplicated, holds on every fixture project
  including a new one that holds a leftover.
- A leftover reported by `validate --audit` was then removed by hand through the seam under a real
  lock acquired on the same directory, and a second `validate --audit` showed it gone — "the next
  command that holds the lock" stood in for, since no write command exists yet to hold one for real.
- A removal staged to fail through the fake file system leaves a `write_atomically` call right after,
  against the same fake, still successful.
- Nine places were shown able to fail, one mutation at a time, each restored: the three exclusion
  points, the finding's fixed level, the audit population, the two recursion/nesting behaviours of
  `find_leftovers`, and the actual removal in `remove_leftovers`. One planted mutation — disabling
  the explicit symlink check — did not turn anything red, because `DirEntry::file_type()` already
  behaves as an `lstat` and neither branch matches a symlink regardless; this is recorded as
  defense-in-depth rather than hidden.
- The existing write ban reaches the new removal path: `std::fs::remove_file` planted in library
  code and in test code was refused each time, and the same call inside `typdoc-fs` was not.

## Decision

- **Issue:** `files.leftover`'s rule id has nowhere to register. Decision 4 left it for decision 9,
  and decision 9 closed with nothing to decide, so the id was never assigned and `docs/design.md`'s
  rules table has no row for it, only prose. `docs/design.md` may not be edited by a ticket.
- **Options considered:** register the id anyway and let the design/code parity test catch the
  mismatch; leave the id an unregistered string literal, used only where the finding is built.
- **Chosen:** the second. Registering it without a matching design row would fail the test that
  checks the two tables against the design exactly, and adding the row is not this ticket's call to
  make. The level is fixed at `warn` for the same reason `files.unreadable` is fixed rather than
  merged through `--strict` — the closer reading of "shares a channel with the entries already
  skipped." Whoever settles the id's registration also settles whether this reading should change.

## Notes

- **A whole, unrelated gate is red on this machine right now, and it is not this ticket's fault.**
  `shell_examples.rs` fails four tests with `Os { code: 122, kind: QuotaExceeded }` copying a
  stand-in binary into a fresh temp directory. Confirmed independently on the unmodified tip this
  ticket branched from — same four failures, same message, before any of this ticket's changes
  existed. `/tmp` is a 9.5G tmpfs at 80% full, holding several gigabytes of scratch from other
  sessions on this shared machine; this is machine disk pressure, not a defect in the write path.
  Every test this ticket added or touched passed; the four failures are the same four regardless of
  which tree is tested. Not silenced, not routed around, reported per rule 2.
- `find_leftovers`/`remove_leftovers` are primitives without a caller yet. The first ticket that
  holds a lock for a real write decides what directory it sweeps with them.
