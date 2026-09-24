# 23: Catalog collection's code, and the body-type field's name (M-11)

Type: wayfinder:grilling
Status: resolved
Blocked by: None (can start immediately)

**Decider: Mild.** Raised by Aria as M-11, 2026-09-23, before any build on ticket 10 started.
Blocks tickets 10, 11, 12, and 22.

## Question

Ticket 6 (M-4) gave `docs/design/catalog/` the code `CAT`. `design.md`'s own rule for a coded
collection: its `match` template takes `{key}` exactly once and allows no globs — the filename is
generated from an allocated key (`CAT-1.md`, `CAT-2.md`, ...), not chosen by whoever writes the
document. But ticket 5's design needs four catalog documents with fixed, meaningful names
(`rules.md`, `commands.md`, `exit-codes.md`, `frontmatter-losses.md`) — a coded collection can't
give them that.

Separately, ticket 6/5's `body-type` field name was chosen to match typdoc's kebab-case CLI-flag
style (`--lock-timeout`); typdoc's own frontmatter field convention, seen in an existing fixture
schema, is snake_case (`blocked_by`).

Aria's proposal, put to Mild: `catalog` drops its code entirely (path-identified, so the four
files keep their fixed names); `spec` keeps `SPC`; the field is renamed `body_type`.

## Answer

**M-11, decided (Mild: *"งั้นเอา content_type ละกัน"* / *"เห็นด้วยคับ"*, 2026-09-23):**

- **a — codes.** `docs/design/spec/` stays a **coded** collection, code `SPC`, files
  `spec/SPC-<n>.md`. `docs/design/catalog/` gets **no code** — path-identified:
  `catalog/rules.md`, `catalog/commands.md`, `catalog/exit-codes.md`,
  `catalog/frontmatter-losses.md`. Ticket 6/5's `CAT` code is withdrawn — Aria's proposal was
  right about the conflict.
- **b — fields, both larger than originally scoped:**
  - **Spec schema:** `title` (string, required), `status` (enum `draft|active|superseded`),
    `superseded_by` (ref → `SPC`, present only when superseded), `migrated_from` (string, the
    source location in the archived docs — e.g. which section of `docs/archived-design/`).
  - **Catalog schema:** `title` (required — catalog documents get a human title too, not just a
    body), `content_type` (enum `["json"]`, required — see c), `explained_by` (ref → `SPC`,
    written as a key like `SPC-4`, not a path — points at the prose spec entry that explains this
    catalog document).
- **c — field name.** Not Aria's proposed `body_type` — Mild's own choice is **`content_type`**.
  Replaces `body-type` everywhere in the contract and tickets.

Ticket 10 (schema/documents), ticket 11 (helper — dispatches on `content_type`, unchanged
logic), the contract (decisions 1-3), and the map all need updating to match — not done by this
ticket, done where each of those already lives.

**Amended 2026-09-24 by Mild's own follow-up:** `explained_by` is `ref[]`, not the single `ref`
recorded above, and optional rather than required. See ticket 10 and contract decision 2 for the
current, correct shape — this ticket's own record of the `ref`/required version stands as the
history of what M-11 actually said, not silently rewritten.
