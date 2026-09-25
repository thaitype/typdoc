# 6: Migrate story 4's design changes out of `docs/migrating-design/design.md`

Type: implementation
Status: resolved
Blocked by: 1, 2, 3, 4 (needs the final, shipped behavior of every other ticket to describe)

## What this delivers

Story 4 changed behavior that `docs/migrating-design/design.md` still described the old way
(the working copy meant to be progressively emptied into `docs/design/spec/`/
`docs/design/catalog/`, per `.chief/project.md`, never actually had anything removed from it
before this ticket). Every piece of design text story 4's behavior actually changed now lives in
`docs/design/spec/`/`docs/design/catalog/`, and the corresponding old text is gone from
`docs/migrating-design/design.md` — not the whole file, only what this story touched.

## Scope

- New `docs/design/spec/SPC-7.md` ("Namespaces and project discovery explained"): project =
  folder containing `.typdoc/` (not necessarily `config.json`), discovery, nested projects, and
  the full wildcard-exclusion behavior (order/last-match-wins, full invisibility, the
  silent-when-`!`-matches-nothing rule, `--namespace`/`TYPDOC_NAMESPACE` not accepting `!`, and
  state surviving exclusion without counting as orphan).
- `docs/design/spec/SPC-1.md`: `collections.empty` added to the always-on rule list (fourteen →
  fifteen), with its own firing condition explained. This rule never existed in the original
  design, so nothing to remove for it — pure addition.
- `docs/design/spec/SPC-2.md`: a new paragraph on `mv`'s own-path check, describing the two
  distinct messages (identical path vs. genuine case-only) instead of the old single, conflated
  one.
- `docs/design/catalog/config-errors.md`: `config.state-orphan`'s `reported_when` text updated
  to say a file matching an excluded namespace is not orphaned either.
- `docs/migrating-design/design.md`: the Project model row, the Discovery paragraph, the Nested
  projects paragraph, the Namespaces paragraph and its "must exist" bullet, the
  `config.state-orphan` sentence in the State paragraph, and the `mv` same-path sentence — each
  edited in place, either corrected to the current fact with a pointer to the new spec document,
  or (Namespaces, Model row, Discovery, Nested projects) replaced with a short pointer where the
  new spec document now fully supersedes the old text. Everything else in the file — collections,
  schemas, refs, query, concurrency, the rest of `mv`, and so on — is untouched: none of it is
  what story 4 changed.
- `.chief/project.md` line 7: design source of truth now points at `docs/design/` (spec +
  catalog), not `docs/archived-design/design.md`; content not yet migrated still lives at
  `docs/migrating-design/design.md`, which is what "progressively emptied" was always supposed to
  mean but had never actually been exercised before this ticket.

## Testing

- `cargo test -p typdoc-core --test rules`: the round-trip test between `rules.rs::ALWAYS_ON`/
  `RULES` and the catalog files still passes (ids unchanged; only `config-errors.md`'s prose
  `reported_when` text changed, which that test doesn't pin verbatim).
- Full workspace suite (`scripts/test.sh`): unaffected, since no code changed, only docs.
- Public-text gate run scoped to every touched file: clean.
