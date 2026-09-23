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
