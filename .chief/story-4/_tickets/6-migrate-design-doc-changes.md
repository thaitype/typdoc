# 6: Migrate story 4's design changes out of `docs/migrating-design/design.md`

Type: implementation
Status: resolved
Blocked by: 1, 2, 3, 4 (needs the final, shipped behavior of every other ticket to describe)

## What this delivers

Story 4 changed behavior that `docs/migrating-design/design.md` still described the old way
(the working copy meant to be progressively emptied into `docs/design/spec/`/
`docs/design/catalog/`, per `.chief/project.md`, never actually had anything removed from it
before this ticket — and now has a standing rule, `.chief/_rules/_standard/design-docs.md`,
saying so explicitly). Every piece of design text story 4's behavior actually changed now lives
in `docs/design/spec/`/`docs/design/catalog/`, and the corresponding old text is a pure deletion
from `docs/migrating-design/design.md` — never edited in place, per the standard.

## Scope

- New `docs/design/spec/SPC-7.md` ("Namespaces and project discovery explained"), created with
  `typdoc new SPC` (allocating the key and bumping `.typdoc/state/default.json`'s `spec.last`
  from 6 to 7 correctly — a manually-written file would have desynced it, caught by running
  `typdoc validate` after). Covers project = folder containing `.typdoc/` (not necessarily
  `config.json`), discovery, nested projects, the full Namespaces mechanics (both what was
  already true and the new wildcard-exclusion behavior: order/last-match-wins, full invisibility,
  the silent-when-`!`-matches-nothing rule, `--namespace`/`TYPDOC_NAMESPACE` not accepting `!`,
  and state surviving exclusion without counting as orphan). The unchanged Namespaces mechanics
  (symlink handling, ASCII-only names, the nesting rule, and so on) came along too: the standard
  says to move a whole section an existing or new SPC comprehensively covers, not just the
  sentence that changed, so `docs/migrating-design/design.md`'s Namespaces paragraph and its
  bullets could be deleted as one clean unit rather than left half-populated beside a newer,
  fuller SPC-7.
- `docs/design/spec/SPC-1.md`: `collections.empty` added to the always-on rule list (fourteen →
  fifteen), with its own firing condition explained. This rule never existed in the original
  design, so nothing to remove for it — pure addition, no `docs/migrating-design/` change needed.
- `docs/design/spec/SPC-2.md`: a new paragraph on `mv`'s own-path check, describing the two
  distinct messages (identical path vs. genuine case-only) instead of the old single, conflated
  one — pure addition to SPC-2 itself.
- `docs/design/catalog/config-errors.md`: `config.state-orphan`'s `reported_when` text updated
  to say a file matching an excluded namespace is not orphaned either.
- `docs/migrating-design/design.md`: the Project model row, the Discovery paragraph, the Nested
  projects paragraph, and the Namespaces paragraph with its bullets are gone — each now fully
  superseded by SPC-7, deleted as whole units with no replacement text left behind (`git diff`
  against this file is deletions only, zero insertions, matching the standard's own "Done when").
  Two smaller facts — the `config.state-orphan`/excluded-namespace nuance and the `mv` same-path
  message split — are **not** touched in `docs/migrating-design/design.md`, on purpose: each is
  one sentence embedded inside a much larger paragraph (State's numbering mechanics; `mv`'s
  locking and ref-rewriting) that story 4 didn't otherwise touch. Editing just that sentence
  cannot be expressed as a pure line deletion (a line-based diff has no way to remove part of a
  line without registering the whole line as changed), and migrating either entire surrounding
  paragraph would mean writing a full "state and numbering" or "mv" spec document — genuinely
  disproportionate to what this story changed, and explicitly against "move only the sections the
  change touches." Both facts are already fully and correctly described in `SPC-7` and `SPC-2`
  respectively; the stale wording sitting beside them in the still-unmigrated `docs/migrating-design/`
  is the known, accepted cost of not over-migrating.
- `.chief/project.md` line 7: design source of truth now points at `docs/design/` (spec +
  catalog), not `docs/archived-design/design.md`; content not yet migrated still lives at
  `docs/migrating-design/design.md`.

## Testing

- `typdoc validate` on the repo: exits 0, no findings — confirms the `SPC-7` numbering desync
  was actually caught and fixed, not just assumed fixed.
- `cargo test -p typdoc-core --test rules`: the round-trip test between `rules.rs::ALWAYS_ON`/
  `RULES` and the catalog files still passes (ids unchanged; only `config-errors.md`'s prose
  `reported_when` text changed, which that test doesn't pin verbatim).
- `git diff <merge-base> -- docs/migrating-design/`: zero insertions, deletions only.
- Full workspace suite (`scripts/test.sh`): unaffected, since no product code changed.
- Public-text gate run scoped to every touched file: clean.
