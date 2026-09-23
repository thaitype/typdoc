# 10: A `number` field past u64 on the write path

Type: wayfinder:grilling
Status: resolved
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

**Decided: `set` writes such a number unchanged and never refuses it; `--json` prints a `number`
with the digits written in the document; and the read side is fixed in this story, because the
write commands cannot be right while it is wrong.**

### What is measured, against the binary built from this branch

The loss is not where the ticket feared it would be. With
[decision 20](20-the-frontmatter-writer.md), the write path is handed the text the reader kept and
writes that text; it never turns a value into a number on the way to disk. So the file is safe,
and what is left is the last step, printing.

That step is worse than "digits are lost":

| In the document | `get --json` prints |
| --- | --- |
| `count: 99999999999999999999` | `1e+20` |
| `count: 99999999999999999998` | `1e+20` |
| `ratio: 1e3` | `1000.0` |
| `version: "99999999999999999999"` (typed `string`) | `"99999999999999999999"` |

Two documents whose numbers differ by one print the same value and cannot be told apart. And the
comparison built on the same conversion agrees with itself:

```
typdoc list --where 'count>99999999999999999998'   ->   0 documents
```

`validate` reports nothing on either document: no error, no warning, exit 0. A project can hold a
number typdoc cannot represent and be told it is entirely well.

### `set` writes it, and does not refuse

The three choices the ticket offered were refuse, write the text through unchanged, or accept the
loss and say so. The third does not arise: there is no loss to accept, because nothing on the
write path parses the value.

Refusing would be forbidding a user to write a value that the file format holds exactly, that the
reader keeps exactly, and that a field typed `string` already carries today without complaint. The
tool would be declining to do a thing it can do, because of a defect in how it reports afterwards.

### `--json` prints the digits written in the document

JSON puts no limit on the digits of a number. The readers do, each in its own way, and that is the
fact the decision turns on. Measured, with one object printed as the document has it:

```
{"count": 99999999999999999999, "ratio": 1e3, "exact": 12345678901234567890123}

python  ->  99999999999999999999      12345678901234567890123     both whole
jq      ->  99999999999999999999      12345678901234567890123     both whole
node    ->  100000000000000000000     1.2345678901234568e+22      rounded
```

A reader that cannot hold the value still rounds it. The difference is who decides and on what.
Today typdoc rounds, silently, and hands over a value no reader can recover the original from.
With the digits printed, a reader that can hold them does, and a reader that cannot rounds a true
value — which is its own limit, declared in its own language, rather than a loss buried in
somebody else's output.

The rule this follows is the one the whole read path is built on: nothing between the file and the
caller decides what `1e3` is. Frontmatter is read into text for exactly that reason. The JSON
printer was the one step that still decided, and `1e3` becoming `1000.0` is that decision showing
in an ordinary document, not only in an extreme one.

**The same rule covers `new` and `set`**, and not by a separate ruling.
[Decision 17](17-the-json-shapes-of-new-set-and-mv.md) settled that both print the object `get`
prints. One shape means one rule, which is what phase 1's ticket 15 asked for.

### The read side is fixed in this story, and it is not scope creep

The ticket asked whether to fix it here or guard the write path around it. There is nothing to
guard: the write path never touches the conversion.

But the write commands print a document, and it is `get`'s document. If that object lies, `new`
and `set` lie in exactly the same way, on values they have just written correctly. So this is not
story 2 reaching into story 1's work for tidiness; it is the condition under which story 2's own
output can be right. Story 2 owns it.

### What it costs, stated

**Output changes for ordinary documents, not only extreme ones.** `ratio: 1e3` prints `1000.0`
today and will print `1e3`. Every golden holding a number is affected. typdoc has never been
released, so there is no compatibility to keep, and the design already says that distribution and
versioning of the tool are not settled; this is the moment such a change is free.

**A reader in a language with only doubles still loses the value.** Nothing can fix that from
here. What changes is that the rounding is theirs, from a number that was true when it reached
them.

**Comparison is not fixed by this, and is not smuggled in.** `count>99999999999999999998`
returning nothing comes from the same conversion, in a different place, and putting it right means
comparing numbers that no primitive holds. That is its own piece of work with its own risks, it
belongs to the query path rather than the write path, and it is
[decision 22](22-comparing-numbers-beyond-a-primitive.md) rather than a sentence here.

### Written into `docs/design.md`, and the gap the binary still has

The JSON output section gains a paragraph: a `number` is printed with the digits written in the
document, with the reason and with what converting first costs.

The binary does not do this yet, and the design is the source of truth, so the difference is
recorded where the suite can see it. `registry::KNOWN_GAPS` gains `[number-text]`, and
`crates/typdoc/tests/frontmatter_scalars.rs` gains
`a_number_outside_the_integer_range_is_printed_converted_not_as_written`, which pins today's
behaviour in both directions: that the two values one apart print alike and that `1e3` prints
`1000.0`, and that the printed text is not yet the text in the file. Closing the gap turns that
test red, which is how the entry cannot go stale.
