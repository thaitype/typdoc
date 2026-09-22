# 20: Which library writes frontmatter, and what a write does not keep

Type: wayfinder:grilling
Status: resolved
Blocked by: None (can start immediately)

## Question

Phase 1's decision 1 split reading from writing: read with `yaml_serde` into typed `String`
fields, write with `yaml-edit` pinned to an exact 0.3.x through a trait of three operations,
behind a mandatory re-read guard. The map lists that decision among the ones story 2 stands on
and does not reopen.

Two things have happened since, and both bear on the half of it that no code rests on yet.

**`yaml-edit` is not a dependency of anything.** Story 1 only read, so the crate was chosen and
never added. `typdoc-core` today depends on `chrono`, `pulldown-cmark`, `serde`, `serde_json`,
`sha2`, `thiserror`, `unicode-general-category` and `yaml_serde`. Changing the choice costs
nothing that exists.

**The prototype in [decision 8](8-yaml-edit-on-anchors-tags-and-text-values.md) weakened the
reason it was chosen.** It was picked as the only crate that preserved comments, quote style and
flow style. It does preserve them on the lines an edit does not touch. On the line it does touch
it drops the value's tag and its quote style, and the guard passes both because the value reads
back as the one that was asked for. Two further cases corrupt the block so badly that the guard
refuses the write and the user cannot complete a `set` at all: setting a key that holds an
anchor leaves an alias pointing at nothing, and setting a key whose value is a block scalar
fuses four lines into one and swallows the next field entirely. The crate was also two days old
when it was surveyed, has one author for 442 of its 469 commits, and had a pull request titled
"Fix a large number of bugs found with fuzzing" in the same week.

So: keep `yaml-edit`, or write with `yaml_serde`, the reader already in the tree, and accept
that a write rewrites the whole frontmatter block?

## What was measured

Run 2026-09-21, against `yaml_serde` 0.10.7, the version in the tree, in a throwaway crate
outside this repository.

**Reserialising an untyped value is not an option, and was not in question.** Reading a document
into `yaml_serde::Value` and writing it straight back changes the user's text: `ratio: 1e3`
becomes `ratio: 1000.0`, `ver: 1.10` becomes `ver: 1.1`, every comment is dropped, and a
document holding an integer longer than 64 bits does not read at all
(`invalid type: integer ... as u128`).

**Writing from the text the read path already keeps does not change any value.** typdoc reads
frontmatter into `(name, text)` pairs, so that no YAML reader decides what `1e3` is. Writing
that text back quotes whatever needs quoting and leaves the rest alone: `1e3`, `1.10`, `0755`
and a 23-digit integer all come back as themselves.

Sixty-five cases were run and all sixty-five round-tripped exactly:

- **44 values**, each written and read back with an untouched neighbouring field: the empty
  string; leading, trailing and whitespace-only text; `no`, `yes`, `on`, `true`, `null`, `~`;
  `123`, `0755`, `1.10`, `1e3`, `-5`, a 23-digit integer; a date and an offset datetime;
  `a: b`, `key:`, `# not a comment`, `value # here`, `*star`, `&amp`, `- item`, `? what`,
  `[a, b]`, `{x: 1}`, `---`, `...`; text with `\n`, with `\r\n`, with a lone `\r`, with a tab;
  Thai text, an emoji, a backslash path, both kinds of quote; 5000 characters; a NUL byte, a
  non-breaking space and a zero-width space.
- **13 key names**: with a space, a colon, a `#`, a leading dash, `123`, `true`, `null`, empty,
  Thai, a newline, dots, brackets.
- **8 list shapes**: empty, single, boolean-looking items, numeric items, whitespace items, an
  item with a newline, Thai and emoji items, items holding `:` and `- `.

Zero failed to write and zero came back different. An empty list is written `[]`; every other
list is written as a block list.

**What a write does not keep** was measured on the same document: comments anywhere in the
block, blank lines, flow lists, the quote style the user chose, aligned spacing, anchors and
aliases, and tags. Read into text, `defaults: &d shared-value` with `team: *d` comes back as two
independent copies of `shared-value`, and `tagged: !!str 123` with `custom: !Ref other-doc`
comes back as `'123'` and `other-doc` with the tags gone.

## Answer

**Decided: `yaml_serde` writes frontmatter, from the text the read path already keeps, and the
design says plainly what a write does not keep.** `yaml-edit` is not added.

The reasoning, in the order it decided the question:

**A write that cannot be made is worse than a write that reformats.** `yaml-edit` has documents
it cannot edit at all: a block scalar or an anchor in the block turns `set` into a refusal, and
the user's way forward is to open the file and do by hand what the tool exists to do. Writing
from text has no such case. Every document the read path accepts, the write path can write.

