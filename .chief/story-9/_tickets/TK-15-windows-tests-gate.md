---
blocked_by:
- TK-13
- TK-14
status: claimed
title: The Windows tests job is a gate
type: implementation
---

# TK-15: The Windows tests job is a gate

## What this delivers

A pull request is red when a test fails, or does not compile, on `windows-latest`.

## Scope

- `ci.yml`: `windows-pass-rate` becomes `windows-test` (Windows tests), whose last step fails when `cargo test`
  did; the report still runs and names targets that did not compile.
- Test-only Windows forms decided in the map: the shell examples are Unix only (SPC-13 says so); the config
  collection-name test leaves out `a:b`.

## Checks

`Windows tests` green on the pull request, and shown able to fail.
