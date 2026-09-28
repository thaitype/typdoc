---
blocked_by: []
status: claimed
title: Prepare the Windows docs for the release branch, not this pull request
type: implementation
---

# TK-19: Prepare the Windows docs for the release branch, not this pull request

## What this delivers

A file in this story holding the README, `docs/how-to/install.md` and `cargo install` changes that say Windows is
supported and show the PowerShell one-liner, to be applied on the first release branch whose release has the
Windows assets. Nothing on this pull request says Windows is supported.

## Scope

- `.chief/story-9/windows-docs-for-release.md`.

## Checks

README and `docs/how-to/install.md` on this pull request still say Windows is not supported yet.
