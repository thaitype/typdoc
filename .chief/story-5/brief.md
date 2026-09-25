# Story 5 — brief

Story 5: `/chief-plan` with the normal grill — no wayfinder — then plan and build.

Branch from `origin/main` (`ad53680`, v0.3.0 plus the post-publish docs). The local checkout may
still be on `docs-post-0.3.0-publish`, which is already merged — the name is old, not this story.

## Goal in one line

Install typdoc without a Rust toolchain: prebuilt binaries on every GitHub release, and a
one-line installer served from `https://typdoc.thaitype.dev/install`.

## Decided

1. **Prebuilt binaries for five targets**, attached to the GitHub release, each with a SHA-256
   checksum:
   macOS x86_64 and aarch64 · Linux x86_64 and aarch64 · Windows x86_64 (zip containing `typdoc.exe`).
2. **Installer scripts served from GitHub Pages of this repo, custom domain `typdoc.thaitype.dev`:**
   - `https://typdoc.thaitype.dev/install` — POSIX `sh`, macOS and Linux:
     `curl -fsSL https://typdoc.thaitype.dev/install | sh`
   - `https://typdoc.thaitype.dev/install.ps1` — PowerShell, Windows:
     `irm https://typdoc.thaitype.dev/install.ps1 | iex`
   - Behaviour: detect OS and architecture, download the matching asset from GitHub Releases,
     verify its SHA-256, install to `~/.local/bin` (Windows: a per-user directory, then say how to
     put it on `PATH`). `INSTALL_DIR` overrides the location. Fail loudly on an unsupported
     platform or a checksum mismatch — never install an unverified file.
3. **No paid code signing or notarization in this story.**
   - macOS: `curl` does not set the quarantine attribute, so Gatekeeper does not block a binary
     installed by the script; the Rust toolchain's linker ad-hoc signs aarch64 binaries, which is
     what Apple Silicon requires to run them. A binary downloaded through a browser *is*
     quarantined — document the `xattr -d com.apple.quarantine` step for that path.
   - Windows: a zip downloaded through a browser shows a SmartScreen warning. Accepted;
     document it.
4. **Provenance: GitHub artifact attestations** (`actions/attest-build-provenance`) on every
   released binary. A checksum published next to the file only catches a corrupted download; an
   attestation shows the binary was built by this repository's workflow. Docs show
   `gh attestation verify`.
5. **Windows is experimental in this story.**
   - Verified in CI only (there is no Windows machine to test on by hand): build, install through
     `install.ps1`, run `typdoc --version`.
   - **The full test suite also runs on Windows in CI as a non-blocking job** (it must never turn a
     PR red) that reports its **pass rate** — passed / total and a percentage — in the job
     summary. This is a baseline to measure progress against in later stories.
   - Fixing Windows test failures is **out of scope** for this story.
   - Docs say plainly that Windows support is experimental.
6. `cargo install typdoc` stays a supported install path.

## Must not be misread

- **Merging a change to the install scripts is a production deploy.** GitHub Pages publishes on
  merge, and what it publishes is what users pipe into their shell. Every change goes through a
  PR with CI, and after a deploy the check is made against the live URL
  (`curl -fsSL https://typdoc.thaitype.dev/install`), not against the file in the repo.
- **DNS is outside the repo.** `thaitype.dev` is on Cloudflare and has no record for `typdoc` yet.
  A `CNAME typdoc → thaitype.github.io` record gets added (DNS only, not proxied, so GitHub can
  issue the certificate). Everything up to the live check can be built without it; the live
  check at the end of the story needs it. Ask for it once, when the Pages setup is merged.
- The existing gate (`scripts/test.sh` + fmt + clippy on ubuntu and macOS) keeps gating
  publishing. Binaries are built from the same commit the crates are published from.
- `scripts/test.sh` is bash and its memory cap uses `systemd-run` (Linux) and a `vm_stat`
  watchdog (macOS). Neither exists on Windows; measuring the Windows pass rate needs its own
  approach, not a port of that cap.

## Fog for the plan (not yet decided)

- Tooling: `cargo-dist` (generates the build matrix, installers, checksums) versus extending the
  existing `publish.yml` by hand. Either way the current gate must stay.
- Trigger: build binaries in the same `publish.yml` dispatch, or on tag push.
- Which version is the first to carry binaries — build them for the existing `v0.3.0` tag, or ship
  them with the next release (and what that version number is).
- Linux aarch64: native arm runner or cross-compilation.
- Pages: deploy from a GitHub Actions workflow or from a folder; whether the site holds anything
  besides the two scripts (for example a landing page that redirects to the README).
- Whether the installer can pin a version (`TYPDOC_VERSION`), and how upgrade and uninstall work.
- Where Windows puts the binary and whether the script edits the user `PATH` or only prints how.

## Done when

- A GitHub release carries the five binaries, their checksums and attestations.
- `https://typdoc.thaitype.dev/install` and `/install.ps1` are live over HTTPS.
- CI installs from the **live URLs** on ubuntu, macOS and Windows runners and runs
  `typdoc --version` successfully.
- The Windows test-suite job reports a pass rate and does not block PRs.
- README, `docs/getting-started.md`, the relevant how-to/reference pages and `skills/typdoc/` are
  updated in the same PR as the behaviour: install one-liners, the browser-download notes for
  macOS and Windows, `gh attestation verify`, Windows marked experimental.

## Repo rules that apply

- Public-text rule: no names of people or agents, and no "who decided" phrasing, anywhere in the
  repo — `.chief/` files, commit messages, PR text, code comments, docs.
- Do not name or link any other project's installer as the model for this one.
- Design changes land in `docs/design/` per `.chief/_rules/_standard/design-docs.md`.
