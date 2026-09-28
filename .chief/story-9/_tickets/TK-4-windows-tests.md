---
blocked_by:
- TK-3
status: open
title: Tests compile and run on Windows
type: implementation
---

# TK-4: Tests compile and run on Windows

## What this delivers

The Windows pass-rate job reports a real number, and the list of what fails there, and why.

## Scope (contract: part 2, Tests on Windows)

- Test code and `typdoc-testkit`: a test about a Unix-only behaviour is `cfg(unix)` with its reason; a test
  whose behaviour should hold on Windows gets a Windows form.
- The pass-rate report no longer says the build does not compile when only the tests do not.

## Checks

`cargo check --tests --target x86_64-pc-windows-gnu --workspace` clean; Linux and macOS test counts unchanged;
the pass-rate job's number and failing list on the pull request.
