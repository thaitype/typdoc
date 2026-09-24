# 21: Whether a write warns when it is about to drop something

Type: wayfinder:grilling
Status: resolved
Blocked by: 20

## Question

[Decision 20](20-the-frontmatter-writer.md) settled that a write rewrites the whole frontmatter
block and that comments, blank lines, flow style, quote style, spacing, anchors, aliases and tags
do not survive it. The design states that, and advises that a document depending on an alias or a
tag be edited by hand rather than by typdoc.

The advice is in the design. The user running `typdoc set` is not reading the design.

Two of the losses are not cosmetic. An alias is one value used in several places; after a write it
is several values that no longer follow each other, and no rule in `validate` reports it, because
the file is valid and every value is the one it was. A tag may be what another tool in the user's
chain reads. In both cases a command that exits 0 with nothing said has destroyed work that was
done deliberately, and the user finds out later or never.

Decide:

- **Whether the command says anything.** A warning naming what was dropped, at which lines, is the
  obvious answer; against it is that a document with a comment in its frontmatter is ordinary, so a
  warning on every write teaches the user to ignore warnings, and the one about the alias drowns.
- **Which losses are worth saying, if not all.** Comments and spacing are cosmetic and common.
  Anchors, aliases and tags change what the file means and are rare. A split along that line is
  available; so is warning about nothing.
- **Before or after.** A warning after the write tells the user what has already happened. Before
  it, the command would have to ask, and no other command in v1 asks anything, or refuse, which
  would put the document back in the class decision 20 was chosen to abolish — one typdoc cannot
  write.
- **Whether a project can choose.** An option that turns the meaning-changing losses into a refusal
  would let a repository that uses anchors keep them safe, at the cost of a config surface and a
  second behaviour to test. The default has to be decided either way.
- **Whether `validate` reports it instead, or as well.** A rule that flags frontmatter holding an
  anchor, an alias or a tag would tell the user before any write ever happens, and it costs nothing
  at write time. It also flags documents nobody intends to write to.

The question `--audit` already answers for skipped files is the nearest precedent: something not
handled gets a place in an accounting rather than silence.

## Answer

**Decided: a write says nothing. The one loss that had to be prevented is prevented at its source
instead, because it was never a matter of warning — it was a value being changed.**

### The measurement this ticket was missing

[Decision 20](20-the-frontmatter-writer.md) recorded as not verified how much real frontmatter
carries comments, anchors or tags. It has now been counted, across every repository typdoc is
meant for:

| Where | Markdown files | With frontmatter | Comments | Anchors | Aliases | Tags | Blank lines | `[flow]` | Quoted |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| chief | 56 | 26 | 0 | 0 | 0 | 0 | 5 | 0 | 0 |
| typmem | 7 | 4 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| the memory store | 465 | 456 | 0 | 0 | 0 | 0 | 0 | 33 | 0 |
| four other projects | 57 | 0 | — | — | — | — | — | — | — |
| **total** | **585** | **486** | **0** | **0** | **0** | **0** | **5** | **33** | **0** |

**Not one of the four things this ticket was opened about occurs in 486 real documents.** No
event can be told for a warning about an anchor or a tag: not who it happened to, not what they
were doing.

### And the count found something nobody was watching

Run against the binary, on a document taken from the memory store:

```
before                          after a write
author: Tiana                   author: Tiana
reviewer:                       reviewer: ''
created: 2026-08-17             created: 2026-08-17
```

**279 of 482 documents have a field written with a name and no value.** In YAML `reviewer:` is
null and `reviewer: ''` is a string of no characters; typdoc reads both as the same empty text,
so a write puts back whichever one the writer emits. A tool reading the same file afterwards —
a site generator, a script of the user's — tells them apart even where typdoc does not.

That is the same kind of harm a dropped tag would do, at 279 documents instead of none.

### Why that is not answered by a warning

A warning is an apology in advance for something the tool chose to do. Here the tool does not
have to do it. The design says a write changes no value, and this is a value, not a style — so
the honest fix is at the source: **typdoc keeps the two apart, and a write puts back the form it
found.** Nothing to warn about, and 279 documents are not touched at all.

This is stated as fixing a promise that is already broken rather than as a new promise, which is
what it is.

**In `--json`, a field written with no value is `null` and one written as an empty string is
`""`**, for the reason [decision 10](10-a-number-past-u64-on-the-write-path.md) gives for a
number's digits: the caller is told what the file says, not what typdoc would have made of it.

**What does not change, and is guarded so it cannot drift.** Inside typdoc the two go on meaning
the same thing — for every rule, for `--set`, for a template and for a query. A required field
written with no value is present, not missing, and a `number` written with no value fails its
type. Both were measured on the binary and both are asserted in the test below, because the
obvious way to implement this decision is to make a bare field absent, and that would turn 279
documents that pass today into findings. Nobody asked for that.

### And for the rest of the losses, nothing is said

Comments, blank lines, flow style, quote style, spacing, and anchors and tags where they occur:
the command says nothing, and the design's table is where a user learns what a write does not
keep.

The argument against warning is the count, and the argument for stopping there is what a warning
on 58 per cent of writes would do to the one that mattered. Warning about everything teaches a
user to read nothing, and what would then be drowned is the rare case this ticket was opened
for. Having removed the common loss rather than announced it, there is nothing left frequent
enough to drown anything.

The design already advises that a document depending on an anchor or a tag be edited by hand.
That advice stays where it is, in the document, and is not moved into the command's output on
the strength of a case nobody has met.

### Written into `docs/design.md`, and the gap the binary still has

The Documents rules gain the distinction between a field written with no value and one written as
an empty string, that a write puts back the form it found, and — in the same breath, because this
is where it would go wrong — that nothing inside typdoc changes meaning. The JSON output section
gains the `null` and `""` sentence.

The binary reads both as the same empty text, so the difference is recorded where the suite sees
it: `registry::KNOWN_GAPS` gains `[empty-value]`, pinned by
`a_field_written_with_no_value_reads_the_same_as_an_empty_string`, which asserts today's reading
in both directions and also asserts the two validation behaviours that must survive the fix.
