# 35: M-22 — `mv` must list plain-text mentions of the moved key in `unrewritten`

Type: implementation
Status: claimed
Blocked by: None (can start immediately)

**Mild's decision (M-22), 2026-09-24, relayed by Aria, found running a real release build.**
Predates story 3's own text-output work — the design already documents `unrewritten`'s
`mention` reason (§JSON output, `mv`) — this is a bug fix (a real code path never built), not a
new decision.

**Repro, on `09c7b7b`:** `body.mentions` at warn (or any enabled level); a document's body says
plain text `WF-3` (no formal ref field, no markdown link — just the bare key mentioned in
prose); `mv story-2:WF-3 --renumber story-1` reports `unrewritten: []` — it says nothing about
that mention at all. Afterward, `validate` correctly reports `body.mentions WF-3 not found`,
because the key really did stop resolving — `mv` should have surfaced this as an
`unrewritten`/`mention` entry at move time, not left it to be discovered by a later `validate`.

## Why this was never built

`UnrewrittenReason::Mention` already exists as an enum variant
(`crates/typdoc-core/src/mv.rs`, search `enum UnrewrittenReason`) but is never constructed
anywhere — it's a placeholder for exactly this feature. `Project::mv_reverse_scan`
(`crates/typdoc-core/src/project.rs`, search `fn mv_reverse_scan`) only calls `self.refs(from,
scope, true, None, deps.env)?` — a reverse scan of **formal** references (frontmatter `ref`/
`ref[]` fields and markdown body links), which never looks at plain body prose at all. A
mention (`crates/typdoc-core/src/links.rs`'s `links::mentions`, used today only by
`validate`'s `body.mentions` check, around `crates/typdoc-core/src/project.rs` line ~3095-3143 —
read that block first, it's your reference for exactly how a mention is found and matched) is a
bare key-shaped token in ordinary prose, with no ref/link syntax around it at all — nothing a
formal-refs reverse scan could ever find.

## The fix

Extend `mv_reverse_scan` (or add a parallel scan alongside it, called from the same place) to
also find every plain-text mention, across every document in scope, of the **key currently
belonging to `from`** (mentions are always key-shaped — `links::mentions`'s own doc comment says
so — so this only applies when `from` resolves to a coded document; skip entirely for a
path-identified move, same as the design's own reasoning for why this case was originally
believed unreachable).

For each document in the project (`Project::incoming_refs`, search `fn incoming_refs`, already
reads every document's file text once for the reverse-refs scan machinery — either extend it to
also return mentions matching a target key, avoiding a second full read of every file, or add a
clearly-justified second pass if that's cleaner; your judgment, but don't silently duplicate a
whole-project disk read without at least considering reusing the existing one), parse its body
with `links::mentions(text, inline_code, fenced_code)` (same `inlineCode`/`fencedCode` options
`body.mentions`'s own check reads via `rule_options`/`bool_option` — reuse that exact pattern, per
the holder's own collection's validation config) and check whether any mention's `.written`,
after stripping a `namespace:` prefix the same way `mention_missing` does (`written.rsplit(':')`),
resolves to `from`'s current key. Do NOT reuse `mention_missing` itself — it checks whether a key
is *currently missing from the index*, which is the wrong question before the move has happened
(the key still resolves at scan time); what you need is a direct comparison against `from`'s own
key.

For every matching mention found, push an `UnrewrittenRef` into `mv_reverse_scan`'s existing
`unrewritten` vector: `reference: RefsReference { other: Resolved(<the holder's own RefName>),
field: "$body".to_owned(), written: mention.written.clone(), position: Some(Position { line:
mention.line, col: mention.col }) }`, `reason: UnrewrittenReason::Mention` — the same shape
`LinksRuleOff` entries already use right above in the same function, just a different `reason`
and no formal `reference` to have found it through (build the `RefsReference` directly here,
since a mention was never a `RefsReference` `self.refs()` itself produced).

Do not touch `rewrite_holder`/`mv_rewrite_changes` — a plain-text mention is never rewritten (the
design's own `unrewritten` concept exists exactly because these can't be safely rewritten
automatically), only reported.

## Tests

1. **Red first** (Mild's instruction): the exact repro (a document with a plain-text body mention
   of the key about to be renumbered/moved) — confirm `unrewritten` is empty today (the bug),
   before making any change.
2. After the fix: the same repro's `unrewritten` includes one entry with `reason: mention`,
   `field: "$body"`, the mention's own written text, and the correct line/col position.
3. Both `mv` (plain, not just `--renumber`) and `mv --renumber` — the design's own wording covers
   both `mv` forms (`unrewritten`/`findings` are "always present" for both, per M-10e), and a
   plain `mv` that changes a document's PATH (not its key) can still break a mention if the
   mention was written as a path-form token, if that's a form `links::mentions` even recognizes —
   check `links::mentions`'s own doc comment/implementation to confirm whether it only recognizes
   key-shaped tokens (in which case plain `mv`, which never changes a key, genuinely has nothing
   to report here — confirm this rather than assuming it, and if so, say so plainly in your report
   rather than silently skipping the case).
4. A negative case: a document mentioning a DIFFERENT key (not the one being moved) is not
   reported.
5. A negative case: a formal ref/link to the moved key is still rewritten normally (unaffected by
   this ticket — only plain-text mentions are new).
6. Confirm a subsequent `validate` run after the move still correctly reports `body.mentions ...
   not found` for the same mention `mv` now also reports proactively — both should agree (belt and
   suspenders is fine; they're two different code paths that should reach the same conclusion).
7. Full gate run: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
   `TMPDIR=/home/thw-home/.cache/typdoc-tmp scripts/test.sh`.

## Docs

This changes `mv`'s own `unrewritten` output whenever a mention exists — a real user-visible
change. Update `docs/reference/commands.md` and `skills/typdoc/references/commands.md` (ticket
33's new doc locations — `docs/commands.md`/`docs/projects.md` no longer exist) wherever `mv`'s
`unrewritten` shape or reasons are documented, to include `mention` alongside whatever reasons
are already listed there (`imported-project`, `mention`, `links-rule-off` — confirm the exact set
and wording already used, match its style). Update `CHANGELOG.md`'s existing `mv` bullet under
`[0.2.0]` (still Unreleased) in place if it already describes `unrewritten`'s reasons, rather than
adding a separate contradictory line.

## Done

- `mv` and `mv --renumber` both list a plain-text mention of the moved key in `unrewritten`, with
  `reason: mention`, at move time — not left for a later `validate` to discover alone.
- Formal refs/links are unaffected, still rewritten normally.
- A mention of an unrelated key is never reported.
- Docs/CHANGELOG reflect the corrected shape.
- All three gates green.
