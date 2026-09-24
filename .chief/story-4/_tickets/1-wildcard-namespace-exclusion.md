# 1: Wildcard namespace exclusion — ordered `!` algorithm

Type: implementation
Status: resolved
Blocked by: None (can start immediately)

## What this delivers

A `namespaces` config entry (and only `namespaces` — see contract §1's "Config shape";
`--namespace`/`TYPDOC_NAMESPACE` explicitly do not get `!` this story) can carry `!`-prefixed
exclusions, gitignore-style, e.g. `["story-*", "!story-1", "!story-2"]`. Demoable end-to-end:
configure a wildcard + exclusion, show the excluded folder is invisible to `validate`/`list`/
`get`/`refs`, show `--namespace story-1`/`new --namespace story-1` fail as "not a namespace of
this project," show a namespace named only by an unmatched `!` produces no finding.

**Known limitation this ticket ships with, fixed by ticket 2:** if the excluded namespace has an
existing `.typdoc/state/<name>.json` (already issued codes), the whole project fails to load
(`config.state-orphan`). Don't try to fix that here — it's a separate, chained ticket so this one
stays a reviewable size. Do not demo this ticket against a namespace with prior state.

## Scope (see contract §1 for full detail)

- `namespaces.rs::resolve`: replace the order-independent `BTreeMap::extend` union with an
  ordered accumulate/remove pass — strip a leading `!` into `(negate, pattern)` per entry,
  resolve `pattern` with today's unchanged `entry_folders` matching, add (positive) or remove
  (negate) from the accumulated set. Empty-match reporting forks on `negate`: unchanged for plain
  entries, **always silent** for `!` entries (exact name or glob alike — this is the one place
  `!` behaves differently from a plain entry, not just "same rule, negated").
  `name_problem`/`nested` checks run only over the final set.
- Verify (write a test, don't assume) that `scope.rs::select`'s existing "not a namespace of this
  project" refusal already fires for `--namespace`/`TYPDOC_NAMESPACE` naming an excluded
  namespace, and for `new`/`mv --renumber` targeting one — contract says this needs no new code.
- Verify (write a test) that `--namespace '!story-1'` already gets `glob_match`'s "not a namespace
  name or a glob" syntax error, not a silent misread — also no new code expected, per contract's
  traced-through walkthrough of `scope.rs:164-190`. If the trace was wrong and it doesn't already
  error clearly, that's a real bug to fix here, not defer.
- Skill (`skills/typdoc/`) and user docs describing `namespaces` ship in this same ticket/PR
  (goal's "deliverable, not follow-on" rule) — document the `!` syntax, order/last-match-wins,
  full invisibility, and the silent-when-`!`-matches-nothing rule.

## Testing (contract's Testing Decisions, wildcard-exclusion + scope-selection bullets)

- Unit tests on `resolve()`: order/last-match-wins (later plain entry re-includes after an
  earlier `!`), `!` matching nothing (exact and glob, both silent, assert no report), plain entry
  matching nothing (unchanged, still asserted).
- Fixture/golden coverage for excluded-is-invisible (`validate`/`list`/`get`/`refs`) — extend
  `fixtures/valid`/`fixtures/broken` and regenerate goldens via `TYPDOC_REGENERATE_GOLDEN`, never
  hand-edit one.
- `select()` tests: excluded namespace named explicitly → not-found error; `--namespace
  '!story-1'` → syntax error.
