# 17: Text output for `toc`

Type: implementation
Status: open
Blocked by: None (can start immediately)

## The work

`toc` without `--json` prints a table with a header row: `line`, `end`, `level`, `heading`; one
row per heading, in document order (`cli.rs:345-353`, replacing the refusal at `cli.rs:351`).
Fix the error path's hard-coded `true` (`cli.rs:355` area) to the command's real `json` variable.

## Tests

- A hand-written golden against a fixture document with headings at more than one level,
  checked for the header row and correct column alignment/labeling.
- A document with no headings: **decided (Aria, 2026-09-23), following `list`'s own precedent —
  no header, nothing printed, the exit code (0) carries the result.** Not a free choice; confirm
  this, don't re-decide it. If a future build session thinks this is wrong for `toc`
  specifically, it flags to Aria rather than picking something else.
- An error case (a document that doesn't exist) asserts plain-text stderr without `--json`.

## Done

- `toc` produces the header-rowed table without `--json`; no "not built yet" refusal remains.
- Its error path prints plain text without `--json`.
