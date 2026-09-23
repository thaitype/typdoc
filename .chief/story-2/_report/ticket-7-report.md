# Ticket 7: a number printed with the digits the document holds

Resolved. Commit `4a2b72c`. Four gates green on the committed tree: `cargo fmt --check` clean,
`cargo clippy --workspace --all-targets -- -D warnings` clean, `scripts/test.sh` 767 passed / 0
failed / 1 ignored (765 / 0 / 1 before), public-text check clean.

## What it does

A field typed `number` is printed in `--json` with the digits written in the document.
`typdoc-core`'s `Number` keeps that text beside the value it converts to; `--json` prints the text
and everything else — equality, ordering, globs, `--sort`, the `list` table, a finding's message,
the config version check — reads the converted value, exactly as before. The document object is
assembled as JSON text, because a `serde_json::Number` holds only a `u64`, an `i64` or an `f64`.

`[number-text]` is out of `KNOWN_GAPS`; `[empty-value]`, `[reverse-scope]` and `[import-anchor]`
remain.

## Checked by running, after the commit

Through the built binary, a `number` field holding each of these came back as the file has it:
`1e3`, `1.10`, `-0`, `99999999999999999999`, `99999999999999999998`, `18446744073709551616`,
`12345678901234567890123`, `3`. The two twenty-digit values printed the same before this change and
print differently now.

Comparison is unmoved, which is the part this ticket was most at risk of changing:

- `--where count=1000.0` still matches a document written `count: 1e3`, and so does
  `--where count=1e3`.
- `--where 'count>99999999999999999998'` still returns nothing.
- `--where count=99999999999999999998` still matches both twenty-digit documents, so the two still
  compare equal.

That is the behaviour decision 22 measured and left alone, and it is unchanged.

## The goldens do not see this, and the documents predicted that they would

No golden file moved. Two reasons, and the second matters more than the first.

Only one golden holds a frontmatter `number` (`fixtures/output/get/field-types/`). More to the
point, a golden is compared **as parsed JSON**, and so are the assertions: both take the run's
standard output already parsed. Reading JSON turns a number into a primitive, so `1e3` and
`1000.0` are one value to the harness and it cannot tell them apart.

Shown rather than argued: with the printer changed back to the converted value, the golden test
passes (4 passed, 0 failed) while `frontmatter_scalars` fails three tests. The digits are pinned
against the bytes on standard output, which is where they can be seen, and the golden harness now
says so in its own module documentation.

The story's goal and its testing decisions both say that closing this gap rewrites every golden
holding a number, and name the size of that diff as an expected cost. That is not what happened,
and the statement is now known to be untrue for the reason above. It is left as it stands here
rather than edited.

Worth carrying forward: the goldens are not a guard for how `--json` prints a number, and the
goldens ticket 15 adds for `new`, `set` and `mv` will not be one either.

## Choices the design does not state

- Only `--json` prints the digits. The design's paragraph is scoped to `--json`, so the `list` text
  table and a finding's message keep the converted value; a document can therefore read `1e3` in
  `--json` and `1000.0` in the table. The wider reading would have moved the code that compares.
- The hand-written `assertions.json` says `1e3` although the check cannot tell it from `1000.0`,
  so the expectation reads as the document has it rather than as the comparison can see it.
- `Number` accepts exactly the texts the previous parse accepted; `1e400`, `0755`, `+3` and `.5`
  are still not numbers.
