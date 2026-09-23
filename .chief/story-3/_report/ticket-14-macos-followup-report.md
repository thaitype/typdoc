# Ticket 14 (macOS follow-up) Report

## Ticket

Extend CI to run on both `ubuntu-latest` and `macos-latest`, and fix `scripts/test.sh`'s
Linux-only memory cap so it works (or fails loudly and correctly) on macOS too.

## Outcome

done locally; the one thing that actually proves it (a real macOS CI run) is next, not yet done

## Decision

- **Issue:** `scripts/test.sh`'s `require_scope` used `systemd-run --user --scope`, Linux-only,
  and exits 2 immediately on macOS by its own explicit design ("fails loudly rather than running
  uncapped"). No workflow-file addition alone would have made the mac job run at all.
- **Chosen:** `ulimit -v` (RLIMIT_AS) for macOS — the only per-process memory lever a plain bash
  script has there, alongside the existing `systemd-run` path for Linux, dispatched on `uname -s`.
  Documented directly in the script, not just in this report, that this is a real, unresolved
  tension: `ulimit -v` caps virtual size, which sits well above resident size on Darwin (dyld
  shared cache, reserved-but-untouched allocator arenas), arguing for a generous number — but
  GitHub's actual `macos-latest` (Apple Silicon, 3 cores / 7 GB RAM, confirmed by checking
  GitHub's own current runner spec rather than assuming the older Intel numbers) only has 7 GB
  total, arguing for a tight one. Picked `5120` MB, favoring not exceeding the box's real RAM over
  comfortably clearing the VSZ baseline. **This number is a reasoned estimate, not a
  measurement — a real run is what actually tells us if it's right**, in either direction (too
  tight: an ordinary build can't even start; too loose: doesn't catch a real runaway).
- Added a `PROBE-OK` check to `--self-test` before trusting the cap to stop a runaway: proves an
  ordinary command still runs under the same cap first, since a cap too tight to run anything
  looks identical, from the runaway-stopped check alone, to a cap correctly killing growth.
- Added a real "did the suite actually run" guard, OS-agnostic by construction: `scripts/test.sh`
  now sums cargo's own "N passed" counts across every suite, prints the total, and exits 3 if a
  run reports exit 0 with zero passed tests — this is the "mac job green because a step was
  skipped or found nothing looks identical to one that works" concern, made into an actual check
  rather than left to eyeballing a log.

## Notes

Reviewed the diff directly (not just the build's own report) before merging — the reasoning in
the script's comments is honest about what's estimated vs. measured, which matters here since the
number that resolves it can only come from a real run. GNU-only shell assumptions checked (none
pre-existing; two introduced by this change were fixed: `mktemp` needs an explicit template on
BSD/macOS, and the new `awk` summation uses only POSIX field-splitting).

All three gates re-verified green on this Linux machine after merging (1010 tests, 56 suites,
`--self-test` still reports "ok"). CI workflow now matrices all three jobs across both OSes,
`fail-fast: false`.

**What only a real run can answer, listed plainly rather than assumed:** whether `ulimit -v`
enforces anything on Darwin at all; whether 5120 MB is workable on the actual runner; any
bash-version quirks; whether the test count matches between the two OS jobs (no known
platform-specific `#[cfg]` gate beyond `#[cfg(unix)]`, true on both, so a match is expected but
unconfirmed).

**Next step, separately authorized (M-12):** push a throwaway branch to prove each CI gate red,
now covering both OSes, then delete it — this is what actually resolves the open questions above,
not further local reasoning about them.
