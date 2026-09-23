# Ticket 11 Report

## Ticket

The central helper for reading a typdoc document's JSON body, dispatched on `content_type`.

## Outcome

done

## Notes

`typdoc_core::read_json_body::<T>(&raw_file_text) -> Result<T, JsonBodyError>`
(`crates/typdoc-core/src/json_body.rs`, `pub` from the crate root). Takes the whole raw document
text, not a path or a `Document` — `Document` discards the body after computing typed fields, so
there's no existing type carrying both; taking raw text also makes "dispatch never depends on
path" true structurally (there's no path parameter to read even by accident), not just by
convention.

`JsonBodyError` has four distinct variants (unreadable frontmatter block, missing `content_type`,
unrecognized `content_type`, invalid JSON despite declaring it) — all four confirmed
pairwise-distinguishable, not just present. All four of ticket 10's real catalog documents
round-trip correctly (23 rules with the right `configurable` split, 9 commands in order, codes
0-7, 7 loss strings) — checked against actual file content, not synthetic fixtures alone.

Ticket 12's calling pattern is settled: read the catalog file's raw text
(`std::fs::read_to_string`), then `typdoc_core::read_json_body::<T>(&text)` — the same shape
`typdoc_testkit::fixtures::design_text()` used for what it's replacing.

Rebased cleanly onto the current tip (ticket 14's macOS follow-up landed in between — disjoint
files, no conflict); all three gates re-verified green after (1022 tests, 56 suites). Fast-forward
merged into `story-3-catalog-and-release`.

**Ticket 12 remains blocked** — not by this ticket any more, but by M-13 (ticket 24's HOW), still
open with Mild.
