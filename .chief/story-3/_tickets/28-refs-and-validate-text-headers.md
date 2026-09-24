# 28: M-16 — header rows for text `refs` and text `validate`

Type: implementation
Status: claimed
Blocked by: None (can start immediately)

**Mild's decision (M-16), relayed by Aria, 2026-09-24, at final contract review of the whole
story.** Aria's own miss, caught at review: text `refs` prints `b.md   see` and text `validate`
prints `notes/b.md  error  <message>  refs.resolve` with no header — against the field-names
principle this story has otherwise applied everywhere else `--json`'s field names have a text
counterpart (M-10g, `list`'s header row from ticket 18, `toc`'s from ticket 17).

## The work

1. **Text `refs`** (`refs_text` in `crates/typdoc/src/cli.rs`, ~line 1737): add a header row,
   same conventions `list_table`/`toc_table` already use — column-aligned (padded to the widest
   entry in each column, two spaces between columns, per those functions' own established
   pattern), header omitted entirely when there are no refs (same "none when empty" rule
   `list_table` documents for `listed_len == 0`). Column names: `written` and `field`, matching
   `reference_json`'s own field names (the field-names principle — don't invent new header
   labels for the same two values `--json` already names). Read `list_table`'s full doc comment
   and implementation first and reuse its column-width/render approach rather than writing a
   third, slightly-different table renderer — extract a shared helper if that's cleaner than
   duplicating, but don't invent a fourth rendering convention either.
2. **Text `validate`** (`validate_text` in `crates/typdoc/src/cli.rs`, ~line 1230): add a header
   row, same conventions, same "none when empty" rule (already true today — 0 findings prints
   nothing; keep that, just add the header for the non-empty case). Reorder the columns to
   `path, level, rule, message` (`rule` moves before the long `message` column — currently it's
   `location, level, message, rule`). Header labels: `path`, `level`, `rule`, `message` — matching
   `finding_json`'s own field names exactly (note `finding_json` uses `path` alone; the current
   `location` variable name in `validate_text`, which prints `path:line:col` when a position
   exists, should get a header of `path` too, since that's what the column fundamentally is per
   `finding_json`'s naming — don't call the header `location`).
3. Update every golden/expected-output test in `crates/typdoc/tests/refs.rs` and
   `crates/typdoc/tests/validate.rs` (and anywhere else asserting the raw text shape of either
   command's stdout — search first, don't assume it's only those two files) to expect the new
   header row and, for `validate`, the new column order.
4. Update `docs/design/spec/SPC-*.md` (whichever one documents these two commands' text output —
   find it, don't guess) and any user-facing docs (`README.md`, `docs/commands.md`, or wherever
   worked examples of `refs`/`validate` text output are shown) to reflect the new header rows and
   the new `validate` column order.
5. Update the `CHANGELOG` (find its current v0.2.0/Unreleased section) — the existing line(s)
   about `refs`/`validate`/`toc`/`list` text output gaining headers should now also cover these
   two, or a new line should be added alongside them, whichever the changelog's existing
   convention for this story's other text-output entries suggests.

## Tests

- New/updated golden tests confirming both commands print a header row when there is output, and
  nothing (not even a bare header) when there is none.
- `validate`'s new column order confirmed in at least one golden with multiple findings spanning
  different rules, so the reordering is actually exercised, not just single-column cases.
- Full gate run: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `TMPDIR=/home/thw-home/.cache/typdoc-tmp scripts/test.sh`.

## Done

- Text `refs` and text `validate` both print a header row (column names matching `--json`'s own
  field names) when there is output, and nothing when there isn't.
- `validate`'s text columns are ordered `path, level, rule, message`.
- Every golden, spec/ page, and user doc showing either command's text output reflects the new
  shape; the CHANGELOG names this change.
- All three gates green.
