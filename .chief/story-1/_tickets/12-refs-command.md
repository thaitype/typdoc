# 12: `typdoc refs`

Type: implementation
Status: open
Blocked by: 3, 10, 11

## What this delivers

- Outgoing refs and `--reverse` through a reverse index, `--field` keeping the refs held in that field in both directions, unresolved references with their reason, and the shape and order that JSON output declares.
- A golden for each direction.

## Done when

- A dangling ref is listed with `unresolved` and no `path`, and `path` and `unresolved` never appear together (a test asserts it).
- `refs` leaves `unimplemented_commands`.
