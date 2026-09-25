# Ticket 03 Report

## Ticket
`pages/install` and `pages/install.ps1`, satisfying the installer contract, plus the PR-time
local-server test and docs.

## Outcome
done

## Decision
- **Issue:** per-script decision — dist-generated vs. hand-written.
- **Verified empirically** (installed cargo-dist 0.32.0, ran a real local build, read dist's own
  generated installer output) rather than assumed: both dist-generated scripts bake one fixed
  version at *generation* time with no runtime version-selection mechanism at all (no
  `TYPDOC_VERSION`/`releases/latest` equivalent), use different env-var names than the contract
  requires (`TYPDOC_DOWNLOAD_URL`/`TYPDOC_INSTALL_DIR` vs. the contract's
  `TYPDOC_INSTALL_BASE_URL`/`INSTALL_DIR`), and PowerShell's default install dir is
  `$HOME\.cargo\bin`, not the contract's fixed `%LOCALAPPDATA%\typdoc\bin`.
- **Chosen:** both scripts hand-written — dist's generated installers can't be configured to
  satisfy the contract at all, on either script, so there was no split decision to make. This
  also **closes ticket 01's flagged gap** (no build-time key to disable dist's PATH-editing
  installer): since this repo's pipeline (`dist build --artifacts=local`) never generates those
  installer scripts in the first place, there's nothing to disable — now stated explicitly in
  `dist-workspace.toml`'s comment.

## Notes
- New fixture-server-based local test harness (`scripts/installer_fixture_server.py` + its own
  unit tests, TDD'd first) simulates GitHub Releases shape for pre-merge testing; new
  `installer-check.yml` PR workflow runs it as a 3-OS matrix job, reusing ticket 01's
  `dist-build.yml` for real target archives. Doesn't touch the existing `gate`.
- Docs updated: README Install section (one-liners, `INSTALL_DIR`, `gh attestation verify`,
  macOS quarantine note, Windows SmartScreen note), `docs/getting-started.md`, and
  `skills/typdoc/SKILL.md`'s stale `0.3.0` references bumped to `0.3.1`.
- Review pass caught and fixed: missing action-version-pin rationale comments in the new
  workflow, and `docs/getting-started.md` initially left untouched despite being named in the
  ticket.
- **Caveat, not fixed here:** the full `scripts/test.sh` gate hit a pre-existing sandbox linker
  crash (Bus error under the sandbox's memory cgroup) unrelated to this diff (zero `.rs`/
  `Cargo.toml` files touched) — not re-verified locally; the PR's real CI will run it for real.
- Commit `4fdc0af` on `story-5-prebuilt-installer` (rebased cleanly, no conflicts). Public-text
  grep clean, checked per-file.
