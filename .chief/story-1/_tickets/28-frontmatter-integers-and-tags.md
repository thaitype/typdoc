# 28: Integers past 64 bits and tagged values fail `frontmatter.parse`

Type: implementation
Status: open
Blocked by: None (can start immediately)

## What this delivers

`ShapeVisitor` in `frontmatter.rs` has no `visit_i128`, `visit_u128` or `visit_enum`, so a field written as `123456789012345678901`, `-9223372036854775809` (also inside a list) or `!Ref x` / `!custom v` fails with "expected a scalar" although each is one. The document then leaves `list` with no warning, `get` exits 2 and `validate` reports it. Every other scalar already keeps the text as written (`1.10`, `01234`, `0x1F`, `1e3`, `yes`), which is what the design says a value does.

- The shape read accepts those scalars, and the text read (`Text`, `Texts`) returns the text as written for each. For a tagged scalar the text is the value without its tag; whether the reader hands `Text` that text has to be run, not assumed.
- A tag on a mapping or a list keeps today's refusal.

## Done when

- Tests fail before the change and pass after it: an integer of 21 digits and one below `i64::MIN` in a string field, in a `number` field and in a list; `!custom v` and `!Ref x` as a field value and as a list item. Each asserts the written text through `get --json` and that `list` still lists the document.
- The scalars that worked before (`u64::MAX`, `i64::MIN`, `!!str`, `!!binary`, `!!timestamp`) give the same output.
- A `number` field written past `u64` is stated in the report: what `coerce` does with it, since `number` is read as a JSON number.
