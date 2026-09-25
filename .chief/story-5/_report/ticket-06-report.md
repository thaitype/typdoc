# Ticket 06 Report

## Ticket
Non-blocking Windows test-suite pass-rate job in `ci.yml`.

## Outcome
done

## Decision
- **Issue:** the contract only said "the job's own counting step must error loudly on zero
  tests" — it didn't specify inline PowerShell vs. a standalone script, and ticket 03 (installer
  docs section) hasn't landed yet, so there was no natural place for a Windows-experimental docs
  line.
- **Options considered:** inline `pwsh` counting logic vs. a standalone, unit-testable script;
  for docs, wait for ticket 03 vs. update the existing README "Windows isn't supported" line now.
- **Chosen:** a standalone Python script (`scripts/windows_test_report.py` +
  `scripts/test_windows_test_report.py`, 11 unit tests) — lets the counting/reporting logic (the
  contract's "thing under test") go through a real TDD loop, and `windows-latest` ships Python.
  Docs: updated README's existing Windows line now rather than waiting on ticket 03.

## Notes
- Non-blocking via job-level `continue-on-error: true` (not step-level).
- `$LASTEXITCODE` propagation needed explicit handling after every native `pwsh` invocation —
  worth remembering for ticket 05's Windows live-check step, which will hit the same thing.
- Code review (strict mode) found and fixed two real issues: duplicated zero-check logic, and an
  uncaught crash if the cargo-output file is missing (now folds into the same loud zero-tests
  path).
- Commit `8d9ab23` on `story-5-prebuilt-installer` (fast-forwarded from
  `story-5-prebuilt-installer-ticket-06`'s single commit `65ee2a2`, rebased onto the branch tip
  first). Public-text grep clean.
- Caveat carried forward: the actual Windows-runner execution of these `pwsh` steps is unverified
  beyond manual review + the Python unit tests — first real confirmation comes from the PR's own
  CI run.
