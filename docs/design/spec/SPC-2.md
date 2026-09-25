---
title: Commands explained
status: active
migrated_from: docs/archived-design/design.md#commands
---

`typdoc` has nine commands: `new`, `get`, `list`, `set`, `toc`, `refs`, `mv`, `pull`, and
`validate`.

- `new` creates a document — either the path to create, or the code of a coded schema, which
  allocates the next key.
- `get` reads one document's frontmatter.
- `list` queries documents by schema, `--where` conditions and sort order.
- `set` updates one or more fields, optionally guarded by an `--if` compare-and-set condition.
- `toc` lists the headings of a document's body with their line ranges.
- `refs` shows a document's outgoing or, with `--reverse`, incoming refs.
- `mv` moves a document within the project and rewrites every ref this project holds to it.
- `pull` re-fetches a pinned remote schema and compares it with the recorded hash.
- `validate` checks the project, or named documents, against its schemas and rules.

`docs/design/catalog/commands.md` holds this same list as plain strings — the set a test
compares the CLI's own command table against, so a command added to one and not the other is
caught rather than drifting apart silently.

**`mv`'s own-path check.** A destination that names the same file as the source is refused at
exit 7 with nothing written, but the message told apart by which of two distinct cases it is.
The same path given twice gets its own message, naming the document and saying nothing was
moved. A name that differs from the source only in case, on a file system that does not tell
the two names apart, keeps the other message: that the file system does not distinguish the two
names, so no change was made. `mv` asks the file system whether the two paths are one file
rather than comparing the text of them, since that is the question whose answer decides whether
anything can be written; which message is shown then follows from whether the two paths were
the same text to begin with.
