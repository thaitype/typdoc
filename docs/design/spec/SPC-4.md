---
title: Frontmatter losses explained
status: active
migrated_from: docs/archived-design/design.md#document-files
---

A write rewrites the whole frontmatter block, not the line it changed. No *value* changes when
it does, but the block's formatting is a best effort, not a promise, and seven kinds of detail
are lost on any write, including one that changes a single field:

| Written in the file | After any write |
| --- | --- |
| Comments, anywhere in the block | Gone |
| Blank lines between fields | Gone |
| `tags: [a, b]` | A block list, one item per line. An empty list is written `[]` |
| `title: 'Ship it'`, `status: "no"` | Unquoted, unless the text needs quotes to survive |
| `id:   WF-3` | One space after the colon |
| `&anchor` with `*alias` | The value written out in full at every place that used it |
| `!!str`, `!Ref`, any other tag | Gone; the value stays |

The last two change what the file means, not only how it looks: an alias becomes several
independent copies of one value, and a YAML tag another tool might read is dropped. A document
that depends on either should be edited by hand, not by `typdoc`.

`docs/design/catalog/frontmatter-losses.md` holds these same seven rows as short strings — the
set a test compares the write path's actual behavior against, corpus document by corpus
document.

## What a write keeps

A write never moves an existing key and never reformats the body, and a key that is added goes at
the end of the frontmatter block. No value changes when the block is rewritten: every value is
carried across as the text it was written with, and quoted where YAML would otherwise read it as
something else. `1e3`, `1.10`, `0755`, an integer longer than 64 bits, `no`, `~`, a date, an
empty string, and text holding a newline, a tab or a leading space all come back exactly as they
were.

A field written with no value at all is not the same as one written as an empty string.
`reviewer:` is YAML's null and `reviewer: ''` is a string of no characters, and a tool that reads
the file after typdoc can tell them apart even where typdoc does not need to. Both are kept as
they were written, and a write puts back the form it found, because a write changes no value and
this is a value, not a style. Inside typdoc the two mean the same: for every rule, for `--set`,
for a template and for a query, a field written with no value is a field that is present and
holds no text, exactly as a field written `''` is. A required field written with no value is
present, not missing, and a `number` written with no value fails its type.

## How a write is made

The block is written with the YAML library the reader uses, from the text the read path kept, so
the writer and the reader cannot disagree about what a document says, and no value is parsed on
its way to disk. Every document the read path accepts can be written: a write that cannot be made
at all is worse than one that reformats, and rewriting the whole block loses the same things on
every run, which is why the table above can list them in advance.

The writer sits behind a small set of operations (set a scalar, set a list, append, remove or
replace one list item, add a key, remove a field) and one call that turns the block into text, so
what stands behind them can change without a caller changing.

A write does not read its own block back before it lands. A round trip that changed a value would
be a defect in the YAML library that every read already trusts without a second check, and not
one typdoc could correct; checking the write and not the read would also leave a misread document,
which is a wrong answer to every command, unchecked. That typdoc assembles the block it meant to,
with the right fields, the right text and the right order, is checked by a test at the boundary of
typdoc's own code, run over every fixture document.

## What counts as frontmatter

A document is YAML frontmatter plus a free Markdown body; `typdoc` owns only the frontmatter. A
file has frontmatter when it begins with a `---` line that opens a block. A block that is present
but empty counts: it is a document that declares itself and has no fields yet, and it is checked
like any other, so every required field it lacks is a finding. A file with no block at all is an
ordinary Markdown file. The two are not merged, because a document missing every required field
would otherwise be filed with the files that are not typdoc's, with no signal. A block that is
present but cannot be parsed is not an absent block: it is the finding `frontmatter.parse`, so a
damaged document is never taken for an ordinary Markdown file. The finding has no position: the
YAML reader gives none for some errors and an imprecise one for others (the start of the mapping,
not the second key, for a duplicate key), and it does not say which kind of error it met, so no
position is kept for any.
