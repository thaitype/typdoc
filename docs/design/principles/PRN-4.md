---
title: A move keeps each ref in the form it was written
status: active
---

**When `mv` rewrites a reference, it keeps the form the reference was written in.**

## Why

Each way of writing a reference is a choice the writer made: a bare key, a key with a prefix, a
relative path, a link wrapped in `<…>`. A tool that normalises every reference on its way past
turns a small move into a large, noisy diff, and overrides choices it was not asked about.

## What follows

- A key-only ref stays key-only; a key written with its slug gets the new slug; a prefixed ref
  keeps its prefix; a relative path stays relative, recomputed from the document that holds it
  (`SPC-2`).
- A body link keeps its `<…>` or `%20` form.

## Where it stops

When the old form cannot name the new place (a prefix to a namespace the document has left, a slug
that has been removed), `mv` writes the nearest form that can, and says nothing more about it.
