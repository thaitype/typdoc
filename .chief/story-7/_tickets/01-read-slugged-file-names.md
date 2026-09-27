# 01: Read slugged file names — key/slug split, `slug` collection key, name states

Type: implementation
Status: open
Blocked by: None (can start immediately)

## What this delivers

`tickets/WF-8-lock-order.md` is the document `WF-8` for every command that reads a project
(`list`, `get` by path, `validate`), and the collection's `slug` key says which form it expects.
Demo: a project with `WF-1.md`, `WF-2-x.md`, `WF-3-ร่าง.md` lists all three by key; under
`required` `WF-1.md` is still listed and `filename.pattern` names it with `collection` and `key`;
a bad `slug` value exits 2 with `config.collection-slug`.

## Scope (contract: Reading a file name; Collection files; `filename.pattern`)

- `template.rs`: maximal-digit key, optional `-<slug>` up to the segment's trailing literal;
  member name states `Expected` / `UnexpectedForm` / `InvalidSlug`; `render(key, slug)`
  (callers pass `None` in this ticket); T1-kind templates: `config.match-template` under
  `optional`/`required`, today's matching under `none`.
- `config::read_collection`: `slug` key, `config.collection-slug` (bad value, non-string,
  uncoded schema), exit 2. `rules.rs` `RULES`, `catalog/config-errors.md`,
  `fixtures/broken/config.collection-slug/` in the same commit.
- Index keeps each member's slug. `keys.unique` covers `WF-5.md` + `WF-5-x.md` (no new code
  expected; a test shows it).
- `filename.pattern` for `UnexpectedForm` and `InvalidSlug` members, with `collection` and `key`;
  stray files unchanged.
- Docs: `docs/reference/project-files.md` (`slug`), `docs/reference/validation.md` (the new
  `filename.pattern` cases), `skills/typdoc/references/project-layout.md`.

## Tests first (strict)

The `template.rs` unit list and the config/`filename.pattern` items in testing-decisions.md,
written red before the code. Plant: drop the maximal-digit rule, see the boundary tests fail.
