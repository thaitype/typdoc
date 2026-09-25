# Ticket 6 Report

## Ticket
Migrate story 4's design changes out of `docs/migrating-design/design.md` into
`docs/design/spec/`/`docs/design/catalog/`, per the standard this story is the first to actually
exercise (the working copy had never had anything removed from it before this) — and per
`.chief/_rules/_standard/design-docs.md`, landed on this branch partway through this ticket.

## Outcome
done

## Decision
- **Issue:** the standard's own "Done when" is strict — `git diff` on `docs/migrating-design/`
  must be deletions only, and the rule forbids editing it in place at all. My first pass at this
  ticket (before the standard's file landed) had instead edited several sentences in place with
  corrected wording and pointers, which showed as replace-in-place (one deletion, one insertion
  per touched line) once actually checked against `git diff --numstat`.
- **Options considered:** (1) keep the in-place corrections, accepting a non-pure-deletion diff;
  (2) convert every touched spot into a whole-line/whole-section deletion, expanding the owning
  SPC to comprehensively cover the section even where parts of it didn't change, so the deletion
  has nothing left to leave behind; (3) for facts embedded inside much larger, otherwise-untouched
  paragraphs (the `config.state-orphan` exclusion nuance inside State's numbering paragraph; the
  `mv` same-path split inside `mv`'s locking/ref-rewriting paragraph), do the same — migrate the
  whole surrounding paragraph, which for both cases means writing an entire new topic document
  (state/numbering, or the rest of `mv`) neither of which story 4 otherwise touched.
- **Chosen:** (2) for the four sections that are squarely, comprehensively about one topic
  (Project's folder definition, Discovery, Nested projects, Namespaces) — `SPC-7` now covers all
  of Namespaces, not just the exclusion-related parts, so `docs/migrating-design/design.md`'s
  Namespaces paragraph and its bullets could be deleted as one clean unit. Verified this is
  genuinely a pure deletion, not just "looks like one," by checking `git diff --numstat` after
  each edit (0 insertions each time — git's line-diff recognizes a later untouched line, e.g. the
  next section's own heading, as an anchor once the lines before it are removed intact). Rejected
  (3) as disproportionate — writing a full state/numbering SPC or moving the rest of `mv`'s
  documentation is real, separate work `.chief/_rules/_standard/design-docs.md`'s own "move only
  the sections the change touches" argues against taking on here. Reverted the in-place edits for
  those two sentences back to their exact original wording instead, leaving
  `docs/migrating-design/design.md` genuinely untouched there — the correct, current facts already
  live in `SPC-7` (state) and `SPC-2` (`mv`), which is what actually matters for a reader.

## Notes
- `SPC-7` was created with `typdoc new SPC "Namespaces and project discovery explained"` rather
  than written by hand — `.typdoc/state/default.json`'s `spec.last` was `6`; a hand-written
  `SPC-7.md` would have left it desynced (the next real `typdoc new SPC` would have collided or
  the file would show as unaccounted-for). Confirmed this class of mistake is real, not
  theoretical, by checking state before and after: hand-writing the file first left `last: 6`
  beside an actual `SPC-7.md` on disk; deleting it, running the real command, and setting
  `status`/`migrated_from` with `typdoc set` afterward (writes touch only frontmatter, so the body
  had to be added separately) left `last: 7`, correctly caught up.
- `collections.empty` never existed in the original design, so there was nothing to remove for it
  anywhere — pure addition to `SPC-1`, no `docs/migrating-design/` change needed at all.
- `typdoc validate` on the repo exits 0 — the direct check the standard names, not just an
  inference from the state-file numbers looking right.
- `git diff <merge-base> -- docs/migrating-design/design.md`: 0 insertions, 16 deletions.
- Full workspace suite unaffected (no product code changed): 1060 tests still passing. Public-text
  gate run scoped to every touched file: clean.
- No code, tests, or fixtures touched — this ticket is entirely `docs/`/`.chief/` content.
