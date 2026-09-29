---
blocked_by: []
status: claimed
title: Windows in publish.yml's test gate
type: implementation
---

# TK-9: Windows in publish.yml's test gate

## What this delivers

`publish.yml`'s `cargo test + clippy` runs on Windows too.

## Scope (contract: Windows in publish.yml)

- The Windows leg, as `ci.yml`'s `Windows tests` runs the suite. Standard mode.

## Checks

Shown red on a planted failing test, then green, on a throwaway pull request.
