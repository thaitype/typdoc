# Ticket 5: writing frontmatter

Resolved. Commit `0af2cb5`. Four gates green on the committed tree, each re-run after the commit:
`cargo fmt --check` clean, `cargo clippy --workspace --all-targets -- -D warnings` clean,
`scripts/test.sh` 820 passed / 0 failed / 1 ignored (799 / 0 / 1 before), public-text check clean.

## Outcome

done

## What it does

`crates/typdoc-core/src/frontmatter.rs` holds `trait FrontmatterWriter` — `set_scalar`,
`append_item`, `remove_item`, `add_key`, and `finish`, which assembles the whole block — and
`YamlSerdeWriter`, its one implementation. It starts from the fields a read already found, is
changed through the trait, and serializes with `yaml_serde`. Field order is kept by a hand-written
`Serialize`, because a map would sort the keys. There is no re-read guard and no line editing: a
write rewrites the block.

`finish` produces text and nothing here puts it on disk. That boundary is deliberate: producing the
block has no failure a file system creates, so it is tested without one. Splicing the block back
with the unchanged body and calling `write_atomically` under the lock belongs to the ticket that
builds `set`, `new` or `mv`.

`.chief/project.md`'s YAML paragraph, which still described `yaml-edit`, a trait of three
operations and a re-read guard before the rename, now says what is built.

## Checked by running

- The round trip walks every document under `fixtures/valid/` — 63 files — and for every field of
  every document changes that field through the trait and asserts every other field reads back
  identically. 51 had a field to check, 10 have no block and 2 no fields, and the test asserts the
  four counts add up to the number of documents found, so a document cannot drop out unnoticed.
- The table of losses is read from `docs/design.md` itself: the reader finds the table by both its
  column headings inside the section that holds it, and a design with no such table is an error
  rather than an empty list, which is its own test. Each row's shape is looked for in the corpus and
  an unrecognized row fails by name instead of being skipped; a second test shows no detector is a
  rubber stamp by giving it a block holding none of the other shapes.
- `fixtures/valid/frontmatter-losses/shapes.md` carries all seven shapes the table names. Moving it
  away turned the corpus test red naming the first unmet shape, and it was restored.
- The literals the design promises come back exactly — `1e3`, `1.10`, `0755`, a thirty-digit
  integer, `no`, `yes`, `~`, `null`, `true` as text, dates, the empty string, and text holding a
  newline, a tab or a leading space.
- Each operation was shown able to fail by changing what it does and watching its own test go red,
  one at a time, each restored: the first field skipped in serialization, `remove_item` made a
  no-op, `add_key`'s existence check removed, `append_item`'s scalar branch removed.
- The existing write ban reaches the new module: `std::fs::write` planted inside the
  block-producing method was refused as a disallowed method, and removed.

## Decision

- **Issue:** decision 20 says the trait of three operations in front of the writer stays, and also
  that a write rewrites the whole block. A single `write(fields) -> String` gives the second effect
  and leaves the first with none.
- **Options considered:** one method taking the finished field list; the trait's operations kept
  with the block assembled at the end.
- **Chosen:** the second. The operations change an in-memory block and `finish` writes all of it, so
  both sentences have an effect — nothing edits a line of the file, and the three operations are
  still the shape a call site sees.

## Notes

- **`[empty-value]` is untouched and the writer makes it visible.** `fields()` still reads
  `reviewer:` and `reviewer: ''` as the same empty text, so the writer puts both back as
  `reviewer: ''`. The design says a write puts back the form it found, so until ticket 6 closes this
  the writer does not keep that promise for a field written with no value. Nothing ships that calls
  the writer yet. Ticket 6 is free to add a case to `Value` or a flag beside the text: `set_scalar`,
  `add_key` and the block's serialization each gain an arm and nothing else moves.
- Two choices the design does not make, both written into the trait's own documentation rather than
  left silent, for whichever ticket builds `set` to confirm or refuse earlier: `append_item` on a
  field that exists and is not a list replaces it with a one-item list in place, and `remove_item`
  taking out the last item leaves an empty list rather than deleting the field.
- The round trip covers `fixtures/valid/` and not `fixtures/broken/`, whose documents are malformed
  on purpose and have nothing to round-trip. The test counts a document it cannot parse rather than
  ignoring it, so widening the reading later changes one path and not the accounting.
