# Goal

A coded document's file name can carry a slug after its key, `tickets/WF-8-lock-order.md`, and
the document is still `WF-8` everywhere: in refs, in arguments, in output and in numbering. The
design is already in `docs/design/` (`SPC-17` Slugs in file names, `SPC-14`, `SPC-2`, `SPC-1`,
`SPC-12`); this story makes the binary do what those sections say.

From a user's point of view, when the story ends:

- `typdoc new WF "Decide lock order" --slug lock-order` creates `tickets/WF-8-lock-order.md`.
  A collection's `slug` (`optional` by default, `required`, `none`) says whether `--slug` may or
  must be given, and a slug that breaks the character rule is refused with nothing written.
- Every command that reads a project treats `WF-8-lock-order.md` as the document `WF-8`. A file in
  the form its collection does not expect, or whose slug holds an excluded character, is still
  that document, and `filename.pattern` reports its name.
- A key written with its slug (`WF-8-lock-order`, `story-2:WF-8-lock-order`,
  `chief::story-3:WF-8-lock-order`) works wherever a key works, as a ref and as an argument,
  and resolves by the key. `validate` reports a ref whose slug is out of date under the new
  configurable rule `refs.slug` (default `warn`).
- `mv` changes a slug (a move to the same key with another slug), rewrites refs in the form
  each was written, and records the previous path in `auto: moves`. `mv --renumber` keeps the
  slug under the new key.
- `--json` output names the document with `key` alone; the slug is only in `path`.
- The catalogs list `refs.slug` and `config.collection-slug`, so `SPC-1`, `SPC-6` and the
  catalogs agree again.
- The user docs, the typdoc skill and the changelog describe slugs; the changelog's upgrade note
  lists the three ways a project that passed before can fail after the upgrade (`SPC-17`, A
  project from before slugs).

## Out of Scope

- File names of documents without a code: no change.
- typdoc making up a slug from a title, or rewriting a given slug into a valid one.
- Mentions of a key written with its slug in plain text: they stay unchecked (`SPC-1`,
  `body.mentions`).
- The lock timeout message on macOS: its own story.
- A release or version bump; merging PR #18 into `main` is outside the story.
