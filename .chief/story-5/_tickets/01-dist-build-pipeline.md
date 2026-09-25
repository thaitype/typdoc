Type: implementation
Status: claimed
Blocked by: None

# Ticket 01 — reusable build workflow + release plumbing

`workflow_dispatch` cannot run a workflow that isn't on `main` yet, so wiring the 5-target build
only into `publish.yml` would leave it unexercised until the real release dispatch — the same
trap the last story hit with its publish gate. Fix: put the build in a **reusable workflow**
(`on: workflow_call`) that two callers use:
- a PR-triggered workflow (`pull_request`, path-filtered) that calls it on every relevant PR, so
  the PR proves the exact build the release will later run, and produces the archives as workflow
  artifacts other tickets (03) can consume;
- `publish.yml`, which calls the same reusable workflow, then adds an `actions/attest-build-
  provenance` step over its output (attestation lives in `publish.yml` only — not generated on
  every PR build).

`dist-workspace.toml`: five targets (`x86_64-apple-darwin`, `aarch64-apple-darwin`,
`x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl`, `x86_64-pc-windows-msvc`), `cargo-dist`
pinned to an exact version. The reusable workflow runs after the existing `gate` job when called
from `publish.yml` (gate stays untouched); the PR-triggered caller runs independently on PRs.

Produces, per target: the archive (`.tar.gz` on Unix, `.zip` containing `typdoc.exe` on Windows)
and a `.sha256` checksum file.

Also in this ticket: bump every published crate's version to `0.3.1` (no behaviour change in
typdoc itself), and write the CHANGELOG/release-notes text for that release — including the line
naming `x86_64-apple-darwin` as built-but-not-run (see ticket 02/build matrix). This text is
prepared now; the actual release dispatch happens after merge, outside this loop.

Demoable on its own: a PR against this branch runs the reusable workflow via its PR-triggered
caller and produces all five archives + checksums as workflow artifacts.
