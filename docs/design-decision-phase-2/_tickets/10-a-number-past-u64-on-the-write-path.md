# 10: A `number` field past u64 on the write path

Type: wayfinder:grilling
Status: open
Blocked by: None (can start immediately)

## Question

Story 1 left a known defect with a doubt recorded against it. Frontmatter is read into typed `String` fields, so an integer past 64 bits keeps its digits in the file and on the read path. In `--json`, a `number` field past u64 becomes a float and loses its digits: `12345678901234567890123` comes back as `1.2345678901234568e22`.

On the read path that is a wrong answer. On the write path it is a rewritten file:

- `set` on a neighbouring field must leave the long number's line byte-identical. Ticket 8 probes whether the editor does that.
- But if any part of the write path goes through the parsed value rather than the text — filling `auto` fields, validating, building the result that `--json` prints, or the reparse guard's comparison — then the lossy value is what gets compared or written, and the user's number is quietly replaced with a rounded one. A guard that compares a rounded value against a rounded value agrees with itself and passes.

Decide:

- Whether the reparse guard compares text or parsed values, and what that means for a document holding a value the reader cannot represent exactly. This is the part that decides whether the defect is a wrong answer or a data loss.
- What `set` does when asked to write a `number` whose digits exceed what the program can represent exactly: refuse it, write the text through unchanged, or accept the loss and say so.
- What `new` and `set` print in `--json` for such a field, given that the read side has the same problem and ticket 15's rule is that one shape serves everywhere.
- Whether the read-side defect is fixed in this story or left as it is with the write path guarded around it. Fixing it is not story 2's job by scope, but story 2 is the story that turns it from a bad answer into a damaged file, which is a reason to decide it here rather than to inherit it.

## What the prototype found

Run 2026-09-21 against the binary built from this branch. Nothing is decided here; the ticket stays
open.

### How to run it again

Copy `fixtures/valid/field-types` somewhere writable, add a document to `records/` whose frontmatter
holds the value under test, and run `typdoc get <path> --json` with `TYPDOC_DIR` pointing at the copy.
The `record` schema already types `count` and `ratio` as `number` and `version` as `string`, which is
the comparison that matters.

### The range where a number survives, exactly

| Written in the file | `get --json` prints |
| --- | --- |
| `9007199254740993` (2^53 + 1) | `9007199254740993` |
| `18446744073709551615` (u64 max) | `18446744073709551615` |
| `18446744073709551616` (u64 max + 1) | `1.8446744073709552e+19` |
| `99999999999999999999` | `1e+20` |
| `-9223372036854775809` (i64 min − 1) | `-9.223372036854776e+18` |

So the boundary is the integer range itself, from i64's minimum to u64's maximum. Inside it every
digit survives. Outside it the value becomes a float and the digits are gone.

This corrects a guess left on story 1's review list, which suspected the boundary was 2^53. It is
not: 2^53 + 1 is exact.

The third row is the one to look at twice. `99999999999999999999` prints as `1e+20`, a different
number, not a rounded one with a long tail. Anything reading that output gets a value that is wrong
in the first digit of the fractional part and looks perfectly tidy.

### What the file and the other paths do

The file on disk is untouched by a read, and a field typed `string` holding the same digits comes
back complete: `version: "12345678901234567890123"` prints as the string
`"12345678901234567890123"`. The loss is in the conversion for a field typed `number`, not in the
reader's handling of the text.

`validate` on the same document reports nothing: no error, no warning, exit 0. A project can hold a
number typdoc cannot represent and be told it is entirely well.

### The write path, from the editor probe

From [decision 8](8-yaml-edit-on-anchors-tags-and-text-values.md), run the same day:

- Setting a neighbouring field leaves a line holding `big: 12345678901234567890123` byte-identical,
  so an edit beside such a value does not disturb it.
- Writing the digits as a value works: `12345678901234567890123` was written as
  `'12345678901234567890123'` and read back as the same string.
- Reading the same document into an untyped value fails outright —
  `invalid type: integer 12345678901234567890123 as u128, expected any YAML value` — rather than
  rounding. So a write path that passes a document through an untyped value does not lose digits
  quietly; it stops. It is the typed `number` conversion that loses them quietly.

## Answer

<filled in on resolve>
