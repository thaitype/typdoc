# 10: A `number` field past u64 on the write path

Type: wayfinder:grilling
Status: open
Blocked by: 8

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

## Answer

<filled in on resolve>
