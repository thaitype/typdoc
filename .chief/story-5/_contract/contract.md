# Contract

## Targets and artifact naming

Five prebuilt binaries, one per target triple, attached to the GitHub release for 0.3.1. Linux
targets are **musl**, not gnu — see "Linux minimum-supported distro" below for why.

| Target                        | Asset                                        |
|--------------------------------|-----------------------------------------------|
| `x86_64-apple-darwin`          | `typdoc-x86_64-apple-darwin.tar.gz`           |
| `aarch64-apple-darwin`         | `typdoc-aarch64-apple-darwin.tar.gz`          |
| `x86_64-unknown-linux-musl`    | `typdoc-x86_64-unknown-linux-musl.tar.gz`     |
| `aarch64-unknown-linux-musl`   | `typdoc-aarch64-unknown-linux-musl.tar.gz`    |
| `x86_64-pc-windows-msvc`       | `typdoc-x86_64-pc-windows-msvc.zip` (contains `typdoc.exe`) |

Each archive ships next to a `.sha256` checksum file and carries a GitHub artifact attestation
(`actions/attest-build-provenance`), generated in `publish.yml` after `cargo dist build`, not by
dist's own CI.

## Build matrix (which runner builds/runs which target)

| Target                       | Builds on              | `typdoc --version` run on |
|-------------------------------|-------------------------|------------------------------|
| `x86_64-apple-darwin`         | `macos-latest` (cross)  | not run in CI — named built-but-not-run in the release notes |
| `aarch64-apple-darwin`        | `macos-latest`          | `macos-latest`                |
| `x86_64-unknown-linux-musl`   | `ubuntu-latest`         | `ubuntu-latest`, then re-verified inside an old-distro container (below) |
| `aarch64-unknown-linux-musl`  | `ubuntu-24.04-arm`      | `ubuntu-24.04-arm`, then re-verified inside an old-distro container (below) |
| `x86_64-pc-windows-msvc`      | `windows-latest`        | `windows-latest`              |

Cross-compilation mechanics for the two musl targets follow cargo-dist's own standard approach
for these targets rather than a hand-rolled toolchain setup — a build-time detail, not part of
this contract.

Only `x86_64-apple-darwin` has no matching-arch GitHub-hosted runner available to this repo, so
it is the one target built-but-not-run — named explicitly in the release notes; a successful
cross-compile is never presented as "verified working."

## Linux minimum-supported distro

A gnu-linked binary built on a recent runner refuses to start on an older distro whose glibc is
older than the one it linked against. Chosen fix: **musl targets**, statically linked, carrying
no glibc dependency at all — this removes the glibc-version coupling entirely rather than
managing it by pinning an old build image.

- Docs state the minimum supported Linux as: any x86_64/aarch64 distro with a kernel new enough
  for Rust's own baseline for these targets (documented alongside the install instructions), with
  no glibc version requirement.
- Proven in CI, not just asserted: after the native build, the resulting binary is copied into an
  intentionally old, unrelated container image on each architecture (e.g. a long-EOL Debian
  release, picked specifically for being older than anything reasonably still in use) and run
  there (`typdoc --version`) to confirm it starts with no dynamic dependency on that system's
  libc. Both arches get this proof, not just one.

## `dist-workspace.toml`

- `cargo-dist` pinned to an exact version (not a range), so upgrading it later is a deliberate,
  visible change rather than something that can silently alter the build.
- `installer = ["shell", "powershell"]` generates the two base scripts, per-script: for either
  script, if its generated behavior cannot be made to satisfy the installer contract below
  through config alone — most notably the single fixed Windows install path — that one script is
  hand-written instead, with dist staying in charge of build + checksums either way. The two
  scripts are judged and decided independently; they do not have to land the same way. **Which
  way each one went is stated plainly in the final report — not left for the diff to reveal.**
- Path modification disabled in dist's own installer config — its shell/powershell installers by
  default append to `PATH` in the user's shell profile; this is turned off (see "no PATH edits"
  below).

## Installer script contract (`install`, `install.ps1`)

Behavior every implementation (dist-generated or hand-written) must satisfy — this is what's
tested, not which tool produced the script:

- Detect OS + architecture; map to one of the five targets above; on anything else, print an
  error naming the unsupported platform and exit non-zero. Never fall back to a "closest" target.
- Resolve the version to install: `TYPDOC_VERSION` if set, else the latest release — resolved via
  the `releases/latest` redirect (`https://github.com/thaitype/typdoc/releases/latest/download/...`
  style URL), never the unauthenticated REST API (rate-limited per-IP, shared CI runners collide).
- Download base: normally GitHub Releases for this repo. A second override,
  `TYPDOC_INSTALL_BASE_URL`, points the download at an arbitrary base instead — **test-only**,
  used to run the installer against a local HTTP server in CI before any release exists (see
  Testing Decisions). Not documented in README/getting-started as a supported user-facing
  variable; a comment in the script itself and this contract are its only documentation, so nobody
  mistakes it for a stable interface.
