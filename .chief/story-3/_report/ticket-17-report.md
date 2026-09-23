# Ticket 17 Report

## Ticket

Text output for `toc`: a header-rowed table, and the error-path fix from hard-coded `true` to
the real `json` flag.

## Outcome

done

## Decision

- **Issue:** the empty-result precedent (no header, nothing printed, exit code carries the
  result) was decided in advance, following `list`'s own precedent — the ticket told the builder
  not to re-decide it, only to flag a genuine objection to Aria rather than deciding something
  else.
- **The build's own reviewer raised a real concern, not acted on:** for `list`, silence
  naturally reads as "no matches" across many documents. For `toc --depth`, the same silent-empty
  shape collapses two different situations into one: "this document has no headings at all" vs.
  "this document has headings, but none survive `--depth`" (e.g. `--depth 1` on a document whose
  headings start at level 2). A user could read the second case as the command having gone to
  the wrong place rather than "nothing at this depth."
- **Chosen:** left as decided — no change. Flagging this to Aria/Mild rather than the builder
  deciding a different behavior on its own, per the ticket's own instruction. Worth a real answer
  before `docs/design/spec/`/the user docs (ticket 22) describe this case, since right now
  neither shape distinguishes them.
- **Routed 2026-09-23 (Aria):** taken to Mild as **M-14**. Ticket 22 does not describe the
  `toc --depth`-filtered-to-empty case in the user docs until M-14 lands.

## Notes

Implementation reuses `list_table`'s row-rendering convention (two-space-separated, padded to
max width, header included, last column unpadded) rather than inventing a new table style. One
small duplication the reviewer found and fixed directly (in scope, safe): the `--depth` filter
predicate was copy-pasted between `toc_json` and the new text path — extracted into
`within_depth`, used by both. A second, larger duplication (`toc_table`/`push_toc_row` vs.
`list_table`'s own row algorithm) was left alone since a shared-helper refactor would cross into
what ticket 18 (`list`'s own header-row work) might also be touching in parallel — worth a
follow-up, not this ticket's scope.

All 21 `toc.rs` tests pass (18 pre-existing + 3 new). Rebased cleanly onto the current story
branch tip (tickets 9/14/15/16 in between — no conflicts, disjoint files); re-ran all three gates
after the rebase (56 test-suite blocks, 0 failed) before merging. Fast-forward merged into
`story-3-catalog-and-release`.
