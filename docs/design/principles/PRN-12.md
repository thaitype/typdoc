---
title: A name points at one thing; a pattern selects many
status: active
---

**A name points at one thing; a pattern selects many.** No form of a name takes a glob, and every
place that selects several things with a pattern writes it with one syntax.

## Why

A name must keep meaning the same thing as a project grows. A glob in a name would change what it
points at when a matching folder or document appears, and a reader of the name could not tell. A
pattern, where the user asks to select several things, is expected to take in what appears; the
same characters meaning different things in two such places is a rule each reader has to learn
again.

## What follows

- A key, a path, a portable name, a namespace prefix and an import alias are exact. `story-*:WF-1`
  is not a name; `WF-1 --namespace story-*` is a selection.
- Globs appear only where the user asks to select several: `namespaces`, `--namespace` and
  `TYPDOC_NAMESPACE`, a collection's `match`, `--where` and `--if` values, the body link `ignore`.
  Each takes the characters of one syntax, and none gives a character a meaning of its own.
- A place refuses a character of the syntax it does not take, rather than reading it as an
  ordinary character that matches nothing.

## Where it stops

Each place may take a subset of the syntax, and says why: `**` only where folders are crossed, `!`
only where an exclusion is configured, `\` only where a value may hold `*` or `,` as itself.
