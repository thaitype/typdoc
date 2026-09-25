# 6: Migrate story 4's design changes out of `docs/migrating-design/design.md`

Type: implementation
Status: resolved
Blocked by: 1, 2, 3, 4 (needs the final, shipped behavior of every other ticket to describe)

## What this delivers

Story 4 changed behavior that `docs/migrating-design/design.md` still described the old way
(the working copy meant to be progressively emptied into `docs/design/spec/`/
`docs/design/catalog/`, per `.chief/project.md`, never actually had anything removed from it
before this ticket — and now has a standing rule, `.chief/_rules/_standard/design-docs.md`,
saying so explicitly, whose unit is the **section**, not the sentence: a fact that changed cannot
be left half-migrated, disagreeing with its own newer SPC, just because the rest of its
paragraph didn't change). Every design section story 4's behavior touches — even where part of
that section was already accurate — now lives entirely in `docs/design/spec/`/
`docs/design/catalog/`, and `docs/migrating-design/design.md` loses those sections as pure
deletions: no replacement text, no in-place edits.

## Scope

- New `docs/design/spec/SPC-7.md` ("Namespaces and project discovery explained") and
  `docs/design/spec/SPC-8.md` ("State and numbering explained"), both created with `typdoc new
  SPC` (allocating the key and correctly bumping `.typdoc/state/default.json`'s `spec.last` —
  6→7, then 7→8; a manually-written file desyncs this, caught the first time by running `typdoc
  validate` after). `SPC-7` covers project = folder containing `.typdoc/` (not necessarily
  `config.json`), discovery, nested projects, and the complete Namespaces mechanics — both what
  was already true (symlink handling, ASCII-only names, the nesting rule, and so on) and the new
  wildcard-exclusion behavior (order/last-match-wins, full invisibility, the
  silent-when-`!`-matches-nothing rule, `--namespace`/`TYPDOC_NAMESPACE` not accepting `!`).
  `SPC-8` covers the complete State/numbering topic: what `.typdoc/state/<namespace>.json` holds
  and how it's written, `state.missing`/`state.malformed`/`state.behind`/`state.retired`,
  `config.state-uncoded`, and `config.state-orphan` — including the new fact that an excluded
  namespace's state file is left alone and never counts as an orphan.
- `docs/design/spec/SPC-1.md`: `collections.empty` added to the always-on rule list (fourteen →
  fifteen). Never existed in the original design — pure addition, nothing to remove.
- `docs/design/spec/SPC-2.md`: expanded with the complete `## mv explained` section — the own-path
  message split (the actual story 4 change: two distinct messages instead of one conflated one),
  plus everything else `### typdoc mv` covered in the old design (locking and atomicity,
  `--renumber`, which collection a moved document lands in, what `mv` cannot rewrite) — the whole
  command section, comprehensively, so it supersedes the old one entirely rather than leaving it
  half-duplicated.
- `docs/design/catalog/config-errors.md`: `config.state-orphan`'s `reported_when` text updated to
  say a file matching an excluded namespace is not orphaned either.
- `docs/migrating-design/design.md`: gone entirely, as pure deletions — the Project model row;
  the Discovery paragraph; the Nested projects paragraph; the Namespaces paragraph and its
  bullets; the whole State section (the `.typdoc/state/<namespace>.json` paragraph through the
  `state.retired`/`config.state-uncoded` paragraph); the whole `### typdoc mv` section (locking
  through what it cannot rewrite); and the whole **Config errors** section (intro prose + the
  20-row id table), verified first, id by id and text by text, against
  `docs/design/catalog/config-errors.md` to confirm it's genuinely redundant before deleting it —
  it was (0 differences). Three cross-references broken by these deletions (`(see State)` ×2,
  `(see Discovery)` ×1, in paragraphs story 4 did not otherwise touch) were repointed to the new
  SPC files — the one deliberate exception to "deletions only," since leaving a dead internal
  link is worse than a 3-line, minimal, necessary fix.
- `.chief/project.md` line 7: design source of truth now points at `docs/design/` (spec +
  catalog), not `docs/archived-design/design.md`; content not yet migrated still lives at
  `docs/migrating-design/design.md`.

## Testing

- `typdoc validate` on the repo: exits 0, no findings, after both `SPC-7` and `SPC-8`.
- `cargo test -p typdoc-core --test rules`: the round-trip test between `rules.rs::ALWAYS_ON`/
  `RULES` and the catalog files still passes (ids unchanged; only `config-errors.md`'s prose
  `reported_when` text changed, which that test doesn't pin verbatim).
- `git diff <merge-base> -- docs/migrating-design/`: 3 insertions (the three repointed
  cross-references), 106 deletions.
- Full workspace suite (`scripts/test.sh`): unaffected, since no product code changed.
- Public-text gate run scoped to every touched file: clean.
