# Ticket 6 Report

## Ticket
Migrate story 4's design changes out of `docs/migrating-design/design.md` into
`docs/design/spec/`/`docs/design/catalog/`, per the standard this story is the first to actually
exercise (the working copy had never had anything removed from it before this).

## Outcome
done

## Decision
- **Issue:** deciding the new SPC structure — one document covering namespaces/discovery/model,
  or splitting further, and where the smaller items (`collections.empty`, the `mv` message,
  `config.state-orphan`'s excluded-namespace nuance) should land.
- **Options considered:** a dedicated SPC for each small item vs. folding them into the existing
  topic-scoped document that already owns that concern.
- **Chosen:** one new document, `SPC-7.md` ("Namespaces and project discovery explained"), for
  the bulk of the design surface (project/folder definition, discovery, nested projects, and the
  full wildcard-exclusion behavior) — this is genuinely new topic surface with no existing home.
  The three smaller items each already have an owning document: `collections.empty` is a
  validation rule (`SPC-1`), the `mv` message is command behavior (`SPC-2`), and
  `config.state-orphan`'s condition is a catalog entry (`config-errors.md`) with its full
  reasoning folded into `SPC-7` since it's fundamentally about what "excluded" means. Splitting
  further would have scattered one coherent topic (namespace exclusion) across several thin
  documents for no benefit.

## Notes
- For `docs/migrating-design/design.md`, only replaced whole paragraphs with a pointer where the
  new spec document now fully supersedes them (the Model row, Discovery, Nested projects, the
  Namespaces paragraph) — everywhere else, only the specific stale sentence or clause was edited
  in place, leaving the surrounding paragraph's still-accurate, story-4-untouched content alone
  (the `config.state-orphan` sentence sits inside a much larger State paragraph about numbering
  mechanics; the `mv` same-path sentence sits inside a paragraph about locking and ref-rewriting).
  Nothing outside what story 4 actually changed was touched.
- `collections.empty` never existed in the original design, so there was nothing to remove for
  it anywhere — pure addition to `SPC-1`.
- Verified the round-trip test (`cargo test -p typdoc-core --test rules`) still passes after the
  `config-errors.md` catalog text change — it checks ids, not the `reported_when` prose, so this
  was expected but worth confirming rather than assuming.
- Full workspace suite unaffected (no code changed): 1060 tests still passing. Public-text gate
  run scoped to every touched file: clean.
- No code, tests, or fixtures touched — this ticket is entirely `docs/`/`.chief/` content.
