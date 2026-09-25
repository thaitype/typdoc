# Ticket 6 Report

## Ticket
Migrate story 4's design changes out of `docs/migrating-design/design.md` into
`docs/design/spec/`/`docs/design/catalog/`, per `.chief/_rules/_standard/design-docs.md`.

## Outcome
done

## Decision
- **Issue, round 1:** my first pass at this ticket (before the standard's rule file landed on
  this branch) edited several sentences in place with corrected wording and pointers, which
  showed as replace-in-place in `git diff --numstat`, not pure deletions.
- **Issue, round 2:** after fixing round 1 to use whole-section deletion for the four sections
  that were entirely superseded (Project row, Discovery, Nested projects, Namespaces), two facts
  were left untouched in `docs/migrating-design/design.md` — the `config.state-orphan` exclusion
  nuance and the `mv` same-path message split — reasoning that migrating their entire surrounding
  paragraphs (State's numbering mechanics; the rest of `mv`) was disproportionate to what story 4
  changed. Flagged this call rather than deciding it was obviously right.
- **Answer:** the standard's unit is the *section*, not the sentence — leaving `migrating-design`
  and the newer SPCs disagreeing about the same topic is exactly what the standard exists to
  prevent, disproportionate or not. Also flagged: a third table, the **Config errors** id table
  (~20 rows, separate from the State/mv sections), still had the old `config.state-orphan`
  wording and should be checked against `SPC-6`/the catalog and deleted if redundant.
- **Chosen:** did the full migration — new `SPC-8` ("State and numbering explained") for the
  entire State topic, `SPC-2` expanded with the entire `### typdoc mv` section (not just the
  message-split paragraph already there), and the whole **Config errors** section deleted after
  verifying, programmatically (id-by-id, text-by-text), that `docs/design/catalog/config-errors.md`
  already covers every row with no gaps. All three now delete cleanly from
  `docs/migrating-design/design.md` with no replacement text.

## Notes
- Both `SPC-7` and `SPC-8` were created with `typdoc new SPC`, not written by hand — confirmed
  `.typdoc/state/default.json`'s `spec.last` tracked correctly at each step (6→7, 7→8) rather than
  assuming it. `typdoc set` filled `status`/`migrated_from` afterward, since a write only touches
  frontmatter; the body was added separately.
- Verified the Config errors table was genuinely redundant before deleting it, not just
  assumed: a script compared the 20 ids and their `reported_when` text between
  `docs/migrating-design/design.md`'s table and `docs/design/catalog/config-errors.md` — no id
  only on one side, no wording mismatch beyond the one row (`config.state-orphan`) already known
  to differ (the catalog has the corrected, current text; the table had the stale one, which is
  exactly why it needed to go).
- Deleting three whole sections broke three cross-references elsewhere in the file that story 4
  never otherwise touched: two `(see State)` pointers (in `typdoc new`'s own paragraph, and in the
  Validation rules table's `state.missing` row) and one `(see Discovery)` pointer (in the Commands
  section's intro). Repointed all three to the new SPC files rather than leaving them dead — the
  one deliberate exception to "deletions only," on the reasoning that a broken internal link is a
  worse outcome than a minimal, unavoidable 3-line fix directly caused by this ticket's own
  deletions, not by touching unrelated content.
- `typdoc validate` on the repo exits 0 at every stage checked — not inferred from the state file
  numbers looking right.
- `git diff <merge-base> -- docs/migrating-design/design.md`: 3 insertions, 106 deletions.
- Full workspace suite unaffected (no product code changed): 1060 tests still passing. Public-text
  gate run scoped to every touched file: clean.
- No code, tests, or fixtures touched — this ticket is entirely `docs/`/`.chief/`/`.typdoc/state/`
  content.
