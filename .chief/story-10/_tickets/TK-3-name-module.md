---
blocked_by:
- TK-1
- TK-2
status: resolved
title: 'name.rs: resolve and format, with the round trip'
type: implementation
---

# TK-3: 'name.rs: resolve and format, with the round trip'

## What this delivers

`resolve` and `format`, and the round trip, with no caller yet.

## Scope (contract: One grammar, two functions)

- `crates/typdoc-core/src/name.rs`: `Identity`, the context, `resolve`, `format` (`Path`, `Portable`, `Like`).
- Unit tests for each step of the reading order and each form; the round trip over `fixtures/valid/*` and the TK-1 project. Strict mode.

## Checks

Gates; the round trip shown able to fail with a planted fault.
