# 8: `yaml-edit` on anchors, tags, and the values story 1 keeps as text

Type: wayfinder:prototype
Status: resolved
Blocked by: None (can start immediately)

## Question

The write half of the frontmatter round trip rests on `yaml-edit`, pinned to an exact 0.3.x and restricted to three operations: set a scalar, append or remove a list item, add a key. It was chosen because it was the only crate in the survey that preserved comments, quote style and flow style, with a byte-identical no-op round trip. Two things were never probed, and were written down as work for a prototype:

- **Anchors and aliases.** A frontmatter block with `&anchor` and `*alias` in it. What happens when a field that is an alias is set? When a field elsewhere in the document anchors a value that the edited field refers to? An editor that expands an alias while setting an unrelated key has rewritten a part of the file nobody asked it to touch.
- **Tags.** `!!str`, `!!binary`, and a custom tag such as `!Ref`. Story 1's ticket 28 made a tagged value read as text rather than fail, so documents with tags now pass through the read path and reach the write path.

Two more cases have joined them since:

- **Integers past 64 bits**, which story 1 also made read as text. A `set` on a neighbouring field must leave such a line byte-identical.
- **The block scalar fusion** already known from the first survey: replacing a block scalar fuses two lines (`note: newb: 2`). The reparse guard catches it. Establish whether it is the only such case or the first one found.

Build the smallest thing that answers these: a handful of real frontmatter blocks carrying anchors, aliases, tags, a block scalar, a long integer, a flow list and comments, each edited with one of the three operations, with the result compared byte for byte against what was intended. Record what `yaml-edit` does in each case, and whether the reparse guard catches it when the result is wrong.

The point is not to grade the crate. It is to know which of these cases the guard turns into a rejected write, because every one of those is a `set` that a user cannot complete, and the contract has to say so rather than let them find out.

## What the prototype found

Run 2026-09-21. Nothing is decided by this section; it is the evidence the decision will be made
from, and the ticket stays open.

### How to run it again

A single throwaway binary crate outside this repository, with three dependencies and the same pins
the read path already uses:

```toml
[dependencies]
yaml-edit = "=0.3.2"
yaml_serde = "0.10.7"
serde = { version = "1.0.229", features = ["derive"] }
```

`yaml-edit` 0.3.2 was published 2026-09-17 and is still the latest release. The editing calls used
are the ones a write seam would use: `Document::from_str`, `doc.as_mapping()`, `mapping.set(key,
value)`, `mapping.get_sequence(key)` with `push` and `remove`, and `doc.to_string()`. The reader is
`yaml_serde::from_str`, into a struct of `String` fields where the read path's own shape matters and
into `BTreeMap<String, String>` where it does not.

Each case is one source document, one operation, and three questions asked of the result: which
lines differ from the source byte for byte, whether the reader can parse the result at all, and what
the edited key reads back as. Twelve edit cases were run that way, and twenty-seven further checks
covering the reader, the entry point, the values written, and the other two operations.

### Cases that passed: the edit touches the target line and nothing else

Setting a scalar that is not involved in the feature under test left every other line byte-identical,
with the result parsing and the new value reading back exactly. The features present in the document
while that was done:

| In the document | Source | Lines changed |
| --- | --- | --- |
| Anchor and a merge key | `defaults: &defaults` … `job:` with `<<: *defaults` | 1, the target |
| An alias | `base: &b hello` / `copy: *b` | 1, the target |
| Tags | `a: !!str 123` / `b: !Ref something` / `c: !!int 7` | 1, the target |
| Values the read path keeps as text | `big: 12345678901234567890123`, `hex: 0755`, `ver: 1.10`, `sci: 1e3`, `flag: no`, `date: 2026-09-21` | 1, the target |
| A block scalar | `note: \|` with two indented lines | 1, the target |
| A flow list | `tags: [a, b, c]` | 1, the target |

The other two operations behave the same way. Appending to a block list added one line and left the
rest; removing an item removed one line; adding a key appended it at the end; and appending to a
flow list kept the flow style, `tags: [one, two]` becoming `tags: [one, two, three]`.

