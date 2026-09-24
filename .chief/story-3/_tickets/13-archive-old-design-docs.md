# 13: Archive the old design documents

Type: implementation
Status: claimed
Blocked by: None (ticket 12 resolved 2026-09-24)

**Unblocked 2026-09-24.** Ticket 12 (rewiring the three catalog-document readers, deleting
`design.rs` and `typdoc-testkit/src/shell_examples.rs`'s extraction mechanism) is merged into
`story-3-catalog-and-release` — confirmed via `rg 'design_text'` / `rg '\bdesign\.rs\b'` /
`rg 'typdoc_testkit::shell_examples'` all returning nothing outside history, per ticket 12's own
report. Nothing left reads `docs/design/design.md` as data; this ticket can move it now.

## The work

1. `git mv docs/design/design.md`, `git mv docs/design/design-decision-phase-1/`, and
   `git mv docs/design/design-decision-phase-2/` into `docs/archived-design/`, preserving their
   internal structure. Nothing inside them is edited — not even a link that now points at a moved
   path, since the archive is a frozen record of what was decided and when, not a maintained
   document (M-7, restated directly by Mild for `design.md`; the same treatment applies to both
   decision registries, per M-7's original wording).
2. Copy (not move) the same three into `docs/migrating-design/` — a working copy. Nothing is
   deleted from it by this ticket; ticket 22 (and any later work) deletes text from it as that
   text's content moves into `docs/design/spec/` or `docs/design/catalog/`. Emptying it fully is
   not required this story.
3. Confirm nothing outside `.chief/story-1/` and `.chief/story-2/` (which keep the old path in
   their own frozen build records, on purpose, per the M-7 discussion) still points at
   `docs/design/design.md`, `docs/design-decision-phase-1/`, or `docs/design-decision-phase-2/` by
   that path. Update any live reference (README, `docs/`, `.chief/project.md`) to the new
   `docs/archived-design/` path if it's pointing at history, or to `docs/design/spec/`/`catalog/`
   if it's pointing at something this story's later tickets will put there.

## Done

- `docs/design/design.md`, `design-decision-phase-1/`, `design-decision-phase-2/` exist only
  under `docs/archived-design/`.
- `docs/migrating-design/` holds an identical copy of all three, untouched by this ticket.
- No live document points at the old paths; `.chief/story-1/` and `.chief/story-2/`'s own records
  are left as they were.
