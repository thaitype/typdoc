---
title: Never write what is invalid; keep what exists and report it
status: active
---

**A write that would produce something invalid is refused, with nothing written. Something invalid
that already exists is kept and reported.**

## Why

typdoc should never be the one that makes a project worse: refusing a write costs the caller one
retry, while writing something invalid leaves a problem for whoever meets it next. But a project
is edited by more than typdoc, and a document that is already wrong is still a document someone
refers to. Dropping it from the project, or refusing to read it, would turn one finding into many
broken references.

## What follows

- `new` validates before it writes and refuses a value the schema rejects (`SPC-2`).
- A file already on disk whose name does not match its collection's expected form is still that
  document, and `filename.pattern` reports it.
- `mv` onto a schema the document does not satisfy is carried out and reported, exit 0
  (`SPC-2`): the move itself produces nothing invalid; the document was never checked against
  that schema before.

## Where it stops

Whether a write "produces something invalid" is judged on what the write itself creates, not on
what the document was already carrying.
