# 19: Text output for `refs`

Type: implementation
Status: claimed
Blocked by: None (can start immediately)

`design.md` already gives the shape (§`typdoc refs`); this ticket implements it. No design
question is open here.

## The work

`refs` without `--json` prints the worked shape already in the design: one line per ref, the
target (bare key, prefixed reference, or path, in whatever form the ref itself carries) and the
field it was found in, e.g. `chief:WF-7   context` / `learnings/x.md   $body`
(`cli.rs:325-343`, replacing the refusal at `cli.rs:332`). Fix the error path's hard-coded `true`
to the command's real `json` variable.

## Tests

- A hand-written golden matching the design's own worked example, plus a case with `--reverse`
  and a case with `--field`.
- An error case (a document that doesn't exist, or `--field` naming a field the schema doesn't
  have) asserts plain-text stderr without `--json`.

## Done

- `refs` produces the design's shown text format without `--json`; no "not built yet" refusal
  remains.
- Its error path prints plain text without `--json`.
