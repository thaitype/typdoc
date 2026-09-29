---
blocked_by:
- TK-99
status: resolved
title: Refs and body links read through resolve
type: implementation
---

# TK-4: Refs and body links read through resolve

## What this delivers

Frontmatter refs and body links read through `resolve`: D1 already so, D2, D3, D6, D10's mentions.

## Scope (contract: One grammar, two functions; Output)

- `refs.rs` `classify`, `classify_body`, `resolve_into_import`, `resolve_into_project`; `project.rs` `mention_missing`, the `auto: moves` reading. Strict mode.
- TK-1 rows that change, changed in the same commit.

## Checks

Gates; `name_forms.rs`; planted faults.
