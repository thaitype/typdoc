# 23: Catalog collection's code, and the body-type field's name (M-11)

Type: wayfinder:grilling
Status: open
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

<filled in on resolve>
