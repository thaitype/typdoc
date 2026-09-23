# 14: Pin the toolchain and wire up CI

Type: implementation
Status: open
Blocked by: None (can start immediately)

Ticket 4's answer, built: three gates, not four.

## The work

1. `rust-toolchain.toml` at the repo root, pinning `1.96.0` — the version the three gates pass on
   today. Confirm `scripts/test.sh`, `cargo fmt --check`, and `cargo clippy --workspace
   --all-targets -- -D warnings` still pass locally once the pin is in place (a bad pin fails
   loudly here, before CI is even involved).
2. A GitHub Actions workflow running all three gates on every push and pull request into `main`.
   `TMPDIR` is set from the runner's own temp directory in the workflow — never the
   `/home/thw-home/.cache/typdoc-tmp` path story 2 used on this specific machine, and never any
   other machine-specific path.

## Tests (see `testing-decisions.md`, "CI")

On a throwaway branch (not `story-3-catalog-and-release`, not `main`): push one commit that
breaks `scripts/test.sh` (an assertion), confirm CI goes red; revert and push one that breaks
`cargo fmt --check` (an unformatted line), confirm red; revert and push one that breaks `cargo
clippy` (a lint hit with no `#[allow]`), confirm red. Delete the branch once all three are shown
able to fail. Only then does a clean CI run on this ticket's own branch count as evidence the
workflow works.

Whether the runner's own `/tmp` has this machine's disk-pressure problem is not checked ahead of
time — the first real CI run (this ticket's own) answers it; if it does, the fix is the same
shape as story 2's (point `TMPDIR` at a disk-backed path on the runner, not skip the check).

## Done

- `rust-toolchain.toml` exists, pins `1.96.0`, and all three gates pass under it.
- CI runs all three gates on push and PR into `main`, with no machine-specific path.
- The red-before-green demonstration is done and recorded in this ticket's report (commit SHAs
  or run links for each of the three failures).
