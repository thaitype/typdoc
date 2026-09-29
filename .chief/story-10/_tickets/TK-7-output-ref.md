---
blocked_by:
- TK-3
status: open
title: Every document printed with its portable name, ref
type: implementation
---

# TK-7: Every document printed with its portable name, ref

## What this delivers

Every document in `--json` and in text output gains `ref`, labelled, beside `path`; an import's documents print their `alias::` (D5, D9, D11).

## Scope (contract: Output)

- `cli.rs` `document_name`, `document_json`, `reference_json`, `identity_text`, `ref_name_text`, `document_text`, list and refs tables; golden files gain `ref`. Strict mode.

## Checks

Gates; `path` byte-identical in every golden file.
