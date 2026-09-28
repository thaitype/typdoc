---
blocked_by: []
status: claimed
title: Characterisation tests of how every name form is read today
type: implementation
---

# TK-1: Characterisation tests of how every name form is read today

## What this delivers

A test that states, for every form of a name, what typdoc does today in a frontmatter ref, in a body link and as
a command's argument from the project folder and from a namespace folder, and the names commands print. It is
written before the name grammar is unified, changes no behaviour, and is the net the parser is rebuilt under: a
row the unification changes is changed on purpose, in the same commit as the code.

## Scope

- `crates/typdoc/tests/name_forms.rs`: sixteen forms (seventeen on Unix, with an absolute path), and the names
  `list` prints for a multi-namespace project and for an import.

## Checks

Passes on Linux, macOS and Windows; a wrong expected value is reported with its form and place.
