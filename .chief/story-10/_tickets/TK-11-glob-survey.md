---
blocked_by:
- TK-2
status: resolved
title: Survey by running how each place reads a glob today
type: implementation
---

# TK-11: Survey by running how each place reads a glob today

## What this delivers

A table, measured on the binary, of what `*`, `**`, `,`, `!`, `\` and a leading dot do in each place that takes a
pattern: config `namespaces`, `--namespace` and `TYPDOC_NAMESPACE`, collection `match`, `--where` and `--if`
values, the body link `ignore`; and the differences that have no stated reason, each with a recommendation. No
behaviour changes.

## Checks

Every cell run, not read from code; the differences reported to the director before the spec is written.
