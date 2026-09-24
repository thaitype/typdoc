# Ticket 12 Report

## Ticket

Rewire the three `design.rs` callers onto the catalog documents ticket 10 built, through ticket
11's `read_json_body` helper, then delete `design.rs`. Folded in by M-13 (ticket 24, resolved
2026-09-24, option 1: hand-list, no fifth catalog document): also delete
`typdoc-testkit/src/shell_examples.rs` and the extraction half of
`crates/typdoc/tests/shell_examples.rs`, and `fixtures.rs`'s `design_text()`/`design.md` root
marker.

## Outcome

done

## Verification

All four required `rg` checks return nothing outside git history:

```
rg 'typdoc_testkit::design\b'      -> (nothing)
rg 'design_text'                   -> (nothing)
rg '\bdesign\.rs\b'                -> (nothing)
rg 'typdoc_testkit::shell_examples' -> (nothing)
```

`docs/design/design.md` is untouched in this worktree (no diff, not moved) — left for ticket 13.

`cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and
`TMPDIR=/home/thw-home/.cache/typdoc-tmp scripts/test.sh` all pass (998 tests across 56 suites).
One unrelated flake hit once during a full run
(`lock_contention::n_processes_racing_one_lock_issue_n_distinct_keys_with_no_document_overwritten`,
a concurrency test sensitive to machine load from other parallel builds on this box) — reran
clean in isolation and clean again in a subsequent full `scripts/test.sh` run; not related to
this ticket's change (no lock/concurrency code touched).

## Drift-detection proof (all three rewired tests)

Each was planted once, confirmed red, then reverted and confirmed green again (no `git diff`
remained in `docs/design/catalog/` after each revert):

- **`typdoc-core/tests/rules.rs`** (`docs/design/catalog/rules.md`): added
  `{ "id": "made.up.drift.probe", "configurable": true }` to the list ->
  `every_rule_the_design_names_is_built_or_listed_and_nothing_listed_is_built` and
  `the_rules_a_config_may_name_are_split_by_the_catalogs_configurable_field` both failed
  ("the design names rule made.up.drift.probe, and it is in neither the registry nor
  unimplemented_rules"). Reverted -> both green.
- **`typdoc/tests/coverage.rs`**: removed `"validate"` from `commands.md`'s list ->
  `every_command_the_design_names_is_built_or_listed_and_nothing_listed_is_built` failed
  ("command validate is in the registry and the design does not name it"). Removed `7` from
  `exit-codes.md`'s list -> `every_exit_code_of_the_design_is_produced_or_listed_and_nothing_listed_is_produced`
  failed ("exit code 7 is in the codes a test produces and the design does not name it"). Both
  reverted -> both green.
- **`typdoc-core/src/frontmatter.rs`** (`docs/design/catalog/frontmatter-losses.md`): added
  `"A made-up shape no fixture demonstrates"` to the `losses` list ->
  `every_shape_the_design_names_as_lost_is_held_by_some_document_in_the_corpus` failed with
  `detector_for`'s own unrecognized-row error. Reverted -> green.

## A scope note on `rules.rs`'s coverage (not a defect introduced here, but worth recording)

The pre-ticket-12 test used `typdoc_testkit::design::rule_ids()`, which read *three* tables from
`design.md`'s "Validation rules" section (always-on, configurable, and a third table of
`config.*` config-error ids, headed `Id | Reported when`) and checked the union — 43 ids total —
against `typdoc_core::rules::RULES` (also 43: 14 `ALWAYS_ON` + 9 `CONFIGURABLE` + 20 `config.*`).

`docs/design/catalog/rules.md`, as ticket 10 built it under the contract's already-fixed shape
(contract decision 1: "one entry per rule id from both of today's two tables"), holds only the
two rule tables — 23 ids, no `config.*` entries. This is a decision from ticket 3/10, not one
this ticket made; the goal's own enumeration of the five sets `design.rs` used to extract
("always-on rule ids, configurable rule ids, command names, exit codes, ... frontmatter losses")
also does not name a "config errors" set as one of the four catalog documents' job to hold.

Given that, `rules.rs`'s rewired first test now compares the catalog's 23 ids against
`ALWAYS_ON ∪ CONFIGURABLE` (23) rather than the full `RULES` (43) — the only comparison the
catalog's actual content supports. This is documented in the test file's own doc comment and in
`validation_rule_ids()`'s doc comment. The practical effect: a `config.*` id added to or removed
from `RULES` with no corresponding catalog change no longer turns any test red (it did before
ticket 12, via `rule_ids()`'s inclusion of the third table). Nothing in ticket 12's own text asks
for a replacement mechanism for that specific coverage, and inventing one (e.g. a new hand-listed
config-error-id list somewhere) would be scope beyond what this ticket names. Flagging this
explicitly rather than letting it pass silently, per the build discipline of not quietly
narrowing a contract-adjacent guarantee — if this coverage is wanted back, it needs its own
ticket/decision, since `docs/design/catalog/rules.md`'s shape is already fixed by contract
decision 1 and is not this ticket's to change.

## Shell examples: which "safe" examples were hand-listed vs dropped

M-13 (ticket 24) resolved to option 1 (hand-list, no fifth catalog document). The pre-existing
`declared_examples()` (12 entries needing a hand-written value because of an unsafe character)
stays exactly as it was — those two tests
(`every_declared_example_matches_its_declared_value_quoted_in_every_listed_shell`,
`removing_the_quotes_from_a_declared_example_changes_what_the_shell_passes`) are untouched in
behavior.

The two tests that only existed to cross-check extraction against the declared list, or against
the quoting-paragraph span, were deleted outright (not adapted):
`every_unsafe_example_has_a_declared_value_and_every_declared_value_matches_an_example` and
`every_code_span_of_the_quoting_paragraph_is_classified`.

Before deleting `typdoc-testkit/src/shell_examples.rs`, I ran its `examples()` extractor once
(temporarily, then reverted) against the real `docs/design/design.md` to enumerate every "safe"
(no shell-unsafe character) example that had never been hand-declared — i.e., examples that used
to reach the harness only through automatic extraction, with their expected value computed by
splitting on whitespace. 33 such examples existed. Judgement call, per example:

**Hand-listed** (9, added to a new `additional_safe_examples()` list in
`crates/typdoc/tests/shell_examples.rs`, run through the existing
`safe_command_and_expected`/`run_in_shell` machinery via a new test
`every_hand_listed_safe_example_reaches_the_stand_in_as_its_own_words`) — every extracted example
that is a real, standalone, worked invocation with concrete arguments (drawn from the design's
"Worked examples" and "Common tasks" tables), because these demonstrate actual usage patterns and
are exactly what this harness exists to keep honest:

- `typdoc list --collection wayfinder,decisions --where status=open --sort status --sort updated_at:desc`
- `typdoc set WF-3 status=claimed owner=zeldia-7a2f --if status=open`
- `typdoc refs precedents/secret-handling.md --reverse`
- `typdoc mv WF-2 --renumber story-3`
- `typdoc set WF-5 blocked_by=WF-3,WF-4`
- `typdoc toc WF-3 --json`
- `typdoc list --where blocked_by=WF-3`
- `typdoc refs learnings/never-send-secrets-over-ship.md --reverse`
- `typdoc get story-2:WF-5`

**Dropped** (24), in two groups, with reasons:

- **23 bare mentions of a single flag or command name, with no arguments**: `typdoc`, `typdoc
  new`, `typdoc pull`, `--json`, `--audit`, `--dir`, `--renumber`, `--namespace`, `--set`,
  `--where`, `--collection`, `--if`, `--code`, `--limit`, `--ids`, `--sort`, `--reverse`,
  `--field`, `--check`, `--strict`, `--schemas`, `--lock-timeout`, `--depth`. Checked their
  context in `design.md`: each is a prose reference to a flag or command's *name* (e.g. "handled
  by `typdoc new`"), never a standalone invocation. A single word made only of letters, digits
  and hyphens has no shell-unsafe character by construction, so asserting it "reaches the stand-in
  as its own word" tests nothing a shell could ever get wrong — no quoting, splitting, or
  expansion behavior is exercised. Kept the list from growing by ~23 near-duplicate,
  zero-marginal-value entries.
- **1 already covered verbatim elsewhere**: `typdoc validate` (from the "Common tasks" table row
  "Broken frontmatter refs, body links and anchors"). `every_listed_shell_runs` (unchanged,
  pre-existing) already runs exactly this command, through every listed shell, asserting the same
  recorded args. Hand-listing it a second time in `additional_safe_examples()` would be pure
  duplication of an existing, still-running check, not a coverage gain.

The harness reads no markdown either way: `additional_safe_examples()` is a fixed, hand-written
`Vec<&'static str>`, and `crates/typdoc/tests/shell_examples.rs` no longer imports or calls
anything from `typdoc_testkit`.

