# 05: `mv --renumber` keeps the slug

Type: implementation
Status: open
Blocked by: 03, 04

## What this delivers

`story-2:WF-5-json-output-shape` renumbered into `story-3` becomes
`story-3/_tickets/WF-8-json-output-shape.md`; refs written with the slug get the new key and the
same slug.

## Scope (contract: `mv`, `--renumber`)

- `mv_renumber` passes the source slug to `allocate_key`/`render`.
- Full-form refs rewritten to the new key with the same slug, prefix form as today.
- Destination `slug` expecting the other form: carried out, `filename.pattern` in the findings,
  exit 0.
- Docs: `docs/reference/commands.md` (`--renumber`), skill `references/commands.md`.

## Tests first (strict)

The `mv_renumber.rs` items in testing-decisions.md. Plant: pass `None` as the slug, see the
kept-slug test fail.
