# Ticket 01 Report

## Ticket
Reusable `workflow_call` build workflow for the five release targets, a PR-triggered caller,
wiring into `publish.yml` with attestation, version bump to 0.3.1, and CHANGELOG text.

## Outcome
done

## Decision
- **Issue:** cargo-dist's own dynamic CI matrix (via `dist plan`/`dist generate`) picks different
  runners than the contract's explicit table — e.g. a real `macos-15-intel` runner for
  `x86_64-apple-darwin` (which the contract deliberately treats as build-but-not-run on
  `macos-latest` cross-compile) and `ubuntu-22.04-arm` instead of `ubuntu-24.04-arm`.
- **Options considered:** adopt `dist generate`'s own workflow output as-is vs. hand-write the
  matrix to match the contract literally, using `dist build` only as the per-target build tool.
- **Chosen:** hand-written matrix matching the contract's table exactly — the contract is
  pre-approved and authoritative; `dist`'s own opinions about runners don't override it.
- **Issue:** two `dist-workspace.toml` settings were required but not mentioned in the ticket —
  discovered only by actually running `dist` against this repo: `allow-dirty = ["ci"]` (dist
  otherwise refuses to run without its own auto-generated release workflow present) and
  `[dist.binaries] "*" = ["typdoc"]` (without it, `dist build` tries and fails to package the
  test-only `typdoc_stand_in` binary into every archive). Also `unix-archive = ".tar.gz"` — dist
  defaults to `.tar.xz`, contradicting the contract's asset-naming table.
- **Chosen:** added all three, verified by actually running `dist build --artifacts=local` (with
  a temporary substitute target, since the sandbox can't build all 5) end-to-end: correct archive
  name, correct checksum, correct binary contents, `typdoc --version` working.
- **Issue:** the contract asks for "path modification disabled in dist's own installer config,"
  but `dist-workspace.toml` (checked against cargo-dist 0.32.0's own source) has no such
  build-time key — `NO_MODIFY_PATH` is a runtime env var the generated script itself reads.
- **Chosen:** left as ticket 03's problem, since it already owns "turning off dist's own path
  modification if its generated installer is used" — flagging so ticket 03 knows it needs an
  env-var/wrapper approach, not a `dist-workspace.toml` setting.

## Notes
- **Real gap found, not fixed here (correctly, per this ticket's scope):** no ticket in the
  original frontier actually creates a GitHub Release or uploads the built assets to it —
  confirmed live: `v0.3.0`/`v0.2.0`/`v0.1.0` on this repo all have `assets: []`, so release
  creation has been a manual, out-of-band step until now. This story's goal explicitly
  requires "release 0.3.1 ... carrying all five binaries" via `publish.yml`, so this is real
  missing scope, not something to leave dropped. **Filed as new ticket 07
  (`07-release-asset-upload.md`), blocked by this ticket, added to the frontier.**
- Action versions pinned to current majors (`checkout@v7`, `upload-artifact@v7`,
  `download-artifact@v8`, `attest-build-provenance@v4`) rather than matching the existing gate
  job's older pin — a deliberate judgement call, commented in the workflow files themselves.
- `dist-build`/`attest` jobs in `publish.yml` are not gated on `inputs.dry_run` — harmless on a
  dry run since no release exists to attach anything to yet (ticket 07 will need to think about
  this interaction).
- Local verification: `cargo fmt --check`, `cargo clippy -D warnings`, and the full
  `scripts/test.sh` gate (1057 tests) all passed on the final commit.
- Commit `f2607bb` on `story-5-prebuilt-installer` (fast-forwarded from
  `story-5-prebuilt-installer-ticket-01`'s single commit, rebased onto the branch tip first).
  Public-text grep clean (checked twice).
