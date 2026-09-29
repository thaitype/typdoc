---
title: Globs
status: active
follows: [PRN-12]
---

A pattern selects several things; a name never takes one (`SPC-18`). Every place that takes a
pattern writes it with the syntax below, and takes the part of it that the place needs.

## The syntax

| Character | Means |
| --- | --- |
| `*` | Any run of characters, none included, within one path segment. In a value, which has no segments, any run of characters. |
| `**` | Any number of path segments, none included. |
| `,` | Separates alternatives: a name or a pattern matches if any of them does. |
| `!` | At the start of an entry, excludes what the entry matches. |
| `\` | Before `,`, `*` or `\`, means that character itself. |

There is no `?`, no `[...]` and no `{a,b}`. A glob segment does not enter a folder whose name
begins with `.`, and a literal segment does: naming a folder says it is wanted, and a glob says
"whatever is here". `*` matches a leading dot in a file name, since a file is named by the pattern
that reaches it rather than found by walking into it (`SPC-17`).

## Where each part is taken

| Place | `*` | `**` | `,` | `!` | `\` |
| --- | --- | --- | --- | --- | --- |
| `namespaces` in `config.json` (`SPC-7`) | yes | no | no: the entries are a list | yes | no |
| `--namespace`, `TYPDOC_NAMESPACE` (`SPC-7`) | yes | no | yes | no | no |
| a collection's `match` (`SPC-17`) | yes | yes | no | no | no |
| `--where`, `--if` values (`SPC-13`) | yes | read as `*` | yes | no | yes |
| `body.links`'s `ignore` (`SPC-1`) | yes | yes | no: the entries are a list | no | no |

Why each place takes what it takes:

- `**` is taken where folders are crossed: a namespace is one folder, so `namespaces` and
  `--namespace` refuse it; `match` walks folders; `ignore` matches paths. A value has no folders, so
  `**` in it is read as `*`.
- `!` is taken only in `namespaces`: an exclusion is a configuration concern, kept with the list it
  excludes from, while `--namespace` is a selection typed for one command.
- `\` is taken only in values, the one place a `*` or a `,` can be meant as itself: a file name or a
  namespace name holding one is not a case a pattern needs to reach.
- `ignore`'s `**` enters a folder whose name begins with `.`, unlike `match`'s: `ignore` filters the
  targets links name, and walks nothing, so there is no folder it would be entering.

A character of the syntax that a place does not take is refused: a config error in a file, bad
arguments (exit 1) on the command line. It is never read as an ordinary character that matches
nothing. In a value, `?`, `[` and `]` are ordinary characters, since a value may hold them.
