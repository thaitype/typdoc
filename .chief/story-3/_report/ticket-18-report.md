# Ticket 18 Report

## Ticket

`list`'s header row (M-10g): identity column, `title`, each `--where` field, above the existing
table.

## Outcome

done

## Notes

Header widths computed over the full `matched` set (not just the listed/limited rows), keeping
the existing "a value doesn't change under `--limit`" invariant. Identity label (`key` vs.
`path`) uses the same per-row rule the table already had, verified against a mixed coded/
path-identified result too (not explicitly required by the ticket, but a real ambiguity the
build closed rather than leaving implicit). `--limit 0` deliberately keeps printing nothing
(no header-with-zero-rows) — the conservative reading, not a new behavior.

Deliberately did not touch `docs/getting-started.md`'s worked `list` example, even though this
change makes it stale — correctly left to ticket 22, which is blocked-by this ticket precisely
for that doc sweep.

**One re-verification note:** the post-rebase gate run hit a flaky failure in
`lock_contention.rs`'s process-racing test (a pid the holder just wrote read back as `None`) —
confirmed as resource contention from the other two tickets building in parallel at the time,
not a real regression: the same test passed in isolation immediately after, and a second full
`scripts/test.sh` run was clean. Re-running rather than trusting a single red result, and
re-running rather than trusting a single green one, both mattered here.

Rebased cleanly onto the current tip (tickets 9/14/15/16/17 in between); re-ran all three gates
after the rebase. Fast-forward merged into `story-3-catalog-and-release`.