**The blast radius moves, it does not grow.** `yaml-edit` keeps every line it does not touch and
damages the line it does, sometimes silently. `yaml_serde` rewrites the block and damages no
value in it, ever, in a way that is identical on every run and can be described in a table in
advance. A user can be told the second; the first they find out about one document at a time.

**The existing promise already allows it.** The design said, before this decision, that existing
YAML style is kept "as a best effort, which is not a promise". This decision does not withdraw
that promise, because there was none; it replaces an effort nobody could predict with a
statement of exactly what survives. The promises that remain — a key never moves, a new key goes
at the end, an unknown field survives, the body is never reformatted — all hold, and the body is
untouched because frontmatter is split from it before either is read.

**The dependency is the smallest part of it, and still counts.** One crate instead of two, and
the one that remains is the one the reader already trusts, so the writer and the reader cannot
disagree with each other about what a document says.

**What is lost is written down rather than left to be discovered.** The design now carries the
table: comments, blank lines, flow style, quote style and spacing go, and so do anchors, aliases
and tags. The last two are called out as changing what the file means and not only how it looks,
with the advice that such a document should be edited by hand. That advice is the honest one:
typdoc cannot keep an alias through a write, and a tool that quietly turned one value into
several would be worse than one that says so.

**Kept from phase 1:** the trait of three operations in front of the writer, so that a
line-based editor or `yaml-edit` can take the place behind it later without the call sites
changing.

**Dropped from phase 1: the mandatory re-read guard.** It was carried forward here at first, on
the reasoning that a guard across one crate still catches a mistake in typdoc's own code. That
reasoning does not survive being asked what event the guard prevents.

With `yaml-edit` the event was immediate and had been reproduced: a block scalar fused four lines
into one and swallowed the following field; an anchor's key, once set, left an alias pointing at
nothing. With `yaml_serde` writing the text the reader already holds, there is no such event to
tell. Sixty-five cases were run, covering every value shape YAML reinterprets, every odd key name
and every list shape, and none of them came back changed.

If one were found, it would be a round-trip defect in `yaml_serde` — the crate every read in the
program already trusts without a second check. A guard would be that crate checking itself, and a
defect in it is not one typdoc can fix; the read path would be wrong long before the write path
was. Guarding the write and not the read would also be inconsistent: a misread document is a
wrong answer to every command, and nothing re-reads to catch that.

What replaces it is a test rather than code that ships: that typdoc assembles the block it meant
to — the right fields, the right text, the right order — is checked at the boundary of typdoc's
own code, which is where a test belongs. The atomic write stays, and so does the refusal to
replace an existing file; both prevent events that can be told, an interrupted run and a
destination that already holds someone's work.

**Not verified:** how much real frontmatter in the repositories typdoc is meant for actually
carries comments, anchors or tags. Nothing here depends on the answer — the loss is stated
either way — but it decides how loud the tool should be about it, which is open below.

## What this opens and closes

- **Closes [decision 8](8-yaml-edit-on-anchors-tags-and-text-values.md).** It was a prototype to
  find out which `yaml-edit` cases the guard turns into a refused write. There is no
  `yaml-edit`, so there are no such cases. Its findings are kept: they are the evidence this
  decision was made from.
- **Closes [decision 9](9-the-error-of-a-write-the-guard-rejects.md) entirely.** It asked what a
  user sees when the guard fires. There is no guard, so nothing fires, and the exit code and
  error id it was opened for are not needed.
- **Answers one of [decision 10](10-a-number-past-u64-on-the-write-path.md)'s four questions.**
  It asked whether the guard compares text or parsed values, and said that was the part deciding
  whether the u64 defect is a wrong answer or a damaged file. The answer is that the write path
  never parses a value at all: it is handed the text the reader kept and writes that text, so a
  value the reader cannot represent exactly is never turned into a number on the way to disk and
  cannot be written in its rounded form. The other three — what `set` does when asked to write such a number,
  what `--json` prints for it, and whether the read-side defect is fixed in this story — are
  untouched and still open. The prototype's separate finding that an untyped read fails outright
  on such a document is now moot: no path reads untyped.
- **Opens: whether a write warns when it is about to drop something.** A `set` on a document
  holding a comment, an anchor or a tag destroys work the user did on purpose. The design's
  advice is to edit such a document by hand, and a user who does not know the advice gets no
  chance to take it. Whether the command says so, whether it says so before or after, and
  whether it can be refused rather than reported, is not decided here.