Writing a value is safe in the sense that matters most here: twenty-one values were written and every
one round-tripped back as the same string. That includes every literal that YAML would otherwise read
as something else — `no`, `yes`, `on`, `off`, `true`, `null`, `~`, `123`, `1.10`, `1e3`, `0755`,
`12345678901234567890123` — as well as `a: b`, `# not a comment`, `*star`, `&amp`, the empty string,
and values with a leading or trailing space. Each came out single-quoted. A value containing a
newline was written as a block scalar (`|-`) and round-tripped. One exception in form only:
`2026-09-21` was written bare, as `k: 2026-09-21`, and still read back as the string it was given.

### Cases the guard catches, so the write is refused rather than made

Both of these produce a result the reader cannot parse, so the mandatory re-read rejects the write
and the file on disk is never replaced.

**Setting the key that holds an anchor.** `base: &b hello` / `copy: *b` / `title: old`, setting
`base` to `replaced`, gives `base: replaced` and leaves `copy: *b` pointing at an anchor that is no
longer defined. The reader fails with `unknown anchor at line 2 column 7`.

**Setting a key whose value is a block scalar.** This one is worse than the earlier survey recorded.
The survey noted that replacing a block scalar fuses two lines. What it actually does here is fuse
the value with the following key and lose both:

```
note: |            note: shortb: 2
  line one    ->
  line two
b: 2
```

Four lines become one, and the unrelated key `b` is swallowed. The reader fails with `mapping values
are not allowed in this context at line 1 column 13`, so the guard holds — but the failure is larger
than "the edited value is wrong", and an editor that did this without a guard would destroy a
neighbouring field.

### Cases the guard does not catch

In both of these the result parses and the edited key reads back as exactly the value that was asked
for, so a guard that compares the intended change against the re-read has nothing to compare against
and passes. What is lost is not the value.

**A tag is dropped when its key is set.** `a: !!str 123`, setting `a` to `456`, gives `a: '456'`. The
tag is gone. The reader parses it and returns the string `456`, which is what was intended, so the
write goes through.

**The quote style is dropped when the key is set.** `name: 'single'   # trailing comment`, setting
`name` to `changed`, gives `name: changed   # trailing comment`. The trailing comment survives; the
single quotes do not. The reader returns the string `changed`, as intended, so the write goes
through.

Both are confined to the key being edited. Neither has been seen to affect a line the edit did not
touch.

### Two further facts the probe turned up

**`Document::from_str` refuses a document with a comment before the first key.** It returns
`Invalid operation 'Document::from_str': Input contains stream-level comments outside the document.
Use YamlFile::from_str() to preserve them.` A frontmatter block that opens with a comment is
ordinary, so this is not an edge case. `YamlFile::from_str` accepts the same input, and setting a key
through it preserved the leading comment in the output. Which entry point the write seam uses is part
of [decision 7](7-the-write-seam-and-the-clock.md); this ticket only records that the first one does
not accept the input and the second does.

**Reading a long integer depends entirely on reading into typed `String` fields.** The same document
that reads correctly into a struct of `String` fields — `big` coming back as
`"12345678901234567890123"`, every digit — fails outright when read into an untyped value:
`invalid type: integer 12345678901234567890123 as u128, expected any YAML value at line 1 column 6`.
That is a hard error, not a silent rounding. It belongs to [decision 10](10-a-number-past-u64-on-the-write-path.md),
and it means any write path that passes a document through an untyped value fails on documents the
read path accepts today.

## Answer

The question this ticket asked — which of these cases the guard turns into a rejected write — no
longer has a subject. [Decision 20](20-the-frontmatter-writer.md) writes frontmatter with
`yaml_serde`, from the text the read path already keeps, and does not add `yaml-edit`. There is
no editor doing surgery on a line, so there is no case where a document's own shape makes a
`set` impossible.

The findings above are kept and are not wasted: they are the evidence decision 20 was made from.
Two of them in particular carried it — that a tag and a quote style are dropped on the edited
line without the guard noticing, which was the reason the crate had been chosen; and that a
block scalar is not merely fused but swallows the following field whole.
