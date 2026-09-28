---
blocked_by:
- TK-1
status: resolved
title: Windows branches for the file system seam and signals
type: implementation
---

# TK-2: Windows branches for the file system seam and signals

## What this delivers

The shipped crates build on Windows, and `windows-build` turns green. Linux and macOS behave as before.

## Scope (contract: Where Windows differs; The refusal)

- `crates/typdoc-fs/src/lib.rs`: the Unix code under `cfg(unix)`; a Windows branch for `create_new`,
  `mode`, `set_mode` and the file identity, as the contract decides.
- `crates/typdoc/src/signals.rs`: `install` under `cfg(unix)`; a Windows branch as the contract decides.

## Checks

Gates; `cargo check --target x86_64-pc-windows-gnu -p typdoc -p typdoc-core -p typdoc-fs` clean; a
planted Unix-only call outside `cfg` turns that check red; `windows-build` green on the pull request with
ubuntu and macOS still green.
