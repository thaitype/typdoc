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

## What counts as frontmatter

A document is YAML frontmatter plus a free Markdown body; `typdoc` owns only the frontmatter. A
file has frontmatter when it begins with a `---` line that opens a block. A block that is present
but empty counts: it is a document that declares itself and has no fields yet, and it is checked
like any other, so every required field it lacks is a finding. A file with no block at all is an
ordinary Markdown file. The two are not merged, because a document missing every required field
would otherwise be filed with the files that are not typdoc's, with no signal. A block that is
present but cannot be parsed is not an absent block: it is the finding `frontmatter.parse`, so a
damaged document is never taken for an ordinary Markdown file. The finding has a position when
the YAML reader gives one, and the reader does not give one for every error.
