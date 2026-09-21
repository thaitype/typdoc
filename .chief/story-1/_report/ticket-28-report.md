# Ticket 28 Report

## Ticket
A frontmatter value written as an integer past 64 bits (`123456789012345678901`,
`-9223372036854775809`, also inside a list) or with a tag of its own (`!custom v`, `!Ref x`, also as
a list item) failed with `invalid type: integer ... as u128, expected a scalar, a list or a
mapping` or `invalid type: enum, expected ...`, although each is one scalar. The document then
left `list` with no warning, `get` exited 2 and `validate` reported `frontmatter.parse`.

## Outcome
done. Each of these values is a scalar and keeps the text written in the file: for an integer its
digits, for a tagged scalar the value without its tag. The document is listed, `get --json` shows
the text, and `validate` reports no `frontmatter.parse`.

- The cause was in the first pass of `frontmatter::fields`, the reader that sorts a value into
  scalar, list of scalars or something else. Its visitor had no `visit_i128`, `visit_u128` or
  `visit_enum`, and `yaml_serde` 0.10.7 gives an integer past 64 bits to the first two and a value
  with a `!` tag to the third. All three are added; the two integer ones answer scalar, and
  `visit_enum` drops the tag and reads the value under it as the shape it has.
- The second pass (`Text`, `Texts`, through `deserialize_str`) needed no change: run on the tree
  with the first pass fixed, it already returns the digits and the value without its tag.
  `deserialize_str` does not look at a tag or at the width of an integer.
- A tag on a mapping keeps its refusal (`field `s` is a mapping, which has no text form`), and so
  does a tagged list that holds a mapping or a list. A custom tag on a list of scalars
  (`l: !custom [a, b]`) is a list of scalars, the same as `!!seq [a, b]` was.
- Before the change, on the unmodified tree (the new CLI file run against a copy of it), 10 of
  the 12 CLI tests fail and 2 pass: the control for the scalars that worked before, and the
  `u64::MAX` case in a `number` field. The two refusals of a tagged mapping and of a tagged list
  that holds a mapping also fail there, because they now assert the message `has no text form`
  where the unmodified tree gave `invalid type: enum, expected ...`; both exit 2 in both trees.
  The tests of `frontmatter.rs` fail the same way with the change removed, and the `coerce` test
  is a pure function that passes before and after.

## Decision if any
- **A `number` field written past `u64` or below `i64::MIN`** is read as a JSON float, not kept as
  text. Run: `num: 123456789012345678901` gives `1.2345678901234567e20` in `get --json`,
  `-9223372036854775809` gives `-9.223372036854776e18`, `18446744073709551616` gives
  `1.8446744073709552e19`, `list` still lists the document and `validate` has no finding. Up to
  `u64::MAX` it is an exact integer (`18446744073709551615` prints as written). This is what
  `coerce` already did with such text (its test passes on the unmodified tree); before the ticket
  the value never got that far, and the ticket adds only the way to reach it: `coerce` parses
  the text as a JSON number, and a JSON number too wide for an integer is a float. Decided:
  pinned as it is (a test in `crates/typdoc-core/tests/coerce.rs`, one in the CLI file, one in
  `frontmatter.rs`), not changed to keep the text. Doubt: the digits
  past what a float holds are lost in the shown value (the run above prints `...20` where the file
  says `...901`), which is not what "a value that does not fit its type keeps its text" would
  give for a value that cannot be held exactly. Keeping the text would make it a
  `frontmatter.types` finding, which changes what `number` means and belongs to a separate change.
  Default: the float stays. The tests compare the float with a tolerance of 1e-15
  because the JSON reader in use is not correctly rounded to the last digit (it reads
  `123456789012345678901` one step away from the nearest float), so the exact last digit of the
  float is not part of what is pinned.
- **A custom tag on a list of scalars is a list of scalars.** The design is silent. It follows
  `!!seq [a, b]`, which was a list of scalars before, and it is the smallest reading that works:
  the tag names something typdoc does not read, so it is dropped as it is for a scalar.
- **The text of a tagged scalar is the value as the reader gives it after the tag**, including a
  quoted or block scalar under a tag (`!custom "a b"` is `a b`, `!custom |` keeps its text and its
  final line ending), which is the same text the untagged form gives.

## Notes
- Tests added: 12 through the built binary in `crates/typdoc/tests/frontmatter_scalars.rs`, 4
  in `frontmatter.rs` (a core-level seam through `frontmatter::fields` with a hand-built schema),
  1 in `crates/typdoc-core/tests/coerce.rs`. Each expected value is written by hand. The CLI cases
  cover a 21-digit integer, `18446744073709551616` and `-9223372036854775809` in a string field, a
  `number` field and a list; `!custom v`, `!Ref x` and `!Ref Thing` as a field value and as a
  list item; and a control that gives the same output for `u64::MAX`, `i64::MIN`, `!!str`,
  `!!int`, `!!binary`, `!!float`, `!!timestamp`, `!<tag:yaml.org,2002:str>`, `!!seq` and the
  scalars that keep their text (`1.10`, `01234`, `0x1F`, `1e3`, `yes`, `~`, Thai). Each case
  asserts the text through `get --json`, that `list --ids` still lists the document, and that
  `validate --json` has no `frontmatter.parse`.
- Consumers read: `Shape` and `Text`/`Texts` are used only by `frontmatter::fields`.
  `frontmatter::fields` is called by `Project::get` and `Project::refs` (an error becomes exit 2),
  by `parsed_fields` (`Project::list`, `evaluate_reached` for `ref.*` conditions, `check_refs`,
  `prescan_refs`), by `parsed_fields_and_body` (the reverse index for `refs --reverse` and the
  `ref.*` reach), and by `validate::check_document` (an error becomes `frontmatter.parse`). None of
  them changes: they took the failure as "no fields" or as the finding, and now they get the
  fields. So a document with such a value now appears in `list`, in the reverse index and in
  `refs`, where it had been dropped; a ref field written `up: !Ref b.md` resolves to `b.md` (run),
  and `up: 123456789012345678901` is a `refs.resolve` finding for a target that does not exist,
  like any other text.
- Seen and not changed: `s: !!null` with nothing after it (a `!!null` tag on an empty value in a
  string field) is refused with `invalid value: string "", expected null`, with the same message
  on the unmodified tree and on this one (run on both). The error comes from `yaml_serde`, for a
  `!!` tag it resolves to a core type, before any visitor method of this file runs. `!!null ~`
  keeps its text `~` in both.
- Checks run before the commit: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D
  warnings` and `scripts/test.sh`, all clean. A call to `std::fs::write` planted inside the new
  `visit_enum` is refused by clippy as `use of a disallowed method std::fs::write`, and was
  removed.
- Mutations, each restored and compared with the saved copy: `visit_i128` removed (fails 3 CLI
  tests and 1 unit test), `visit_u128` removed (3 and 2), `visit_enum` removed (6 and 2),
  `visit_enum` answering scalar for every tag, so that a tagged mapping or list is not looked at
  (3 and 1), and `coerce` refusing a `number` longer than 19 characters (2 CLI tests and the
  `coerce` test).

## Checked again after the build, by running
- `scripts/test.sh`: 746 passed, 0 failed, 1 ignored; `cargo fmt --check` and clippy with `-D warnings` clean.
- Run on the final binary with fields `s` (string), `l` (list): `123456789012345678901`, `-9223372036854775809`, a 21-digit item in a list, `!custom v`, `!Ref x` and a tagged list item give their text through `get --json`, and the documents are listed by `list --ids`.