- Download the matching archive and its `.sha256` file; recompute the checksum locally and
  compare; on mismatch, print an error and exit non-zero **before** any file is placed on disk
  outside a temp directory. Never install an unverified file.
- Install location: `${INSTALL_DIR:-$HOME/.local/bin}` on macOS/Linux; on Windows, always
  `%LOCALAPPDATA%\typdoc\bin` — one fixed path regardless of which generator produced the script,
  so the docs only ever need to print one Windows path. Both overridable via `INSTALL_DIR`.
- **No `PATH` or shell-rc edits, on any OS.** If the install directory isn't already on `PATH`,
  print the one line the user needs to run (`export PATH=...` / a profile line for Windows) and
  exit 0 — installation succeeded either way.
- Exit codes: `0` success, non-zero on any failure (unsupported platform, download failure,
  checksum mismatch). No partial-success exit code.

## Pages site (`typdoc.thaitype.dev`)

- A dedicated directory (e.g. `pages/`) holding exactly three files: `install`, `install.ps1`,
  and a root `index.html` that redirects to `https://github.com/thaitype/typdoc` (meta-refresh or
  a small JS redirect — GitHub Pages serves static files only, no server-side redirect).
- Deploy workflow (`pages.yml`) using `actions/deploy-pages`, triggered on push to `main` only,
  path-filtered to that directory (mirrors `publish-check.yml`'s existing path-filter pattern) —
  a merge here is a production deploy per the brief, so it goes through a normal PR with CI, same
  as everything else.
- The Pages source setting and the custom domain (`typdoc.thaitype.dev`) are repo settings, not
  files — requested with the exact setting to flip when the workflow is ready, not changed
  directly by this work.

## CI: live-URL install check

This checks the **already-deployed** thing, not a PR's code — a PR gate here would fail on
unrelated PRs whenever GitHub Pages/Releases hiccups, and would pass or fail based on state the
PR didn't touch. It is not a PR check anywhere. Instead:

- A step **after deploy** in `pages.yml`: fetch `https://typdoc.thaitype.dev/install` and
  `/install.ps1` and confirm they serve, blocking that `pages.yml` run if they don't.
- A step **after the release** in `publish.yml`: run the live install one-liner and `.ps1`
  equivalent on ubuntu, macOS and Windows runners, then `typdoc --version` — blocking that
  `publish.yml` run, not any PR.
- A scheduled run (e.g. weekly) repeating the same live check independently of any deploy or
  release, to catch drift (cert expiry, DNS, Pages outage) between releases.

## CI: Windows test-suite pass-rate job

- New job, Windows-only, **non-blocking** (must never turn a PR red) — runs the full test suite
  without `scripts/test.sh`'s memory cap (its `systemd-run`/`vm_stat` mechanism doesn't exist on
  Windows; this job needs no memory cap of its own, just running the suite and counting results).
- Reports passed/total and a percentage in the job's `$GITHUB_STEP_SUMMARY`. This story does not
  fix any Windows failure it finds — the number is the deliverable, not a green run.

## Testing Decisions

- **Installer scripts, pre-merge:** no release carries these assets until 0.3.1 is published, so
  the PR-time test can't hit real GitHub Releases. Instead, the CI-built candidate archives and
  their `.sha256` files are served from a local HTTP server started in the job, and the installer
  is pointed at it via `TYPDOC_INSTALL_BASE_URL`. Against that local server: default install,
  `INSTALL_DIR` override, `TYPDOC_VERSION` pin (to a second fixture version served locally), and
  an intentionally-corrupted checksum (must fail loudly, non-zero exit, nothing installed) all
  run on all three OS families. This is the same shape as `check-in-ci.md`'s existing pattern of
  testing a documented workflow by actually running it, not by asserting on doc text.
- **Installer scripts, post-deploy/post-release:** covered by "CI: live-URL install check" above
  — a separate, later check against the real live URLs and the real release, not a substitute for
  the pre-merge local-server test.
- **The five binaries** are tested by execution (`typdoc --version`) on a same-arch runner per
  the build matrix above, with the two musl binaries additionally proven inside an old-distro
  container per "Linux minimum-supported distro." The one target with no same-arch runner is
  accepted as built-but-not-run and named as such, not silently passed over.
- **The existing gate** (`scripts/test.sh` + fmt + clippy on ubuntu/macOS) is unchanged and
  continues to run before any dist build step in `publish.yml` — this story adds steps after the
  existing `gate` job, it does not touch it.
- **The Windows pass-rate job** is not a correctness test of typdoc; it is a measurement. Its own
  step (computing and printing the rate) is the thing under test — it must never itself fail
  silently (a step that finds zero tests should error loudly, mirroring `scripts/test.sh`'s own
  existing "refuses to report success on a zero count" behavior).
