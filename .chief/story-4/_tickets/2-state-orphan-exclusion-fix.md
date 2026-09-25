# 2: Excluding a namespace with existing state must not break project load

Type: implementation
Status: resolved
Blocked by: 1

## What this delivers

Fixes the load-fatal conflict found during contract (contract §1, item 6): `state::orphans`
flags any `.typdoc/state/<name>.json` whose `<name>` isn't in the *current* `config.namespaces`,
and `config.state-orphan` is not a `validate`-only finding — it joins every other config error
`load_inner` fails the whole project load on. Once ticket 1 makes `resolve()` drop an excluded
namespace from `config.namespaces`, excluding a namespace that has ever issued a code (the
motivating scenario for this story — `story-1`, `story-2` already in use) would otherwise break
**every**
typdoc command on that project. This ticket is what makes the feature actually usable for that
case, not an optional follow-up.

## Scope (contract §1, item 6)

- `namespaces::Resolved` gains the set of namespace names matched by at least one entry and then
  removed by a later `!` — names the config's own patterns actually reached and excluded, not
  "everything on disk."
- `state::orphans`'s `known` set becomes `config.namespaces` names ∪ this excluded set.
- Edge case, already confirmed correct by design (no extra code needed for it, just don't break
  it): a folder that no longer exists at all, named only by a `!` entry that therefore matches
  nothing, adds nothing to the excluded set — its leftover state file is a genuine orphan and
  still fires `config.state-orphan`.

## Testing — the gate for this ticket (write these red against today's code first)

1. A namespace with an existing state file that has already issued codes (e.g. up to `TK-3`)
   gets excluded via `!` → `validate`/`list`/`get`/`new` (in a *different* namespace) must all
   succeed — not exit on `config.state-orphan`.
2. Remove the `!` (re-include the namespace) → `new` in that namespace issues `TK-4`, not `TK-1`,
   and the state file's bytes are byte-for-byte unchanged from before exclusion to after
   re-inclusion — proving exclusion truly never read or wrote it.
3. A state file whose folder no longer exists and matches no entry (positive or `!`) at all still
   fires `config.state-orphan` — the true-orphan case stays caught.

If fixing this reveals a doc line from ticket 1 needs a clarifying sentence (state survives
exclusion, continues numbering on re-inclusion), add it here rather than opening a new ticket.

## Also in scope: fix the stale `NoProjectAt`/`NoProject` error text (found by manually running
ticket 3's binary, 2026-09-24)

Ticket 3 made project discovery folder-based (`.typdoc/` existence, not `config.json`
existence), but `crates/typdoc-core/src/error.rs:33` and `:36` still read `"no project found:
there is no .typdoc/config.json in {from} or above it"` and `"no project found: {dir} has no
.typdoc/config.json"` — wrong now that a bare `.typdoc/` folder with no `config.json` is a valid
project. Reword both to say there is no `.typdoc/` folder (exact wording is this ticket's call).
Grep `skills/typdoc/SKILL.md` and `skills/typdoc/references/exit-codes.md` for the old wording
and update alongside. Add a test asserting the corrected message text (there wasn't one covering
the exact wording before — the existing tests that "lock in the exact wording," per ticket 3's
own note, were checking something else; verify that claim while here). Folded into this ticket
rather than opened separately since it's a small fix in the same file ticket 2 already touches.
