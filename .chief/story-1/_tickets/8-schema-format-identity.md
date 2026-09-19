# 8: Is the schema format typdoc's own, or JSON Schema?

Type: wayfinder:grilling
Status: resolved
Blocked by: None (can start immediately)

## Question

The design's overview says frontmatter is "validated against JSON schemas" and its principles say types come "from JSON schemas", yet the Schema format section defines a custom format (`fields`, `type: ref[]`, `transitions`, `acyclic`, `auto`, `extends`) that is not JSON Schema. Confirm that the format is typdoc's own, decide whether any JSON Schema interoperability is wanted (importing, exporting, editor completion) or explicitly not, and correct the wording in `docs/design.md` so a reader is not misled.

## Answer

Decided with the human, 2026-09-19.

**The schema format is typdoc's own JSON format. It is not JSON Schema, and v1 has no interoperability with it** (no import, no export, no reading of JSON Schema files).

- **Why not adopt a JSON Schema subset with `x-` extensions:** the things the design gets most value from (`ref` and `ref[]` with `target`, `transitions`, `acyclic`, `auto`, `code`, `extends` across remote files) have no JSON Schema equivalent, so they would all be extensions. That buys partial interop at the price of carrying JSON Schema's whole vocabulary, and a format once adopted cannot be dropped.
- **Why not an export command:** the design fixes nine commands; a tenth for an unproven need is not worth it. It can be added later without breaking anything.
- **Wording fixed in `docs/design.md`** in three places: the Overview ("schema files (a JSON format of typdoc's own, not JSON Schema)"), the Schema-driven principle (states why JSON Schema cannot express the model), and the opening of the Schema format section.
- Consistent with ticket 12: the schema format is one of the formats covered by the single `version` in `config.json`.

**Possible later, not decided:** publish a meta-schema (a JSON Schema describing typdoc's own schema files) so editors can complete and check `schemas/*.json`. That needs no new command and does not make the format JSON Schema. Recorded under Not yet specified in the map. No one has said they want editor completion; this was not investigated.

## Not verified

Nothing was run; this is a wording and scope decision. The design was grepped for "JSON schema" before and after: two mentions, both rewritten.

