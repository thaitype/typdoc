---
title: A document has one identity; a name is that document written from a context
status: active
---

**A document has one identity; a name is that document written from a context.** What typdoc prints
in a context is accepted in that context, and its portable name (`ref`) in every context. A name
typed in a ref and on the command line means the same, except a path with no prefix, which is
relative to where it is written. `path` is where the file is, for opening it.

## Why

People and agents copy names between the places typdoc reads them: a ref in frontmatter, a link in
a body, a command's argument, and what a command printed. A name that finds a document in one of
them and nothing in another looks like a missing document, not like a rule to learn, and the
difference is found only when something breaks.

## What follows

- A ref is a name. The direction is a ref that means the same from every document: a path in a ref
  carries its `namespace:` or `project::` prefix, and a key may drop its prefix, since it is read in
  the writer's namespace. The grammar reads a path with no prefix in a ref too, from the base it
  gives it, and requiring the prefix is not part of it.
- One grammar reads every name, and one function writes every name, as its inverse. A ref and an
  argument differ only where the grammar leaves the choice to the place: the base of a path with no
  prefix.
- A key, and a path with a `namespace:` or `project::` prefix, name the same document wherever they
  are written. A form accepted in one place is accepted in the others in the same release.
- Every document typdoc prints carries its portable name beside its `path`, and that name is
  accepted in a ref and as an argument, and in a body link when it is a path.
- Reading a name typdoc wrote in a context, in that same context, gives back the document it was
  written for.

## Where it stops

- A path with no prefix is relative to where it is written: the document or its namespace folder in
  a ref, the project folder as an argument, and `./` the current directory there. `./` and `../` in
  a ref are always from the document's folder; `refBase` moves only a path with no prefix.
- A body link is not a name but a Markdown link, read as Markdown readers read it (`PRN-6`): a path
  from the document, never a key.
- `path` stays where the file is, from the project folder: it is for opening the file, not a name to
  write elsewhere.
