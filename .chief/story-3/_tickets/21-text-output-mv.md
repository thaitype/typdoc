# 21: Text output for both forms of `mv`, and `mv --json`'s new `rewritten` field

Type: implementation
Status: claimed
Blocked by: 16

M-10(e) and M-10h's answers, built together since both forms of `mv` get the identical new
shape. Blocked by 16 for the shared labeled-block renderer.

## The work

1. Both `mv` (plain) and `mv --renumber` without `--json` print: the destination document's
   labeled block (ticket 16's renderer) — **replacing**, for `--renumber`, today's built bare-key
   output (a deliberate, accepted change, M-10h); then, always present as their own lines:
   - `rewritten: N refs in M documents` — a count only, in text.
   - `unrewritten:` — its own count, then one line per entry naming the project, document, field,
     and written form of the ref that wasn't rewritten (matching what `mv`'s existing report
     already tracks for this); `none` if there are none.
   - `findings:` — listing entries or `none`, from the same schema-satisfaction check `mv`'s
     `--json` output already carries.
2. `mv --json` (both forms) gains `rewritten`: the full list behind the text count above, one
   entry per rewritten ref, each `{document, field, before, after}`. Additive — every field
   `mv --json` prints today keeps printing, unchanged.
3. No `--verbose` flag is added (M-10h) — this ticket does not add one.
4. Fix the hard-coded `true` in both forms' error paths (`cli.rs:408-...`, both branches) to each
   command's real `json` variable.

## Tests (see `testing-decisions.md`, "Text output")

Both forms of `mv`, each case below, in text and in `--json`:
- A move that rewrites at least one ref: `rewritten:`'s count is correct; `mv --json`'s
  `rewritten` list has one entry per actual rewritten ref, each field correct.
- A move that leaves at least one ref unrewritten (e.g. a ref from another project, or with
  `body.links` off): `unrewritten:`'s count and per-entry line match the actual case, not a
  placeholder.
- A clean move with nothing to rewrite: `rewritten: 0 refs in 0 documents`, `unrewritten: none`.
- An error case (a destination that already exists, for plain `mv`) asserts plain-text stderr
  without `--json`.

## Done

- Both forms of `mv` produce the shape above without `--json`; neither refuses with "not built
  yet."
- `mv --json` (both forms) carries `rewritten` as the full list, additive to its existing shape.
- Both forms' error paths print plain text without `--json`.
