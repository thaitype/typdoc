---
title: Validation rules explained
status: active
migrated_from: docs/archived-design/design.md#validation-rules
---

Every rule `typdoc validate` can report falls into one of two groups.

**Always on.** Fifteen rules — `schema.valid`, `frontmatter.parse`, `frontmatter.types`,
`frontmatter.transitions`, `refs.resolve`, `refs.target`, `refs.acyclic`, `keys.unique`,
`collections.overlap`, `collections.empty`, `state.missing`, `state.malformed`, `state.behind`,
`state.retired`, and `files.unreadable` — cannot be turned off or reconfigured. Queries and
writes depend on the correctness they guarantee, so there is no `validation` key that changes
their level. `collections.empty` fires once, at `warn`, when the project has no collections at
all, regardless of what else `.typdoc/` holds — it is a finding of `validate`'s, not a config
error, so it never stops another command from running.

**Configurable.** Nine rules — `body.links`, `body.anchors`, `body.mentions`,
`refs.codedByPath`, `refs.moved`, `names.shadowed`, `frontmatter.unknown`, `filename.pattern`,
and `imports.absent` — have a default level and can be raised, lowered, or turned off
project-wide under `validation.global` in `config.json`, or per collection in that collection's
own file. `--strict` raises every remaining `warn` to `error`.

`docs/design/catalog/rules.md` holds the machine-readable form of this same list: one entry per
rule id, each carrying `configurable: true` for a rule in the second group and `configurable:
false` for a rule in the first. This document explains why the split exists; that one is what
code and tests read.

## Merge order

typdoc's defaults, then `validation.global`, then the collection's `validation`. A collection file
merges key by key, so it states only what differs, and this holds inside one rule's own setting
as well: a collection that sets only `level` keeps every option `validation.global` gave the same
rule, and an option the collection does set replaces only that option.

## `frontmatter.parse`

The frontmatter block cannot be parsed: invalid YAML, or a block that is never closed. No other
rule is evaluated for that file, and it takes no other part in a run: a query has no fields of
it to filter, sort or print by, and a reverse lookup reads no refs from it.

## `body.links`

Markdown links in the body (inline, image and reference-style) point at existing files; also text
that looks like a link but is not, and a reference label defined twice. Its `ignore` option holds
globs of relative targets to skip, matched after percent-decoding.

## `refs.moved`

A ref, or a mention when `body.mentions` is on, points at a key or path recorded in some
document's `auto: moves` field and no longer resolves. It replaces the ordinary missing-target
finding for that ref and names the new key. A body link is a ref here like a frontmatter value,
so a body link to a moved target is `refs.moved`, not `body.links`; it is looked up without its
`#anchor`, since a recorded move never has one. Without a field with `auto: moves`, a moved ref
is still reported as missing, without the new key.

## `body.mentions`

It checks plain-text keys and never turns them into refs, so `refby` never counts a mention and
`mv` never rewrites one.

| Text | Checked |
| --- | --- |
| `see WF-3` | yes |
| `` `WF-3` `` (inline code) | per `inlineCode` |
| Inside a fenced code block | per `fencedCode` |
| `[WF-3](WF-3.md)` | no; `body.links` checks it |
| `UTF-8`, `SHA-256` | no; not a known code |
| `WF-3a`, `xWF-3` | no; word boundaries required |

A mention with no prefix is looked up in the document's own namespace only, and a mention with a
sibling prefix (`story-2:WF-5`) in the namespace it names. A mention has one outcome for every
lookup that fails, not found, unlike a ref, whose `unresolved` tells the causes apart.
`fencedCode` defaults to `false` because code blocks often hold logs, commands and diffs that
contain key-like text.
