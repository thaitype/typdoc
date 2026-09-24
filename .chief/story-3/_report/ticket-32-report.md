# Ticket 32 Report

## Ticket

M-20: every printed document name must follow the design's one naming table (a coded document is
`key` when the project has exactly one namespace, `namespace:key` when it has several; an uncoded
document is always its path). Two directions of the same bug: `list`/`--ids` under-qualify in a
multi-namespace project; `refs`/`mv`'s `unrewritten:` over-qualify in a one-namespace project.

## Outcome

done

## The fix

One `bool` — "this project has more than one namespace" (`project.config().namespaces.len() > 1`)
— computed once wherever `Project` is already in scope at each call chain's entry point in
`crates/typdoc/src/cli.rs`, then threaded down:

- **Entry points** (`fn list`, `fn refs`, `fn mv`, `fn mv_renumber`) now return
  `Result<(ListResult, bool), Error>` / `Result<(RefsReport, bool), Error>` /
  `Result<(MvReport, bool), Error>` instead of just the report, carrying `multi_namespace`
  alongside it. The match arms in `run` unpack the tuple and pass the bool on (discarding it with
  `_` on the `--json` branches, which are unaffected — `--json` always carries `path`/`namespace`/
  `key` as separate fields, never a concatenated name).
- **Direction 1** (`table_row`, the `--ids` loop in `list_outcome`, `list_table`): both now go
  through a new shared helper, `fn identity_text(doc: &Document, multi_namespace: bool) -> String`
  — `{namespace}:{key}` for a coded document when `multi_namespace`, else the bare key, else the
  path. `list_table`'s header logic (`identity_label`: `key`/`path`/`document`) is untouched — it
  names what kind of value a column holds, not how a key happens to be spelled.
- **Direction 2** (`ref_name_text`, called by `ref_outcome_text`, called by `refs_text` and
  `unrewritten_text`): now takes `multi_namespace` too. When the project has several namespaces it
  still renders `namespace:key`; when it has exactly one it now renders the bare `key` instead of
  unconditionally prefixing `namespace:`. The `project::` prefix for an imported-project document
  is untouched (a separate, unrelated namespace count). Because `ref_name_text`/`ref_outcome_text`
  are shared, this one change fixes `refs` (forward and `--reverse`) and `mv`'s `unrewritten:` line
  at once.

Example, multi-namespace project (`story-1`, `story-2`, each with a document coded `WF-1`):

```
$ typdoc list --collection tickets --ids     # before: WF-1 / WF-1 (ambiguous)
story-1:WF-1                                 # after
story-2:WF-1
```

Example, single-namespace project (`default`, coded document `WF-1` referenced by `WF-2`):

```
$ typdoc refs WF-2 --reverse                 # before: default:WF-1
document  field                              # after
WF-1      blocked_by
```

## Verification

**Red then green**, both directions — proven by stashing only the `cli.rs` fix (keeping the new
tests) and re-running:

- Direction 1 (pre-fix, actually red):
  - `ids_qualifies_a_coded_documents_key_when_the_project_has_several_namespaces`: got
    `["WF-1", "WF-1"]`, expected `["story-1:WF-1", "story-2:WF-1"]`.
  - `list_table_qualifies_a_coded_documents_key_when_the_project_has_several_namespaces`: got
    `["WF-1", "WF-1", "WF-9"]` for the identity column.
  - `a_mixed_list_result_qualifies_only_the_coded_rows_in_a_multi_namespace_project`: coded rows
    printed bare (`WF-1`, `WF-1`, `WF-9`) instead of qualified.
  - `list_table_and_ids_stay_bare_when_the_project_has_exactly_one_namespace` correctly stayed
    green even pre-fix, confirming it as a genuine no-op regression guard, not a false positive.
- Direction 2 (pre-fix, actually red):
  - `refs_reverse_without_json_prints_the_holders_name_and_field_per_line`: got
    `default:WF-1`/`default:WF-3`, expected bare `WF-1`/`WF-3`.
  - `refs_field_without_json_keeps_only_that_fields_refs`: got `default:WF-2`, expected bare
    `WF-2`.
  - `unrewritten_names_a_coded_holder_by_its_bare_key_in_a_single_namespace_project` (new mv.rs
    test): got `default:WF-1  $body  ../old.md`, expected `WF-1  $body  ../old.md`.
- After restoring the fix (`git stash pop`, diffed byte-identical against the pre-stash file), all
  of the above went green, plus every other existing test.

**Existing goldens updated** (checked every `list`/`--ids`/`refs`/`mv` golden project-wide, not
just assumed): `crates/typdoc/tests/refs.rs`'s two single-namespace (`valid/refs`, one namespace,
`default`) text-mode goldens — `refs_reverse_without_json_prints_the_holders_name_and_field_per_line`
and `refs_field_without_json_keeps_only_that_fields_refs` — changed from `default:WF-N` to bare
`WF-N`. `valid/refs-worked-example` (two namespaces, `chief`/`team`) needed no change — its
`chief:WF-7` example is already correct under the new rule and stayed green throughout, confirming
the multi-namespace case wasn't regressed. `mv.rs`/`mv_renumber.rs`'s two existing `unrewritten:`
goldens use uncoded holders (printed by path either way) and needed no change; `mv_renumber.rs`'s
`project()` fixture (two namespaces) also stayed green unchanged, confirming multi-namespace
`mv`/`--renumber` output isn't regressed either.

**Gates**: `cargo fmt --check` clean; `cargo clippy --workspace --all-targets -- -D warnings`
clean; `TMPDIR=/home/thw-home/.cache/typdoc-tmp scripts/test.sh` → `1006 test(s) passed across 56
suite(s)`, exit 0, zero failures.

**Ticket 29's own header tests** (`the_header_row_names_path_for_a_path_identified_collection`,
`the_header_row_names_document_when_the_matched_set_mixes_coded_and_path_identified_documents`,
`the_default_table_shows_identity_title_and_every_field_used_in_where`) re-run in isolation:
all three still pass, confirming the header-label logic (`key`/`path`/`document`) is unaffected by
this ticket's change to the row *values*.

## Notes

- Reused the existing `fixtures/valid/several-namespaces` fixture for Direction 1 (two namespaces,
  `story-1`/`story-2`, each with a ticket coded `WF-1` — exactly this ticket's own repro shape)
  rather than adding a new fixture, since it already matched precisely.
- `identity_text` (Direction 1) and `ref_name_text` (Direction 2) both defensively fall back to
  the bare key if a coded document's `namespace` were ever `None` (shouldn't happen per
  `Document`'s own doc comment — coding requires a collection, which requires a namespace — but
  handled rather than assumed).
- No fixture reaches the `imported-project` reason yet (`ref_name_text`'s `project::` prefix,
  pre-existing gap noted by ticket 29/32 both), so that branch remains untested by construction,
  consistent with prior tickets in this story.
