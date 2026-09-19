# 8: Is the schema format typdoc's own, or JSON Schema?

Type: wayfinder:grilling
Status: open
Blocked by: None (can start immediately)

## Question

The design's overview says frontmatter is "validated against JSON schemas" and its principles say types come "from JSON schemas", yet the Schema format section defines a custom format (`fields`, `type: ref[]`, `transitions`, `acyclic`, `auto`, `extends`) that is not JSON Schema. Confirm that the format is typdoc's own, decide whether any JSON Schema interoperability is wanted (importing, exporting, editor completion) or explicitly not, and correct the wording in `docs/design.md` so a reader is not misled.

## Answer

