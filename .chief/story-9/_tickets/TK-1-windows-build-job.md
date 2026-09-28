---
blocked_by: []
status: resolved
title: Blocking Windows build job in CI
type: implementation
---

# TK-1: Blocking Windows build job in CI

## What this delivers

A pull request shows a failed check whenever the shipped crates do not build on Windows.

## Scope (contract: CI)

- Job `windows-build` in `.github/workflows/ci.yml` on `windows-latest`:
  `cargo build --locked -p typdoc -p typdoc-core -p typdoc-fs`, no `continue-on-error`.
- `windows-pass-rate` unchanged.

## Checks

Pushed before TK-2, so the job's first run is red with the same six `typdoc-fs` errors as
`windows-pass-rate` reports on `main`; ubuntu and macOS jobs stay green.
