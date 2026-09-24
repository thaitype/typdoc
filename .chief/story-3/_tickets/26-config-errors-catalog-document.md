# 26: Restore config.* rule-id coverage via a fifth catalog document

Type: implementation
Status: claimed
Blocked by: None (ticket 13 resolved 2026-09-24)

**Opened 2026-09-24, Aria's call, per the one-doc-per-concept rule from Q6.** Ticket 12's own
report flagged (not fixed, correctly out of its own scope) that `docs/design/catalog/rules.md`,
as ticket 10 built it under contract decision 1's shape ("one entry per rule id from both of
today's two tables"), holds only 23 ids — the two Validation-rules tables `design.md` always had.
The pre-ticket-12 test also checked a *third* table, headed `Id | Reported when`, of `config.*`
config-error ids (20 of them, making `typdoc_core::rules::RULES`'s full 43: 14 `ALWAYS_ON` + 9
`CONFIGURABLE` + 20 `config.*`). That coverage silently dropped when `rules.rs` was rewired onto
the catalog document instead of the old `design.rs` extractor.

The goal states this story's coverage of design.md's old extracted sets "continues unchanged."
Losing the `config.*` set is a **Done-criterion miss**, not a flagged-but-acceptable narrowing —
fix it now, before ticket 25.

`config.*` ids are their own concept (config-file-shape errors, not document-validation rules),
so per the one-doc-per-concept rule from Q6 this gets its **own** catalog document, not a
reshaping of `rules.md`'s already-contract-fixed shape.

## The work

1. Find the config-error table. It moved in ticket 13: was `docs/design/design.md`'s
   "Validation rules" section, third table (`Id | Reported when`, ~20 `config.*` rows); now under
   `docs/archived-design/design.md` (frozen — read it, don't edit it) or equivalently
   `docs/migrating-design/design.md` (also fine to read from, same content).
2. Create `docs/design/catalog/config-errors.md`, same shape as `docs/design/catalog/rules.md`:
   `content_type: json` frontmatter, a JSON body with one entry per `config.*` id from that third
   table (id + the "Reported when" text, matching whatever field names `rules.md`'s entries use
   for its own analogous fields — mirror the established convention exactly, don't invent a new
   shape). Give it a real `explained_by` per the same `ref[]`-to-`SPC-*` convention the other four
   catalog documents use (reuse an existing SPC entry if one already covers config validation
   errors; only add a new SPC entry if none does).
3. `schemas/catalog.json` and `.typdoc/collections/catalog.json` already describe the general
   catalog shape (used by all four existing catalog docs) — confirm this new document validates
   against them as-is; only touch the schema if something about config-errors' shape genuinely
   doesn't fit (unlikely — don't add fields it doesn't need).
4. Update `crates/typdoc-core/tests/rules.rs`: read this new document (via ticket 11's
   `read_json_body` helper / ticket 12's `typdoc_testkit::fixtures::read_catalog` helper, same
   pattern as the existing read of `rules.md`) and check its id set against
   `typdoc_core::rules::RULES`'s `config.*` subset (the 20 ids that are neither `ALWAYS_ON` nor
   `CONFIGURABLE`) — same "both directions" coverage property the existing test already has for
   the other 23 (an id in the doc with no matching code, or vice versa, turns it red). Keep the
   existing 23-id check exactly as ticket 12 left it; this adds the missing 20, it doesn't replace
   what's there.
5. Update `rules.rs`'s and `validation_rule_ids()`'s doc comments (ticket 12 left explicit notes
   there about this exact gap) — remove the "not covered any more" language once this ticket
   closes the gap.

## Tests

- New/updated test in `rules.rs` covers all 43 ids (23 existing + 20 new), both directions.
- Drift proof: plant a made-up `config.*` id in the new document with no matching code -> test
  fails; revert -> green. Plant a made-up entry in `RULES`'s config-error subset with no matching
  catalog entry -> test fails; revert -> green. (Same proof style ticket 12 used for the other
  three catalog documents — do this for real, don't assume the property holds.)
- Full gate run: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `TMPDIR=/home/thw-home/.cache/typdoc-tmp scripts/test.sh`.

## Done

- `docs/design/catalog/config-errors.md` exists, same shape as the other four catalog documents,
  holds all 20 `config.*` ids with a real `explained_by`.
- `rules.rs` checks the full 43-id set (23 + 20) against `RULES`, both directions, restoring the
  goal's "coverage continues unchanged" criterion.
- All three gates green.
