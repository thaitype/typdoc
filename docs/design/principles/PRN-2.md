---
title: A key is the identity; a file name only decorates it
status: active
---

**A document is found by its key. The rest of a file name only helps a person read it, and a
part that no longer matches is reported, not treated as a broken reference.**

## Why

A key is issued once and never reused (`SPC-8`), so it is the one thing about a coded document
that does not change. Everything else a reference may carry alongside it, such as the words in a
file name, can drift without anyone meaning to change which document is meant. Failing a
reference because of the part that drifts punishes the reader for the part that does not matter.

## What follows

- A key written with a slug resolves by its key; an out-of-date slug is a finding at its own
  configurable level, not a missing target.
- `mv --renumber` issues a new key and keeps the rest of the name, because the rest describes
  the document, not its number.

## Where it stops

A body link is a file path, not a key (`SPC-14`), and it has to match the file exactly (see
`PRN-6`). A path ref to a document without a code has no key to fall back on and resolves by path.
