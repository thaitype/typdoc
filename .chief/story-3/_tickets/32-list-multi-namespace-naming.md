# 32: M-20 — every printed document name must follow the design's one naming table

Type: implementation
Status: resolved
Blocked by: None (can start immediately)

**Mild's decision (M-20), 2026-09-24, relayed by Aria, found running a real release build.**
**Scope expanded by Aria, same day, after re-verifying ticket 29**: this is not just a `list` bug
— it's the SAME naming table applied wrongly from both directions, in different commands. Fix
every place a document's printed name is built, not just `list`/`--ids`.

The design's own naming table (search `docs/migrating-design/design.md` or
`docs/archived-design/design.md` for the table with rows "In this project" / "In an imported
project" — around line 133 in the archived copy) is explicit: a coded document's name is `key`
**only when the project has exactly one namespace**, `namespace:key` **when it has several**; an
uncoded document is always its `path`, unaffected by namespace count either way. Exactly one rule,
two directions it's currently violated:

## Direction 1 — `list`/`--ids` under-qualify in a multi-namespace project

**Repro:** a project with two namespaces, `story-1` and `story-2`, each containing a document
coded `WF-1`. `list` (text) and `list --ids` both print the bare key `WF-1` **twice** — a name no
other command can actually use to pick one of them: passing that same bare `WF-1` back to another
command exits 1 as ambiguous (multiple keys named `WF-1` exist project-wide). `list` must print a
name every other command actually accepts, and today it doesn't, in a multi-namespace project.

## Direction 2 — `refs` (and `mv`'s `unrewritten:`) over-qualify in a ONE-namespace project

**Aria's own finding, verifying ticket 29:** in a project with exactly **one** namespace, text
`refs` prints `default:WF-2` — fully qualified even though the project only has one namespace, so
the naming table says this should be the bare key `WF-2`. It happens to still work as an
*argument* (`get default:WF-2` resolves fine — a fully-qualified name is always accepted, even
when a bare one would do), which is presumably why this went unnoticed, but it's still the wrong
name per the table: qualification should track whether the project genuinely *needs* it (several
namespaces), not be unconditional.

## Where this lives today (the bug, both directions)

