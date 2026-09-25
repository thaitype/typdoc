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
