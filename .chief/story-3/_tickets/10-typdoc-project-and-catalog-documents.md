# 10: Set up this repo's `.typdoc/` project and author the four catalog documents

Type: implementation
Status: resolved
Blocked by: None (M-11 landed 2026-09-23 — see ticket 23's Answer for the full decision)

**Unblocked 2026-09-23.** M-11's actual answer differs from Aria's proposal in two ways this
ticket must build to, not the original draft below: `spec` keeps a **code** (`SPC`), and the
field is **`content_type`**, not `body_type`. Both schemas also gained fields neither ticket 6
nor this ticket's first draft had. See "The work" below, already updated; ticket 23's Answer has
the full quote and reasoning.

**Follow-up, 2026-09-24 (Mild): `explained_by` changed from `ref` to `ref[]`, and from required to
optional** — spec documents may not exist yet during migration, and a catalog document may
reasonably explain itself with more than one. Built directly (not a new ticket): `schemas/catalog.json`
updated, and the four catalog documents' frontmatter changed from `explained_by: SPC-N` to
`explained_by: [SPC-N]`. The four `SPC` entries this ticket authored below (back when the field
was read as required) needed no change themselves — only the field's shape and cardinality moved,
not what it points at or why. See contract decision 2 for the current, correct shape.

Contract decision 3 and ticket 5/6's answers give the shape; this ticket builds it and fills it
with today's real data (the same five sets `design.rs` extracts from `design.md` today, read by
hand from `docs/design/design.md` before it moves — not copied from `design.rs`'s own fixture,
which is a test double, not the source).

## The work

1. **`.typdoc/config.json`** for this repository (none exists yet), one namespace, two
   collections:
   - `docs/design/spec/` — **coded**, code `SPC`, files `spec/SPC-<n>.md`. Schema: `title`
     (string, required), `status` (enum `draft|active|superseded`), `superseded_by` (ref → `SPC`,
     present only when superseded), `migrated_from` (string, the source location in
     `docs/archived-design/`).
   - `docs/design/catalog/` — **no code**, path-identified: `catalog/rules.md`,
     `catalog/commands.md`, `catalog/exit-codes.md`, `catalog/frontmatter-losses.md`. Schema:
     `title` (required), `content_type` (enum `["json"]`, required), `explained_by` (ref → `SPC`,
     written as a key like `SPC-4`, not a path).
   - **`explained_by` is grouped with two required fields in M-11's answer with nothing marking
     it optional (unlike `superseded_by`, explicitly "only when superseded") — treat it as
     required unless building against it proves that reading unworkable.** If required, each of
     the four catalog documents needs a real `SPC` document to point at: author four short `SPC`
     entries (`status: active`) as part of this ticket, one per catalog document, each explaining
     that document's rules/commands/exit-codes/losses in prose. This is a floor, not the full
     `docs/design/spec/` migration (that stays ticket 22/later work) — four short entries, not a
     comprehensive rewrite of everything `design.md` ever said.
2. Managed with the `v0.1.0` binary (M-5). If a step is blocked by a bug in that binary, do the
   step by hand instead and name the gap in this ticket's report — do not wait on a typdoc fix.
3. **Four catalog documents**, each `content_type: json`, body exactly one JSON object:
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

- `.typdoc/config.json`, both collection files, and both schemas exist and validate clean.
- The four catalog documents exist with content matching `design.md`'s current tables exactly
  (verified by the manual read-back above, recorded in the report).
- If `explained_by` is built as required: four `SPC` documents exist, one per catalog document,
  each a short, real explanation — not a placeholder — and every `explained_by` ref resolves.
