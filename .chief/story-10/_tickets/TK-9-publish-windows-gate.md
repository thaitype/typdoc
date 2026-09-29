---
blocked_by: []
status: resolved
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

## Evidence

Throwaway pull request #35 (closed, branch deleted) ran the gate job exactly as publish.yml has it:
red with a planted failing Windows test, run 36516652557 (windows-latest failed on it, ubuntu and
macOS green); green with the plant removed, run 36517039758 (all three legs; Windows 1138/1138).
