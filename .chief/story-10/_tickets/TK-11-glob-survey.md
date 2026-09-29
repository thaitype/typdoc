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

## Answer

Measured on the binary: `namespaces` takes `*` and `!`, and refuses `**`, `?`, `[..]`, `\` and `,` as config
errors; `--namespace` and `TYPDOC_NAMESPACE` take `*` and `,`, and refuse `!`, `**`, `?`, `[..]`, `\`; `match`
takes `*` (matching a leading dot in a file name) and `**` (entering no folder whose name begins with `.`), and
reads `?`, `[..]`, `\`, `!` literally, matching nothing, while `{a,b}` is a config error; `--where` and `--if`
values take `*` (crossing `/`), `,` and `\`, read `**` as `*`, and `?`, `[..]`, `!` literally; the body link
`ignore` takes `*` and `**` (entering dot folders), reads the rest literally, and is matched against the target's
path from the project folder, not the link as written.

Decided: G1, `match` and `ignore` refuse `?`, `[..]`, `\`, `!` and `,` in an entry as a config error, as
`namespaces` does (TK-13). G2, `!` stays with `namespaces`: exclusion is a configuration concern, and
`--namespace` is a selection typed for one command. G3, `\` escapes only in `--where` and `--if` values, the
only place a literal `*` or `,` can be meant. G4, `**` in a value is `*`, since a value has no folders. G5,
`ignore`'s `**` enters dot folders, since it filters targets as written, and walks nothing. G6, `ignore` keeps
matching the path from the project folder; SPC-1 and the reference say so, with `**/assets/**` as the example.
