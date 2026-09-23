# 14: Pin the toolchain and wire up CI

Type: implementation
Status: resolved (macOS follow-up built; live CI proof still pending, see report)
Blocked by: None (can start immediately)

Ticket 4's answer, built: three gates, not four.

**Reopened 2026-09-23 (Aria, relaying Mild):** *"ให้ทำ github actions ที่ ubuntu กับ mac นะครับ"* —
CI runs every gate on **both** `ubuntu-latest` and `macos-latest`. Windows not asked; don't add
it. Reopening this ticket rather than filing a new one — same deliverable, `.github/workflows/ci.yml`,
already built for ubuntu; this is that file's own follow-up, not new scope. The red-before-green
proof (this ticket's own Tests section, still outstanding as of the ubuntu-only build — see the
merged report) should be redone once macOS is added, so the proof covers both runners, not just
the one already shown red. Pushing that proof is now approved (M-12): a throwaway branch, deleted
after, as `mildronize` via the one-shot credential helper — not the story branch, no PR, nothing
to `main`.

**Extra care for the macOS leg, per Aria:**
- **The mac job must actually run the suite**, not just report green because a step silently did
  nothing or skipped. Compare the test count `scripts/test.sh` reports between the ubuntu and
  macOS jobs' logs — they should match (same suite, same tests) unless there's a real, named
  platform difference.
- **macOS ships bash 3.2 and BSD `sed`/`mktemp`/`grep`, not GNU.** Read `scripts/test.sh` for
  anything GNU-only before assuming it just works on macOS: `sed -i` with no suffix argument (BSD
  requires one, even if empty), `grep -P`, `readlink -f`, `mapfile`, and similar. CI will surface
  a real incompatibility, but check first rather than finding out only from a red run.
- `TMPDIR` from the runner's own temp directory on both OSes — the existing `${{ runner.temp }}`
  approach already used for ubuntu should work for macOS runners too (it's a GitHub Actions
  built-in, not an ubuntu-specific variable), but confirm rather than assume.
- **Confirmed, not just suspected (checked `scripts/test.sh` directly):** `require_scope()`
  (lines 39-48) exits 2 immediately — before running a single test — if `systemd-run` isn't on
  the machine: *"refusing to run the tests uncapped."* `systemd` is Linux-only; it deterministically
  will not exist on `macos-latest`. This isn't a maybe the CI run might surface — `scripts/test.sh`
  as it stands today cannot run on macOS at all, by its own explicit design ("fails loudly rather
  than running uncapped: a safety net that disappears quietly is worse than none"). This ticket's
  macOS work therefore isn't just a workflow-file addition — it needs a real change to
  `scripts/test.sh` itself: a macOS-appropriate memory cap (e.g. `ulimit -v`, or a `launchctl`
  equivalent) alongside the existing `systemd-run` path, keeping the same "fail loudly, never run
  uncapped" principle rather than quietly dropping the cap on macOS. Decide and build that
  change as part of this ticket — it's a real gap the CI addition surfaces, not a design question
  to route elsewhere.

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
