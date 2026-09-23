# 10: Set up this repo's `.typdoc/` project and author the four catalog documents

Type: implementation
Status: open
Blocked by: 23

**HOLD, 2026-09-23 (Aria):** waiting on M-11 from Mild — a real conflict found before this
ticket started. A coded collection's `match` must contain `{key}` exactly once (design.md's own
rule), which `CAT` (ticket 6's proposed code for `docs/design/catalog/`) can't satisfy alongside
fixed, meaningful filenames like `rules.md`/`commands.md`. Aria's proposal to Mild: `catalog` has
no code (path-identified, so filenames stay fixed); `spec` keeps `SPC`; the frontmatter field is
`body_type` (snake_case, typdoc's own frontmatter convention, e.g. `blocked_by`), not
`body-type`. **Do not start this ticket until M-11 lands** — see ticket 23. Tickets 11, 12, and
22 are transitively blocked through this one.

Contract decision 3 and ticket 5/6's answers give the shape; this ticket builds it and fills it
with today's real data (the same five sets `design.rs` extracts from `design.md` today, read by
hand from `docs/design/design.md` before it moves — not copied from `design.rs`'s own fixture,
which is a test double, not the source).

## The work

1. **`.typdoc/config.json`** for this repository (none exists yet), one namespace, two
   collections:
   - `docs/design/spec/` — prose, no fields beyond what typdoc requires of any document.
   - `docs/design/catalog/` — one schema requiring a `body-type` frontmatter field (`enum`,
     `values: ["json"]`, required). No other fields required; a catalog document's actual content
     is its body, not its frontmatter.
2. Managed with the `v0.1.0` binary (M-5). If a step is blocked by a bug in that binary, do the
   step by hand instead and name the gap in this ticket's report — do not wait on a typdoc fix.
3. **Four catalog documents**, each `body-type: json`, body exactly one JSON object:
   - `docs/design/catalog/rules.md`: `{"rules": [{"id": "<id>", "configurable": <bool>}, ...]}`
     — every rule id in `design.md`'s two Validation-rules tables today, `configurable: true` for
     ids from the table whose second column is `Default`, `false` for the table whose second
     column is `Checks`. One list, not two (contract decision 1).
   - `docs/design/catalog/commands.md`: `{"commands": [...]}` — every `### typdoc <name>` heading
     in `design.md`'s Commands section today.
   - `docs/design/catalog/exit-codes.md`: `{"codes": [...]}` — every code in the exit-codes table.
   - `docs/design/catalog/frontmatter-losses.md`: `{"losses": [...]}` — every row of the "Written
     in the file" / "After any write" table, text kept short as today's fixture holds it.
   Read these from `docs/design/design.md` directly (it is still in place when this ticket runs,
   unless ticket 13 has already moved it — check first), not from `crates/typdoc-testkit/src/design.rs`'s
   own fixture text, so a copy-paste from the wrong source isn't how a stale entry gets in.

## Tests

- `typdoc validate` (or `--audit`) on this project reports the four catalog documents and any
  `docs/design/spec/` documents clean.
- A manual read-back: every id `design.rs`'s current functions (`rule_ids`,
  `always_on_rule_ids`, `configurable_rule_ids`, `command_names`, `exit_codes`,
  `frontmatter_losses`) return today, run once against `design.md` before this ticket starts,
  matches the corresponding catalog document's contents exactly — a diff, not a glance.

## Done

- `.typdoc/config.json`, both collection files, and the catalog schema exist and validate clean.
- The four catalog documents exist with content matching `design.md`'s current tables exactly
  (verified by the manual read-back above, recorded in the report).
