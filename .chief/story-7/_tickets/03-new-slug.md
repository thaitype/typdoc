# 03: `new --slug`

Type: implementation
Status: resolved
Blocked by: 01

## What this delivers

`typdoc new WF "Decide lock order" --slug lock-order` creates `tickets/WF-8-lock-order.md`.
Demo: the golden `new --slug --json`; each refusal exits 1 and leaves `last` where it was.

## Scope (contract: `new --slug`)

- clap `--slug <SLUG>` on `new`; with a path target `BadArgument`.
- Before the lock and allocation: character rule, `--slug` under `none`, missing `--slug` under
  `required` — each exit 1, no number spent.
- `allocate_key` passes the slug to `render`.
- Golden case `fixtures/output/new/coded-slug/`.
- Docs: `docs/reference/commands.md` (`new --slug`), `skills/typdoc/SKILL.md` and
  `references/commands.md`.

## Tests first (strict)

The `new` items in testing-decisions.md. Plant: move the slug check after allocation, see the
"`last` unchanged" test fail.
