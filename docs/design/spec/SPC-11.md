---
title: Design documents explained
status: active
migrated_from: .chief/story-3/_contract/contract.md
---

typdoc's own design is kept as typdoc documents, in two collections of this repository's own
project under `docs/design/`.

## Spec and catalog

- **`spec`** (`docs/design/spec/SPC-<n>.md`, coded `SPC`) is prose for people. No code and no test
  reads a spec document, or any other design prose, as data: not to import a list from it and
  not to compare the code against it. Its fields are `title` (required), `status` (`draft`,
  `active` or `superseded`), `superseded_by` (a ref to the `SPC` that replaces it) and
  `migrated_from` (where the text was moved from).
- **`catalog`** (`docs/design/catalog/<name>.md`, no code: each document is known by its path)
  holds what code and tests read: the lists the code must agree with, such as the rule ids, the
  command names and the exit codes. Its fields are `title` (required), `content_type` (required,
  and `json` is its only value) and `explained_by` (optional, a list of refs to the `SPC`
  documents that explain it). The body is JSON only.

A test that checks the code against a list reads the catalog document, never the prose that
explains it, so the prose can be rewritten freely and the list is the one place the check rests
on.

## Reading a catalog document's body

`typdoc_core::read_json_body` reads a whole document, frontmatter and body, into a type the caller
names. It decides how to read the body from the document's own `content_type` field and from
nothing else: it is never given the document's path, and it does not look at the schema's name.
For `content_type: json` it parses the body as JSON into the caller's type. Every other outcome is
its own error, and none of them falls back to treating the body as prose:

- the frontmatter block cannot be read;
- there is no `content_type` field, which includes a document with no frontmatter block;
- `content_type` holds anything but `json`, an empty field included;
- the body is not JSON, or is JSON of a shape the caller's type does not accept.

Only tests call it. `validate` and `get` do not read `content_type`, and do not parse a body as
JSON. A catalog document is always a Markdown file with a JSON body; documents stored as plain
`.json` files are not designed.
