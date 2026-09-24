# 29: Two defects Aria found running PR #2 for real (release build, fresh project)

Type: implementation
Status: resolved
Blocked by: None (can start immediately)

**Found 2026-09-24, Aria, running commit `1a8b4ee` for real** (a release build against a fresh
project) — two real defects, both fix-on-`story-3-catalog-and-release` so they land in PR #2
(already open, not merged). Do not merge.

## Defect 1 — text `refs` lost the column naming the document at the other end

`typdoc refs WF-1 --reverse` prints:

```
written  field
WF-1     blocked_by
```

It never says **which document** (`WF-2`) is the one pointing at `WF-1` — that's the whole
question `--reverse` answers. `--json` already has it (`path`/`key` in the entry, e.g.
`tickets/WF-2.md`). The design's own worked example always showed the name first
(`chief:WF-7   context`).

**Fix:** put the other document's name first — the same rendering `unrewritten_text` already
uses for this exact concept (`crates/typdoc/src/cli.rs`, search `fn unrewritten_text` and
`fn ref_name_text`): `ref_name_text(name)` for a resolved `RefOutcome`, `"(unresolved: {reason})"`
otherwise — then `field`. Keep `written` as a third column **only for `RefsDirection::Out`**: for
`Out`, `written` can genuinely differ from the resolved name (an alias, a relative form, etc.) so
it adds real information; for `In` (`--reverse`), `written` is just how the holder happened to
write a reference to the document you already asked about — it never tells you anything the
command's own subject doesn't already say, so drop it there (Aria: "keep written only if it adds
something for out").

Header column name for the new first column: **not** `path` or `key` (see Defect 2's reasoning —
a document's identity here is sometimes a `namespace:key` and sometimes a bare path, so neither
single JSON field name is a correct header alone). Use `document` — consistent with how
`RefsReference`'s own doc comment already calls this ("the document at the other end") and how
`ref_name_text`'s doc comment describes it ("A document's identity, text-mode").

So: `Out` direction header is `document  field  written`; `In` direction header is
`document  field` (two columns, no `written`). Both still follow ticket 28's "no header at all
when there is no output" rule and reuse `render_table`/`render_row` (added in ticket 28) rather
than a new rendering path.

## Defect 2 — mixed `list` rows print header `key` over a row holding a path

`list_table`'s current header logic (`crates/typdoc/src/cli.rs`, search `fn list_table`):

```rust
let identity_label = if matched.iter().any(|doc| doc.key.is_some()) {
    "key"
} else {
    "path"
};
```

`any` is wrong for a **mixed** result (some coded documents, some not, e.g. a `list` spanning
multiple collections): if even one matched document has a key, the header claims `key` for
*every* row, including ones printing a bare path like `notes/setup.md` — a header must not claim
a column holds something a row in it plainly doesn't.

**Fix:** three cases, not two:
- every matched document has a key → header `key` (unchanged, homogeneous coded case).
- no matched document has a key → header `path` (unchanged, homogeneous uncoded case).
- a real mix of both → header `document` (new, generic — same reasoning and same word choice as
  Defect 1's fix, for consistency: neither `key` nor `path` alone is accurate, and the cell value
  itself is already "key when coded, else path", the same shape `ref_name_text` produces).

## Tests

- New/updated golden(s) for `refs --reverse` showing the holder's name in the first column, e.g.
  a project with `WF-2` (coded) holding `blocked_by: WF-1`, run as `refs WF-1 --reverse`, expects
  a `document  field` header then `WF-2  blocked_by`.
- A golden for `refs` (forward, `Out`) confirming the three-column form (`document  field
  written`) still shows `written` when it's genuinely informative (e.g. a ref written in a form
  that differs from the resolved canonical name, if such a fixture exists or can be built
  cheaply; otherwise confirm the column is present and correct even when written happens to equal
  the resolved name — the column's presence for `Out` is what's being tested, not that it must
  always differ).
- A new `list` golden spanning a mix of a coded and an uncoded collection in one result, showing
  the header reads `document` (not `key`) and each row shows the right cell value (key or path,
  unchanged per-row logic).
- Confirm the two existing homogeneous `list_table` cases (all-coded, all-uncoded) are unchanged
  — don't just add the new case, re-run/confirm the existing ones still pass.
- Full gate run: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `TMPDIR=/home/thw-home/.cache/typdoc-tmp scripts/test.sh`.

## Docs

- `docs/commands.md`: update the `refs` and `list` sections' worked examples/description to match
  the new shapes.
- `docs/design/spec/SPC-5.md` (or wherever ticket 28 documented `refs`'s header row): update the
  worked example and column description for the `document` header and the direction-dependent
  `written` column.
- `CHANGELOG.md`: this is a fix to a change already listed under `[0.2.0]` for this same
  unreleased version (ticket 28's own line, not yet released) — correct that existing bullet in
  place rather than adding a second, contradictory one for the same still-unreleased version.

## Done

- `refs --reverse`'s first column names the holder document; `refs` (forward)'s three columns are
  `document, field, written`; `refs --reverse`'s two columns are `document, field`.
- `list`'s header never claims `key` for a row holding a path — mixed results get a `document`
  header; homogeneous results are unchanged.
- Docs, spec, and CHANGELOG reflect the corrected shapes (not a second entry alongside the wrong
  one).
- All three gates green.
