---
title: Text output names its fields
status: active
---

**Text output shows the name of every field it prints.** A value without its name is not
printed.

## Why

A bare value is ambiguous as soon as there are two of them: a person reading `open 3` cannot tell
which is the status and which the count. Printing the name costs a few characters and removes the
guessing.

## What follows

- `get` prints one `name: value` line per field; `list` prints a header row naming its
  columns (`SPC-5`).
- Write commands, `mv --renumber` included, print the same labeled block as `get` for the
  document they wrote; a caller that wants a bare key reads it from `--json` (`SPC-2`).

## Where it stops

`list --ids` prints one key or path per line and nothing else, because its purpose is to be piped.
