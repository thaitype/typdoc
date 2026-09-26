---
title: Schema fields explained
status: active
migrated_from: docs/archived-design/design.md#schema-format
---

A schema's `fields` maps each field name to a definition: a type and options. Two of the options
decide values that `typdoc` itself writes.

## `default`

`default` applies to every type and is filled in by `typdoc new`. A default that does not fit the
field's type is written as given and reported by `frontmatter.types`, like any other value a
write puts where its type does not fit.

## `auto`

`auto` applies to `date`, `datetime` and `list` fields:

- `create`: set by `new`, never changed after.
- `update`: set by `new` and by every `set` that changes a value.
- `moves` (`list` only): `mv` appends the document's previous key or path on every move, always
  with its prefix (`story-2:WF-5`, `notes/old-name.md`), `--renumber` included. Running the same
  `mv` again after a stop does not append it a second time.

`new` fills every default first, then every `auto` field, and applies the `--set` values last, so
an explicit value wins over both.

Field names carry no meaning; only `auto` does. A field such as `created_at` without `auto` is an
ordinary field that `typdoc` never fills. Neither `set` nor `new` may write an `auto` field
directly: it is a validation error. Because edits made outside `typdoc` (by hand, or a file tool
changing the body) are invisible to it, an `auto: update` field means "frontmatter last changed
through `typdoc`", not "file last modified". A `list` field with `auto: moves` is opt-in per
schema and holds plain strings, never refs, since they name documents that no longer exist under
that name; the `refs.moved` rule reads it. Without such a field nothing is recorded.
