# Goal

Install typdoc without a Rust toolchain: prebuilt binaries attached to every GitHub release, and
a one-line installer served from this repo's own domain — alongside the existing
`cargo install typdoc` path, which stays supported. macOS and Linux only — see "Windows" below
for why.

The story ends with **release 0.3.1** published through `publish.yml` carrying all four binaries,
their checksums, and their attestations — the first release to do so. Cutting that release is an
owner action after merge, not loop work — see "After Merge" below for the exact order.

From a user's perspective:

- `curl -fsSL https://typdoc.thaitype.dev/install | sh` installs typdoc on macOS or Linux
  (x86_64 or aarch64) with no Rust toolchain required.
- Windows is not supported yet, by this installer or by `cargo install` — see "Windows" below.
- The script detects OS/architecture, downloads the matching release asset, verifies its
  SHA-256 before installing, and fails loudly (never installs an unverified file) on a checksum
  mismatch or an unsupported platform. Default install location is `~/.local/bin`; `INSTALL_DIR`
  overrides it.
- Every released binary carries a SHA-256 checksum and a GitHub artifact attestation
  (`actions/attest-build-provenance`), so provenance can be verified independently of the
  installer (`gh attestation verify`).
- `TYPDOC_VERSION` lets the installer pin a specific release instead of installing latest.
- Each binary is executed (`typdoc --version`) in CI on a runner of its own architecture; the one
  target that cannot be run in CI (`x86_64-apple-darwin`, no matching-arch GitHub-hosted runner
  available to this repo) is named explicitly as built-but-not-run in the release notes — a
  passing cross-compile is never presented as a binary that runs.
- CI installs from the **live URL** (`https://typdoc.thaitype.dev/install`), not repo files, on
  ubuntu and macOS runners, and runs `typdoc --version` on each.
- `https://typdoc.thaitype.dev/` itself (no path) redirects to this GitHub repo, via a root
  `index.html` — the domain is never a bare 404.
- The Windows test suite runs in CI on every relevant change, as a non-blocking job. It does not
  yet report a pass-rate baseline; see "Windows" below for what it reports instead, and why.
- README, `docs/getting-started.md`, the relevant how-to/reference pages, and `skills/typdoc/`
  are updated in the same PR as the behaviour they describe.

## Windows

`typdoc-fs`, the crate implementing `typdoc_core::Fs`, is Unix-only by design: it uses
`std::os::unix::fs::{MetadataExt, PermissionsExt}` unconditionally, with no Windows branch at
all, so a Windows binary cannot compile. Windows becomes its own later story — no release
target, no installer, no documented support here. The Windows CI job this story adds keeps
measuring where Windows currently stands rather than skipping the platform entirely: supporting
Windows is two steps, compiling and then a pass rate, and today's job reports that the build is
stuck at step one — it does not compile — instead of a pass-rate number (see the contract's "CI:
Windows test-suite pass-rate job" and "Testing Decisions").

## Out of Scope

- Paid code signing or notarization. Documented workaround only: `xattr -d
  com.apple.quarantine` for a macOS binary downloaded through a browser.
- Making the Windows build compile, fixing any Windows test failure, or reporting a pass rate —
  this story's Windows job reports the current state (see "Windows" above) and nothing more;
  everything past that, starting with making the build compile, is its own later story.
- Comment-review — deferred to a later story.
- Editing `PATH` or any shell rc file, on any OS, from the installer or from cargo-dist's own
  generated installer (its path-modification behaviour is turned off). The installer prints the
  one line to run if the install directory isn't already on `PATH`.
- Separate upgrade/uninstall tooling: upgrade is re-running the installer; uninstall is a
  documented manual removal of the installed binary.

## Needs from outside the repo

Required for the story to be truly done, not optional — just not performable by a commit in this
repo:

- **Pages source / custom-domain setting**, set in repo settings on this public repo: done.
  Source is GitHub Actions, custom domain `typdoc.thaitype.dev`, HTTPS enforced (owner-verified
  via API). No longer outstanding.
- **Cloudflare DNS record** for the install domain (`CNAME` to `thaitype.github.io`, DNS-only):
  outside this repo's automation. Already live, confirmed externally resolving to
  `thaitype.github.io` — no longer outstanding.
- **`github-pages` deploy environment**: the story branch (`story-5-prebuilt-installer`) has been
  added to its allowed deploy branches, temporarily, so the full flow can be proven before merge
  (see "Full-flow proof" below and the pages.yml trigger change in ticket 04). Removing it again
  is an After Merge item.

## Full-flow proof (after the loop, before the PR is marked ready)

Pages is now configured (source: GitHub Actions, custom domain `typdoc.thaitype.dev`, HTTPS
enforced) and deploys on push to the story branch too, temporarily — so the full install flow
can be proven end-to-end before the PR is marked ready, not just after merge:

1. A GitHub **pre-release `v0.3.1-rc.1`**, carrying the four binaries, their checksums, and their
   attestations — no crates.io publish. Produced by a workflow triggered by a push (e.g. an rc
   tag), not a hand upload, so attestations are generated correctly; reuses ticket 01's reusable
   build workflow as its build step.
2. Once that pre-release exists: run the live install one-liner on ubuntu-latest,
   ubuntu-24.04-arm and macos-latest with `TYPDOC_VERSION=v0.3.1-rc.1` set, confirming each
   installs and runs. `releases/latest` skips pre-releases, so this proves the full flow without
   touching what a default (no-`TYPDOC_VERSION`) install resolves to.

Done: `.github/workflows/release-rc.yml` (tag-triggered) and
`.github/workflows/full-flow-proof.yml` (dispatchable, version-parameterized). The pre-release is
at `https://github.com/thaitype/typdoc/releases/tag/v0.3.1-rc.1` (four archives + checksums,
`isPrerelease: true`, `gh attestation verify` confirmed against a downloaded archive); the live
install proof is workflow run `36145328017`, all three legs (`ubuntu-latest`,
`ubuntu-24.04-arm`, `macos-latest`) succeeded.

## After Merge (owner actions, not loop work)

The loop's tickets end at "PR open, CI green," and the full-flow proof above happens once more
after that, before the PR is marked ready. Everything below happens after the PR merges:

1. **Merge** the PR to `main`.
2. **Remove the story branch** from `pages.yml`'s trigger and from the `github-pages` deploy
   environment's allowed deploy branches — both were temporary, added only for the full-flow
   proof above. Also remove `full-flow-proof.yml`'s `push` trigger (its own header comment
   explains why it's there and that it's safe to drop once `workflow_dispatch` is reachable the
   normal way, which merging to `main` provides).
3. **Pages deploys** on the push to `main`; the domain continues resolving over HTTPS as it did
   during the full-flow proof.
4. **Owner dispatches `publish.yml`** for the real `0.3.1` — dry run first, then the real run,
   same pattern as the last story's release. This is the first time crates.io is touched.
5. **Post-release live check** runs as part of that same dispatch (ticket 05's post-release
   step) — confirms the live install one-liner works against the actual `0.3.1` release, not
   just the `v0.3.1-rc.1` pre-release.
6. **Delete the `v0.3.1-rc.1` pre-release and tag** once the real `0.3.1` release exists — it
   was only ever a proof artifact, not something meant to stay listed alongside real releases.
