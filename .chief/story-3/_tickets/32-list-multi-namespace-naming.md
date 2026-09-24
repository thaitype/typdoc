# 32: M-20 — multi-namespace `list` text and `--ids` must print `namespace:key`

Type: implementation
Status: claimed
Blocked by: None (can start immediately)

**Mild's decision (M-20), 2026-09-24, relayed by Aria, found running a real release build.**
Predates story 3 — the design's own naming table already states the right behavior — this is a
bug fix, not a new decision.

**Repro:** a project with two namespaces, `story-1` and `story-2`, each containing a document
coded `WF-1`. `list` (text) and `list --ids` both print the bare key `WF-1` **twice** — a name no
other command can actually use to pick one of them: passing that same bare `WF-1` back to another
command exits 1 as ambiguous (multiple keys named `WF-1` exist project-wide). The design's own
naming table (search `docs/migrating-design/design.md` or `docs/archived-design/design.md` for
the table with rows "In this project" / "In an imported project" — around line 133 in the
archived copy) is explicit: a coded document's name is `path`, or `key` **only when the project
has exactly one namespace**, or `namespace:key` **when it has several**. `list` must print a name
every other command actually accepts, and today it doesn't, in a multi-namespace project.

## Where this lives today (the bug)

`crates/typdoc/src/cli.rs`:
- `fn table_row` (search for it — builds one `list` row): the identity cell is
  `doc.key.clone().unwrap_or_else(|| doc.path.clone())` — always the bare key for a coded
  document, with no namespace-count awareness at all.
- `list --ids`' own loop (inside `fn list_outcome`, search for the `ids` branch): same pattern,
  `doc.key.as_deref().unwrap_or(doc.path.as_str())`.

Neither of these has access to *how many namespaces the whole project has* — only to the
individual `Document`s matched by this particular query (`Document.namespace: Option<String>` is
the name of the one namespace THIS document is in, not a count of the project's namespaces).
Whether to print bare `key` or `namespace:key` is a **project-level** fact (design: "when it has
several"), not a per-query one — even a `list` result that happens to match documents from only
one namespace must still use `namespace:key` if the *project* has more than one, since a later,
unrelated `set`/`get` of that same bare key could still be ambiguous project-wide.

## The fix

Thread "does this project have more than one namespace" from wherever `Project` (or its
`config.namespaces`) is in scope down to `table_row` and the `--ids` branch — both live in
`crates/typdoc/src/cli.rs`'s `fn list`/`fn list_outcome` call chain (read `fn list`, `fn
list_outcome`, `fn table_row`, `fn cell_value` in full first to see the cleanest place to add this
— probably a `bool` parameter threaded alongside `matched`/`columns`, decided once from
`project.config.namespaces.len() > 1` at the point `Project` is still in scope, the same shape
`identity_label`'s own header-naming logic in `list_table` already threads a decision through).

When true (project has several namespaces) and a row's document has a key: print
`{namespace}:{key}` instead of the bare key. A document's `namespace` field can be `None` (a file
outside every namespace folder) — that case has no key at all (namespace-outside documents are
never coded, since coding requires a collection, which requires a namespace — confirm this
invariant holds by checking `Document`'s own doc comments / `document.rs`, rather than assuming
it, and handle it defensively either way) so it already falls through to the `path` branch
unaffected.

Apply this in BOTH places that print a document's identity as its "key or path" form for `list`:
`table_row` (the default table) and the `--ids` loop. Do NOT change `get`, `refs`, `mv`, or any
other command's own identity rendering (`document_name`, `ref_name_text`, etc.) — those already
use the full name shape (`namespace`/`key`/`project` as separate JSON fields, or `ref_name_text`'s
already-correct `namespace:key` text rendering for refs) and are out of this ticket's scope; this
bug is specifically in `list`'s two "just the key" shortcuts.

Also check `list_table`'s header (`identity_label`, ticket 28/29's own logic) still reads
correctly once this changes: the header says `key` (or `document` for a mixed coded/uncoded
result, per ticket 29) regardless of whether the printed value is a bare key or a
`namespace:key` form — a `namespace:key` value is still fundamentally "the key," just fully
qualified, so no header change is needed here; confirm this reasoning holds rather than assuming
it, by checking the resulting header against ticket 29's own three-case logic once this lands.

## Tests

1. **Red first** (Mild's instruction): a test with two namespaces, each holding a document coded
   `WF-1`, running `list` (text) and `list --ids`, asserting the CURRENT (wrong) bare-`WF-1`-twice
   output, confirming it's what happens today — then flip the assertion to the correct
   `namespace:key` form and confirm it fails before any fix.
2. After the fix: the same two-namespace fixture's `list` text and `--ids` output both show
   `story-1:WF-1` and `story-2:WF-1` (or whatever this project's actual namespace names render
   to — match the exact separator/format `ref_name_text`'s existing `namespace:key` rendering
   already uses, for consistency).
3. A single-namespace project's `list`/`--ids` output is UNCHANGED — still the bare key. This is
   the regression case: don't let the fix apply `namespace:key` unconditionally.
4. A mixed result (some coded docs across two namespaces, some uncoded documents by path) in a
   multi-namespace project: coded rows show `namespace:key`, uncoded rows show their bare path,
   unaffected.
5. Confirm every existing `list`/`--ids` golden test (virtually all of which use single-namespace
   fixtures) is unchanged.
6. `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
   `TMPDIR=/home/thw-home/.cache/typdoc-tmp scripts/test.sh`.

## Done

- In a project with more than one namespace, `list` (text) and `list --ids` print `namespace:key`
  for every coded document, a name another command can actually resolve unambiguously.
- A single-namespace project's output is unchanged (bare key).
- Uncoded documents are unaffected in either case (always their path).
- All three gates green.
