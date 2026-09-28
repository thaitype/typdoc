---
blocked_by:
- TK-16
status: claimed
title: pages/install.ps1, tested on windows-latest against the fixture server
type: implementation
---

# TK-17: pages/install.ps1, tested on windows-latest against the fixture server

## What this delivers

A hand-written PowerShell installer mirroring `pages/install`: the version resolved at run time (the latest
release, or `TYPDOC_VERSION`), `INSTALL_DIR`, the checksum verified before anything is installed, PATH never
edited (it says how). A release with no Windows asset, the latest or a pinned one, fails with a clear message
naming the version and installs nothing.

## Scope

- `pages/install.ps1`.
- `scripts/test_installer_windows.ps1` and a `windows-latest` leg in `installer-check.yml`, against
  `scripts/installer_fixture_server.py`: default install, `INSTALL_DIR`, a `TYPDOC_VERSION` pin, a corrupted
  checksum, a release without a Windows asset (latest and pinned), the user PATH left unchanged.

## Checks

Every scenario passes on `windows-latest`; nothing is installed in the failing ones.