## Standards-review follow-up applied before commit

A parallel `/chief-review-code` Standards pass flagged real duplication: the "join repo root,
`read_to_string`-or-panic, `read_json_body`-or-panic" shape was written out three times (two of
them in the same crate, `typdoc-core`). Fixed by adding
`typdoc_testkit::fixtures::read_catalog::<T>(relative)` and having all three rewired call sites
(`typdoc-core/tests/rules.rs`, `typdoc-core/src/frontmatter.rs`'s corpus test, and
`typdoc/tests/coverage.rs`) use it instead of their own copies. Also fixed stale "the design's
table of losses" / "a rewording of the table's prose" wording in `frontmatter.rs`'s
`detector_for` and its two callers' messages, left over from when the source was a markdown
table rather than a JSON `losses` array. Re-verified fmt/clippy/`scripts/test.sh` and all three
drift proofs after this refactor — unchanged results.

## Merge note (2026-09-24)

Built in a worktree branched before M-13/`explained_by` landed on the main checkout; the build
itself cherry-picked both onto its own branch before starting (documented in its own handback).
Merging required first bringing `story-3-catalog-and-release` up to include M-13/`explained_by`
via a direct cherry-pick of the same two commits (different hashes, identical content — cherry-
pick always mints new hashes), then merging ticket 12's branch cleanly on top (one trivial
conflict, in this ticket's own `Status:` line, resolved to `resolved`). Re-verified all three
gates after the merge: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D
warnings`, `scripts/test.sh` (998 tests, 56 suites, 0 failed).

**Flagging the scope-narrowing the build itself already caught, not treating it as silently
fine:** `rules.md`'s rewired test now checks 23 rule ids (the two Validation-rules tables
`design.rs` always covered) rather than the 43 ids the old `RULES` constant checked (which also
pulled in a third table of `config.*` config-error ids). This traces to contract decision 1's own
catalog shape (`rules.md` = "one entry per rule id from both of today's two tables"), not to
anything this ticket introduced — but it is a real, if pre-existing, narrowing of what the
"design.rs replacement" catches: a `config.*` id added to or removed from `RULES` no longer turns
any test red. Worth a decision on whether `config.*` ids need their own coverage (a fifth catalog
document, or folded into an existing one) — not decided or fixed here, since the shape itself was
already contract-decided before this ticket started.
