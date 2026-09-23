# 8: Text output (no `--json`) — the shapes `design.md` doesn't already fix (M-8)

Type: wayfinder:grilling
Status: resolved — M-10(a-h) and M-10g all decided
Blocked by: None (can start immediately)

**Decider: Mild.** M-10(a-f), M-10g, the field-names principle, and (once split out and checked
against `design.md`) M-10h are all decided (Mild: *"ตามข้อเสนอได้เลย"*, then *"ดีคับ"* /
*"เห็นด้วยคับ"* for M-10h, 2026-09-23) — see Answer.

**M-10, principle (Mild, 2026-09-23):** *"อยากให้ เวลาที่ไม่ใช้ JSON แล้วยังเห็นชื่อ field อยู่นะ
ไม่งั้น งงเลย"* — text output always shows field names; no bare, unlabeled value. This constrains
every shape M-10(a-f) can land on, and it reopened one shape already thought settled:

**M-10g, resolved (Mild: *"ดีคับ เห็นด้วย"*):** `list`'s text output — already built, previously
listed below as needing no change — gains a header row: `path` (or `key` for a coded collection),
then `title`, then each `--where` field, in that order. No header when the result is empty (exit
0, as today, unchanged). `list --ids` is unchanged: one key or path per line, no header, since
it's built for piping into another command. This changes the already-built `list_outcome` and its
golden tests — contract material for `/chief-plan`, not a new ticket.

Aria's own proposal, not yet Mild's answer, for M-10(a-f): the write commands (`get`/`set`/`new`)
print the same labeled block; `toc` is a table with a header row.

**Raised by Mild (M-8), 2026-09-23:** *"M-8: output แบบไม่ใส่ --json ยังไม่ได้ทำ จะเอาเข้า v0.2.0"*
— text output goes into v0.2.0. Today every command without `--json` answers `the output without
--json is not built yet` and exits 1 (`crates/typdoc/src/cli.rs:211, 248, 263, 332, 351, 394,
428`).

**Aria's instruction:** where `design.md` already fixes the text shape per command, it's
contract material, not a ticket; where it doesn't, that's fog. This ticket carries only the
fog — the open questions — for Aria to route (format is a UX decision; some may be Mild's).

## Already spec'd — contract material, not part of this ticket

- **`list`** — already built (`list_outcome` in `cli.rs`). `design.md`'s own shape ("a table of
  key or path, `title`, and every field used in `--where`", plus `--ids`) is superseded on one
  point by M-10g above: a header row is added; everything else about the shape stands.
- **`refs`** — currently gated ("not built yet", `cli.rs:332`) but `design.md` already has a full
  worked example (`typdoc refs precedents/secret-handling.md --reverse` → `chief:WF-7   context` /
  `learnings/x.md   $body`). Each row already names its field inline (`context`, `$body`), so it
  already satisfies M-10's field-names principle without change. Building it is mechanical:
  implement the shown format.
- **`mv --renumber`** — was built matching the design's bare-key shape; **M-10h changes it** (see
  Answer) to the same shape as plain `mv`. No longer contract-material-as-is — it moves into the
  decided shapes below.
- **`validate`, plain and `--schemas`** — currently gated (`cli.rs:394`, for the non-`--audit`
  branch). `design.md`'s worked-examples table says the shape at a level a build ticket can act
  on directly: "One line per finding", vs. `--audit`'s already-built "summary by collection and
  rule first, then details". The finding's fields are already fixed by the existing `--json`
  shape (referenced from Audit mode's JSON description). Treated as contract material: render one
  line per finding from that existing shape; no open UX question blocks it.

## Answer

**M-10(a-f), decided (Mild: *"ตามข้อเสนอได้เลย"*), all under the field-names principle** (no bare,
unlabeled value):

- **(a) `get`:** a labeled block, one `name: value` per line — `path`, `collection`, `schema`,
  `namespace` (and `key` when coded), then frontmatter fields in file order. A `number` field
  prints with the document's own digits (not a converted-out value), matching the write path's
  existing `[number-text]` rule.
- **(b) `set`, (d) `new <path>`, (f) `new`, coded form:** all three print the same labeled block
  as `get`, for the document as it stands after the write. (f) deliberately changes today's
  built, design-documented behavior (a bare key, `# stdout: WF-3`) — accepted knowingly, not an
  oversight: a caller that wants just the key uses `--json` and reads it out, the same as any
  other field.
- **(c) `toc`:** a table with a header row, `line  end  level  heading`, one row per heading.
- **(e) `mv`, plain form:** the destination's `get`-shaped block, then, always present as their
  own lines: `rewritten: N refs in M documents` (a count); `unrewritten:` with its own count and
  one line per entry (project, document, field, written form); `findings:`, listing its entries
  or the word `none`.

**M-10h, decided (Mild: *"ดีคับ"* / *"เห็นด้วยคับ"*, 2026-09-23).** The flag on `mv --renumber` was
warranted — the claim that it "follows the same shape" as plain `mv` was Aria's own addition when
writing up M-10(e), not something Mild had actually seen. Checked against `design.md`
(§`typdoc mv`), which gives `--renumber` its own explicit worked example (`# stdout for
--renumber: WF-4`) and states its scriptability rationale outright. Decided anyway: `mv
--renumber` uses the same labeled block as the other write commands, not the bare key. Both `mv`
forms additionally print the move summary above — `rewritten:`/`unrewritten:` — new for both,
not only for `--renumber`. `mv --json` gains `rewritten` as the **full list** — one entry per
rewritten ref, each `{document, field, before, after}` — additive to its existing shape; text
prints only the count. **No `--verbose` flag**: the detail lives in `--json`, and `git diff`
already shows every changed line for a human at the terminal.

This is a design change (per project rule 6, `design.md` would normally need to change first —
but M-7 already moves and freezes it this same story), so the correct, current shape is recorded
in `docs/design/spec/` and in the user docs that show `mv`'s output, not left only in this
ticket or silently deviated from an archived document.

## `list_outcome` change already covered

See ticket 8's original M-10g note — `list` gains a header row, `--ids` unchanged. That answer
stands unchanged by this update.
