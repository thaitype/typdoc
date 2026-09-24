# Ticket 21 Report

## Ticket

Text output for both forms of `mv` (labeled block, `rewritten:`/`unrewritten:`/`findings:`), and
`mv --json`'s new additive `rewritten` field.

## Outcome

done

## Decision

- **Issue:** rebasing onto the current story-branch tip hit a real merge conflict in
  `crates/typdoc/src/cli.rs`'s `use typdoc_core::{...}` import list — ticket 17 (merged in the
  meantime) added `Heading`; this ticket's own branch added `RefName` and `RewrittenRef` to the
  same block.
- **Options considered:** none needed a design call — both sides were independent, additive
  imports for unrelated new types (`Heading` for `toc`, `RefName`/`RewrittenRef` for `mv`), not a
  substantive disagreement. A throwaway decision-support agent would have added nothing here.
- **Chosen:** took the union of both additions, resorted alphabetically. Verified rather than
  assumed correct: `cargo clippy` clean (no unused-import warning would have caught a stray
  addition, no missing-import error confirmed nothing was dropped), full `scripts/test.sh` green
  after.

## Notes

The real work: `typdoc-core` (not the CLI) now tracks which refs a move actually rewrote, not
just which it couldn't — `RewrittenRef { document, field, before, after }` on `MvReport`,
populated where `rewrite_holder` already decides a ref changed. The CLI layer only formats this:
`mv --json`'s `rewritten` list, and text's `rewritten: N refs in M documents` count (M = distinct
holder paths, so one holder touched on two fields counts once). All four required test cases
(rewrites something / leaves something unrewritten / clean move / error path) covered for both
`mv` forms, in both text and `--json` — a code-review pass caught and the build fixed a missing
`--json` `rewritten: []` assertion for the clean-move case before committing.

**Flagged, already tracked, no action needed here:** the build noted `docs/commands.md` is stale
for `mv --renumber`'s old bare-key description (and for ticket 16's get/set/new changes too) and
asked whether a docs ticket exists. It does — ticket 22, blocked by this ticket among others,
exists precisely for this sweep (contract decision 5). Nothing to do differently.

Rebased with one manual conflict resolution (see Decision above); all three gates re-verified
green after. Fast-forward merged into `story-3-catalog-and-release`.
