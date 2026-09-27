# 04: `mv` changes a slug

Type: implementation
Status: open
Blocked by: 01, 02

## What this delivers

`typdoc mv story-2:WF-5 story-2/_tickets/WF-5-json-shapes.md` renames the file, keeps the key,
rewrites refs in their written form and records the previous path in `auto: moves`.

## Scope (contract: `mv`)

- Relax the coded-source refusal (`project.rs` ~3558) only for a destination that fits the
  source's own template with the same key; everything else keeps today's messages.
- Destination `UnexpectedForm`: carried out, `filename.pattern` in the findings, exit 0.
  Destination `InvalidSlug`: exit 1, nothing written.
- Refs: key-only untouched; full-form gets the new slug, or becomes key-only when the slug is
  removed (prefix kept); body links and path refs as for any `mv`.
- `auto: moves` records the previous path.
- Docs: `docs/reference/commands.md` (`mv`), `docs/how-to/move-and-rename.md` (changing a slug),
  skill `references/commands.md`.

## Tests first (strict)

The `mv.rs` items in testing-decisions.md. Plant: accept any key in the same folder, see the
"another key is refused" test fail.
