---
blocked_by:
- TK-17
status: open
title: The live checks cover install.ps1 without depending on a release
type: implementation
---

# TK-18: The live checks cover install.ps1 without depending on a release

## What this delivers

After a deploy, the live check proves `https://typdoc.thaitype.dev/install.ps1` is byte-identical to the
repository's, as it does for `/install`, which holds whether or not a release has Windows assets. The Windows
install itself is proven where a release exists: `publish.yml`'s live install check gains a `windows-latest`
leg, which runs only after a release is published.

## Scope

- `scripts/check_live_install.py` and its self-test: `install.ps1` byte check.
- `publish.yml`: `verify-live-release` on `windows-latest` runs the PowerShell one-liner.

## Checks

The self-test covers the new check; nothing on `main` installs from a release.
