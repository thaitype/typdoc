# Goal

Install typdoc without a Rust toolchain: prebuilt binaries attached to every GitHub release, and
a one-line installer served from this repo's own domain — alongside the existing
`cargo install typdoc` path, which stays supported.

The story ends with **release 0.3.1** published through `publish.yml` carrying all five binaries,
their checksums, and their attestations — the first release to do so. Cutting that release is an
owner action after merge, not loop work — see "After Merge" below for the exact order.

From a user's perspective:

- `curl -fsSL https://typdoc.thaitype.dev/install | sh` installs typdoc on macOS or Linux
  (x86_64 or aarch64) with no Rust toolchain required.
- `irm https://typdoc.thaitype.dev/install.ps1 | iex` installs typdoc on Windows (x86_64),
  documented as experimental.
- Both scripts detect OS/architecture, download the matching release asset, verify its SHA-256
  before installing, and fail loudly (never install an unverified file) on a checksum mismatch
  or an unsupported platform. Default install location is `~/.local/bin` (Windows: the per-user
  directory); `INSTALL_DIR` overrides it.
- Every released binary carries a SHA-256 checksum and a GitHub artifact attestation
  (`actions/attest-build-provenance`), so provenance can be verified independently of the
  installer (`gh attestation verify`).
- `TYPDOC_VERSION` lets the installer pin a specific release instead of installing latest.
- Each binary is executed (`typdoc --version`) in CI on a runner of its own architecture; any
  target that cannot be run in CI (e.g. no Windows-arm runner needed here, but the case can recur)
  is named explicitly as built-but-not-run in the release notes — a passing cross-compile is never
  presented as a binary that runs.
- CI installs from the **live URLs** (`https://typdoc.thaitype.dev/install`, `/install.ps1`), not
  repo files, on ubuntu, macOS and Windows runners, and runs `typdoc --version` on each.
- `https://typdoc.thaitype.dev/` itself (no path) redirects to this GitHub repo, via a root
  `index.html` — the domain is never a bare 404.
- The Windows test suite runs in CI on every relevant change and reports a pass-rate baseline
  (passed / total, a percentage) as a non-blocking job — a number to improve against in later
  stories, not a gate.
- README, `docs/getting-started.md`, the relevant how-to/reference pages, and `skills/typdoc/`
  are updated in the same PR as the behaviour they describe.

## Out of Scope

- Paid code signing or notarization. Documented workarounds only: `xattr -d
  com.apple.quarantine` for a macOS binary downloaded through a browser; the Windows SmartScreen
  warning on a browser-downloaded zip is documented, not suppressed.
- Fixing Windows test failures — this story measures and reports the pass rate, nothing more.
- Comment-review — deferred to a later story.
- Editing `PATH` or any shell rc file, on any OS, from either installer or from cargo-dist's own
  generated installer (its path-modification behaviour is turned off). Both installers print the
  one line to run if the install directory isn't already on `PATH`.
- Separate upgrade/uninstall tooling: upgrade is re-running the installer; uninstall is a
  documented manual removal of the installed binary.

## Needs from outside the repo

Required for the story to be truly done, not optional — just not performable by a commit in this
repo:

- **Pages source / custom-domain setting**, set in repo settings on this public repo: requested
  with the exact setting to flip, not changed silently by this work. Outstanding.
- **Cloudflare DNS record** for the install domain (`CNAME` to `thaitype.github.io`, DNS-only):
  outside this repo's automation. Already live, confirmed externally resolving to
  `thaitype.github.io` — no longer outstanding.

## After Merge (owner actions, not loop work)

The loop's tickets end at "PR open, CI green." Everything below happens afterward, in this exact
order, outside the loop:

1. **Merge** the PR to `main`.
2. **Request the Pages source / custom-domain setting** be flipped (the "Needs from outside the
   repo" item above) — reported with the exact setting once the merged `pages.yml` is ready to
   receive it.
3. **Pages deploys** on that push to `main`; the domain starts resolving over HTTPS.
4. **Live check**: confirm both installer URLs serve (the post-deploy step from ticket 05 covers
   this once it runs for real).
5. **Owner dispatches `publish.yml`** for 0.3.1 — dry run first, then the real run, same pattern
   as the last story's release.
6. **Post-release live check** runs as part of that same dispatch (ticket 05's post-release step)
   — confirms the live install one-liners work against the actual 0.3.1 release.