`crates/typdoc/src/cli.rs`:
- `fn table_row` (builds one `list` row): the identity cell is
  `doc.key.clone().unwrap_or_else(|| doc.path.clone())` — always the bare key for a coded
  document, with zero namespace-count awareness (Direction 1's bug).
- `list --ids`'s own loop (inside `fn list_outcome`): the same pattern,
  `doc.key.as_deref().unwrap_or(doc.path.as_str())` (Direction 1's bug, same shape).
- `fn ref_name_text` (renders a `RefName`, used by both `refs_text`'s `ref_outcome_text` helper
  — forward and reverse — and by `mv`'s `unrewritten_text`, via that same shared helper ticket 29
  just factored out): **always** renders `namespace:key` for any coded document, with zero
  namespace-count awareness either (Direction 2's bug — the opposite mistake: unconditional
  qualification instead of unconditional bareness).

None of these three sites has access to *how many namespaces the whole project has* — only to
individual `Document`/`RefName` values, each of which knows its OWN namespace, not the project's
total count. Whether to qualify is a **project-level** fact (design: "when it has several"), not
a per-row or per-reference one.

## The fix

Thread one `bool` — "this project has more than one namespace"
(`project.config.namespaces.len() > 1`, decided once, wherever `Project` is in scope at each call
chain's own entry point) — down to all three sites, and make each one apply the SAME rule:

- **`table_row`/`--ids`** (Direction 1): when the project has several namespaces and a document
  has a key, print `{namespace}:{key}` instead of the bare key (this direction was ticket 32's
  original scope — implement it as already speced). When the project has exactly one namespace,
  keep printing the bare key (already correct today — don't touch this case).
- **`ref_name_text`** (Direction 2): when the project has exactly one namespace, print the bare
  `key` (dropping the `namespace:` prefix it unconditionally adds today). When the project has
  several, keep the existing `namespace:key` rendering (already correct today for that case —
  don't touch it).

`ref_name_text` currently takes only `&RefName` — it has no project in scope at all where it's
called (`ref_outcome_text`, called from `refs_text` and `unrewritten_text`). Trace both call
chains up to wherever a `Project`/its namespace count is available (`refs_outcome` for `refs`;
whatever builds `mv`'s outcome for `unrewritten_text`) and thread the same bool through, the same
way you thread it for `table_row`/`--ids`. Since `ref_name_text` and `ref_outcome_text` are
SHARED between `refs` and `mv`, fixing the shared function fixes both callers at once — that's
correct and intended, not scope creep (Aria: "make every printed name follow that one table —
list, --ids and refs alike").

A document's `namespace` field can be `None` (a file outside every namespace folder) — that case
has no key at all (namespace-outside documents are never coded, since coding requires a
collection, which requires a namespace — confirm this invariant by checking `Document`'s/
`RefName`'s own doc comments rather than assuming it, and handle it defensively either way), so it
already falls through to the `path` branch in every site, unaffected by this ticket.

The `project::` prefix `ref_name_text` adds for an imported-project document is a SEPARATE,
unrelated concern (a different namespace-count entirely — the imported project's own, not this
one's) — do not conflate the two; only the `namespace:key` vs. bare-`key` part of `ref_name_text`
changes, the `project::` prefix logic is untouched.

Check `list_table`'s header (`identity_label`, ticket 28/29's three-case logic: `key` /
`document` / `path`) still reads correctly once Direction 1 lands: the header still says `key`
(or `document` for a mixed result) regardless of whether the printed value is a bare key or a
`namespace:key` form — confirm this by re-running ticket 29's own header tests, don't just assume
it.

## Tests

1. **Red first** (Mild's instruction), both directions:
   - Direction 1: a two-namespace fixture (`story-1`/`story-2`, each with a document coded
     `WF-1`), asserting the CURRENT (wrong) bare-`WF-1`-twice output for `list` and `--ids`,
     confirming it's what happens today — then flip to the correct `namespace:key` assertion and
     confirm it fails before any fix.
   - Direction 2: a one-namespace fixture with a coded document, asserting the CURRENT (wrong)
     `namespace:key`-qualified output for text `refs` (and, separately, for `mv`'s
     `unrewritten:` line if an existing fixture makes this cheap to add), confirming it's what
     happens today — then flip to the correct bare-`key` assertion and confirm it fails.
2. After the fix:
   - Direction 1: the two-namespace fixture's `list` text and `--ids` output both show
     `story-1:WF-1` and `story-2:WF-1` (match `ref_name_text`'s existing `namespace:key`
     separator/format for consistency).
   - Direction 2: the one-namespace fixture's `refs` (forward and reverse) and `mv`'s
     `unrewritten:` output all show the bare key, no `namespace:` prefix.
   - A multi-namespace project's `refs`/`mv` output is UNCHANGED — still `namespace:key` (don't
     regress the case that was already correct).
   - A single-namespace project's `list`/`--ids` output is UNCHANGED — still the bare key (don't
     regress this either).
3. A mixed `list` result (some coded docs across two namespaces, some uncoded documents by path)
   in a multi-namespace project: coded rows show `namespace:key`, uncoded rows show their bare
   path, unaffected.
4. Confirm every existing `list`/`--ids`/`refs`/`mv` golden test (virtually all of which use
   single-namespace fixtures, so most `refs`/`mv` goldens should CHANGE from qualified to bare —
   check which ones actually need updating rather than assuming none do) reflects the corrected
   behavior.
5. `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
   `TMPDIR=/home/thw-home/.cache/typdoc-tmp scripts/test.sh`.

## Docs

Update `docs/commands.md` and `docs/design/spec/SPC-5.md` (or wherever `refs`'s and `list`'s
worked examples currently show a `namespace:key`-or-bare-key form) so every worked example matches
the corrected, namespace-count-aware rule. Update `CHANGELOG.md` if `refs`'s or `mv`'s output
shape is already mentioned there from an earlier ticket (correct in place, per this story's own
established convention — don't add a duplicate/contradictory bullet for the same unreleased
version).

## Done

- In a project with more than one namespace, `list`, `--ids`, `refs`, and `mv`'s `unrewritten:`
  all print `namespace:key` for a coded document — a name another command can actually resolve
  unambiguously.
- In a project with exactly one namespace, all four print the bare `key` — no unconditional
  qualification.
- Uncoded documents are unaffected in every case (always their path).
- All three gates green.
