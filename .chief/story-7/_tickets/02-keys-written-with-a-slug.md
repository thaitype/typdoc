# 02: Keys written with a slug — arguments, refs by key, `refs.slug`

Type: implementation
Status: open
Blocked by: 01

## What this delivers

`WF-5-json-output-shape`, `story-2:WF-5-…` and `chief::story-3:WF-5-…` work wherever a key does,
and resolve by the key. `validate` reports a stale slug under `refs.slug` (default `warn`).
Demo: `typdoc get WF-5-old-name` prints `WF-5`; a frontmatter ref with an old slug resolves and
gives one `refs.slug` warning; at `error`, `set` on that document is refused.

## Scope (contract: Keys written with a slug; `refs.slug`)

- One shared `<key>-<slug>` reader in `typdoc-core`, replacing `argument::looks_like_key` at
  `argument.rs` and `refs.rs` (`classify`, `resolve_into_project`); `links::looks_like_key_shape`
  (mentions) unchanged.
- Refs in key form keep their written slug; `Via::Key`; `refs.codedByPath` does not fire.
- `refs.slug`: `rules.rs` `RULES` + `CONFIGURABLE`, `catalog/rules.md`, `fixtures/broken/refs.slug/`
  in the same commit; `validate` finding at the ref; `new`/`set` pre-write check at its level,
  refusing only at `error` (SPC-2).
- Mentions: a test pins that `WF-3-lock-order` in text is not reported.
- Docs: `docs/reference/commands.md` (arguments with a slug), `docs/reference/validation.md`
  (`refs.slug`), `skills/typdoc/references/{commands,validation}.md`.

## Tests first (strict)

Arguments and refs items in testing-decisions.md. Plant: make the `refs.slug` comparison always
equal, see its tests fail.
