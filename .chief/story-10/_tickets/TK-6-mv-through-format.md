---
blocked_by:
- TK-3
status: resolved
title: mv rewrites through format, and keeps a body link's form
type: implementation
---

# TK-6: mv rewrites through format, and keeps a body link's form

## What this delivers

`mv` rewrites every ref through `format(Like)`, and keeps a body link's `./`, anchor and encoding, every link on a line (the 0.6.0 bug).

## Scope (contract: mv)

- `mv.rs` `sibling_prefix`, `key_form`, `namespace_of`, `folder_of`, `rewritten_path_ref`; `mv_reverse_mentions` (D10). Strict mode.
- The explicit test of the 0.6.0 bug.

## Checks

Gates; the bug's test red on the old code, green after.
