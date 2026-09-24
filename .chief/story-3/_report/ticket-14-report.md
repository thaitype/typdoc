# Ticket 14 Report

## Ticket

Pin the toolchain to `1.96.0` and wire up CI on the three real gates (`scripts/test.sh`,
`cargo fmt --check`, `cargo clippy`).

## Outcome

done, with one piece of acceptance evidence still outstanding (see Notes)

## Notes

`rust-toolchain.toml` pins `1.96.0` with `rustfmt`/`clippy` components. `.github/workflows/ci.yml`
runs the three gates as separate jobs on every push and on PRs into `main`; `TMPDIR:
${{ runner.temp }}` avoids hard-coding this machine's `/home/thw-home/.cache/typdoc-tmp`.

All three gates verified locally under the pin. Each gate's ability to fail was demonstrated
locally (a flipped assertion, an unformatted line, an unallowed clippy hit — each reverted after,
`git diff` confirmed clean) rather than only asserted.

**Outstanding, not done:** the ticket's own acceptance criterion is proving red-then-green on a
real GitHub Actions run (a throwaway branch, pushed, watched go red on each gate, then deleted).
That needs a push to the actual `thaitype/typdoc` remote, which this story is holding off on
("don't push until Mild says so") — so it is deliberately not done yet, not forgotten. Whoever
does the eventual push/PR step for this story should run that demonstration before treating
ticket 14 as fully proven, not just merged.

**A real risk flagged for that run, not fixed blind:** `scripts/test.sh` calls `systemd-run
--user --scope` to cap memory (`require_scope`, `scripts/test.sh:39-55`) and exits with a clear
error if no user systemd session is available. Whether GitHub's `ubuntu-latest` runner has one
for the actions-runner user is unverified — this is the first thing to check if the `test` job
fails on the real run in a way that doesn't look like an actual test failure.

Merged into `story-3-catalog-and-release` (merge commit, `.github/workflows/ci.yml` and
`rust-toolchain.toml` added, no conflicts with ticket 9's changes).
