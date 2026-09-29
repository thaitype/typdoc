---
blocked_by:
- TK-3
status: resolved
title: Arguments read through resolve
type: implementation
---

# TK-5: Arguments read through resolve

## What this delivers

Command arguments read through `resolve`: D1, D2 already so, D8.

## Scope (contract: One grammar, two functions)

- `argument.rs` `Argument::parse`; `project.rs` `resolve`, `resolve_key`, `printed_key`, `resolve_uncoded_target`'s namespace; the scope's prefix. Strict mode.

## Checks

Gates; `name_forms.rs` rows for arguments; Windows tests.
