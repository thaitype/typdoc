# 10: `typdoc set`

Type: implementation
Status: claimed
Blocked by: 2, 3, 5

## What this delivers

- Fields written under the namespace's lock, with `--if` decided under that same lock, so a condition and the write it guards cannot be separated by another process; a false `--if` exits 3 with nothing written and the error object on standard error naming the condition that failed (decision 17).
- `auto: update` stamped when at least one value changes; writing an `auto` field directly is a validation error; `k=` removes a field.
- `frontmatter.transitions`, the one rule the read core does not check, with the first fixture whose declared command is a write.
- `--json` printing the document after the write, with no list of what changed.

## Done when

- The transitions fixture runs on a copy, asserts exit 2 and the finding, and leaves the document's bytes unchanged.
- A false `--if` leaves the file byte-identical.
- A `set` on a document outside every namespace is an ordinary write and its `--json` leaves `namespace` out, as the contract's default says.
- A value the reader keeps as text is written back unchanged, including a number no primitive holds.
