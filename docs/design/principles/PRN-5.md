---
title: One rule for documents with and without a code
status: active
---

**Documents with and without a code follow the same rules, unless a difference can be named.**

## Why

A user learns typdoc once. Every rule that holds for one kind of document and not the other is a
rule the user has to remember and an agent has to be told about. When the two kinds already share
a weakness, adding a guard to one of them does not protect anyone; it only makes the two behave
differently.

## What follows

- A reference that may go stale after a rename is allowed for both: a path ref to a document
  without a code, and a key written with its slug. Neither is warned about for being written that
  way.
- Both kinds are renamed with `mv`, and both keep their references in the form they were written
  (`PRN-4`).

## Where it stops

Numbering exists only for coded documents, and everything that follows from a key (issuing,
renumbering, resolving by key) has no counterpart for a path. That difference is named, so it is
allowed.
