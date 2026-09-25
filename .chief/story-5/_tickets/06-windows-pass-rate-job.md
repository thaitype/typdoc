Type: implementation
Status: resolved
Blocked by: None

# Ticket 06 — Windows test-suite pass-rate job

Add a Windows-only job to `ci.yml`, non-blocking (must never turn a PR red), running the full
test suite without `scripts/test.sh`'s memory cap (its `systemd-run`/`vm_stat` mechanism doesn't
exist on Windows — this job needs no memory cap of its own). Reports passed/total and a
percentage to `$GITHUB_STEP_SUMMARY`.

The job's own counting step must error loudly if it finds zero tests, mirroring
`scripts/test.sh`'s existing "refuses to report success on a zero count" behavior — a step that
silently ran nothing must not read as a valid 0% or 100%.

This ticket does not fix any Windows test failure it finds. Docs: mark Windows support as
experimental next to the install instructions, and mention the pass-rate job as the baseline
future stories improve against.

Demoable on its own, independent of every other ticket in this story: a PR shows the job posting
a pass-rate summary without blocking merge, even with some Windows tests failing.
