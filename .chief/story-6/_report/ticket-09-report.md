# Ticket 09 Report

## Ticket
Story close check: no code or comment changes. On `origin/main` at `9fe7708`, after every batch
has merged: search `crates/` for history citations, run the citation audit over the whole story,
and list every mismatch listed for a decision and every change to Rust that is not a comment.

## Outcome
done

## Search of `crates/` for history

```
grep -rnE '\b[Tt]icket [0-9]+|\bdecisions? [0-9]+|\bM-[0-9]+|\.chief/|docs/design\.md|migrating-design|archived-design' crates/
```

One match, and it is not a citation:

```
crates/typdoc/tests/get.rs:519:    for argument in [".chief/note.md", ".hidden.md", "..two/note.md"] {
```

`.chief/note.md` is a path in the test that checks a path with a leading dot; it names no
planning record.

Namespace names such as `story-1`, `story-2` and `story-3` appear throughout `crates/` (`grep -rnoE
'story[- ][0-9]+' crates/` finds them in 21 files). Every one is data: a namespace in a fixture, a
prefix in an example ref (`story-2:WF-5`, `chief::story-3:WF-5`), or a state file name
(`.typdoc/state/story-3.json`). None refers to a story of this repository's planning.

## Citation audit over the whole story

`citation-audit.py 648f01e^ 9fe7708 crates` (`648f01e^` is `33459a8`, the base of the first batch;
the two other pull requests merged in that range, #9 and #13, change nothing under `crates/`):

```
432 hunks with removed citations; 172 with an SPC in the added lines; 260 without
```

The per-batch citation tables, each matched row for row against the audit of its own pull
request, hold 90 (#10), 59 (#11), 75 (#12), 46 (#14), 70 (#15) and 78 (#16) rows, 418 in all; the
pilot (#8) predates the one-row-per-hunk table and lists its 14 citations one per citation. A hunk
of the whole-story diff is not always a hunk of one pull request's diff, since a file changed in
two pull requests (`json_body.rs`, in #8 and #10) can merge differently, so the two counts are not
claimed to map one to one. Every row gives one of: an SPC cited in the comment; an SPC cited
through the test file's `//! Covers` line; a comment removed entirely whose design is held by a
named SPC; process only (a ticket, a record, a date), dropped; a deferred area whose design stays
in `docs/migrating-design/`; a string or rename commit; or a match that is not a citation.

## Mismatches listed for a decision, by pull request

| # | Pull request | Mismatch | Decision |
|---|---|---|---|
| 1 | #8 | `leftovers.rs`: said nothing ships that leaves a leftover by being killed mid-write | Fixed in #8 (comment corrected) |
| 2 | #8 | `staging.rs`: said no command a fixture declares writes yet (two places) | Fixed in #8 (comment corrected) |
| 3 | #8 | `state_write.rs`: said the setup file has one space of indentation | Fixed in #8 (comment corrected) |
| 4 | #8 | `golden.rs`: gave the regenerate command without the memory ceiling | Fixed in #8 (comment corrected) |
| 5 | #8 | SPC-10: said `lock.json` and `vendor/` are written, which nothing does today | Fixed in #8 (they are, once `pull` writes them) |
| 6 | #8 | SPC-10: gave the temp file name without the in-process count | Fixed in #8 (name as the code builds it) |
| 7 | #8 | `json_body.rs`: a test named as if a missing block were its own case, asserting `MissingContentType` | Fixed in #10 (test renamed) |
| 8 | #10 | Three `AlreadyExists` messages a user sees ended in a decision number | Fixed in #10 (number dropped) |
| 9 | #10 | An `#[allow]` reason described `body.mentions` as not built | Fixed in #10 (reason updated) |
| 10 | #10 | `mv` does not check the `git-common` lock mode, which `set` and `new` refuse | Deferred by decision |
| 11 | #10 | Reverse `refs`, `refby` and `mv` scan this project only; the design includes imported projects | Deferred by decision |
| 12 | #10 | Query scope does not reach imported projects | Deferred by decision |
| 13 | #10 | `body.mentions` does not use imported projects' codes or prefixes | Deferred by decision |
| 14 | #10 | A file inside a namespace folder that no collection matches takes the loose lock | Deferred by decision |
| 15 | #11 | The design says a lock taken away is reported; the release result is discarded | Deferred by decision |
| 16 | #11 | `pid_alive` has no `/proc` on macOS, so a live lock owner reads as stale | Deferred by decision (fix right after the story) |
| 17 | #11 | A test assertion message named a decision number | Fixed in #11 (number dropped) |
| 18 | #12 | A body link under `refBase: namespace` resolves against the namespace folder; the design says the document | Deferred by decision (bug fix after the story) |
| 19 | #12 | `mv` neither rewrites nor reports reference-style links and their definitions | Deferred by decision (bug fix after the story) |
| 20 | #12 | A relative `extends` in a pinned schema resolves against the project folder, not its URL | Deferred by decision |
| 21 | #12 | SPC-4 said a `frontmatter.parse` finding has a position when the reader gives one | Fixed in #12 (SPC-4 corrected) |
| 22 | #12 | `config.state-orphan` stops the command with exit 2; SPC-8 and SPC-6 say it stops nothing | Deferred by decision (bug fix after the story; the code is to change) |
| 23 | #12 | The text audit prints no finding lines; the design's audit table says it does | Deferred by decision |
| 24 | #12 | Machine-file errors do not name the lookup step they came from | Deferred by decision |
| 25 | #12 | `refs.rs`: a test named as if the import form were not read | Fixed in #12 (test renamed) |
| 26 | #14 | `refs` prints a document of an imported project under a name `get` does not accept | Deferred by decision (bug fix after the story, with the work on imported projects) |
| 27 | #14 | A schema's `code` is not checked against its pattern | Deferred by decision (bug fix after the story) |
| 28 | #14 | SPC-2 said `mv --renumber` prints only the new key | Fixed in #14 (SPC-2 corrected) |
| 29 | #14 | Every lock records the host name `unknown-host` on macOS; the design names Linux alone | Deferred by decision (folded into the fix of 16) |
| 30 | #15 | `state.rs`: a test named after a decision number | Fixed in #15 (test renamed) |
| 31 | #16 | Four test names said how the behavior changed ("no longer", "now", "as it did") | Fixed in #16 (tests renamed) |

Nothing is left waiting for a decision. Rows 1 to 6 were first listed in #8 and then, under the
rule that a comment the code clearly contradicts is corrected, fixed in the same pull request.
Beyond these, each pull request's "Corrected comments" section lists the comments corrected to
the code without being listed first.

Not a mismatch and not decided in this story: the rule id `files.leftover`, which `validate`
reports, has no entry in `docs/design/catalog/rules.md` (seen in #8, outside its scope).

## Rust changes that are not comments

Each in its own commit, listed in the contract, and the only differences the comment-only proof
reports for its pull request.

| Pull request | Change |
|---|---|
| #10 | `crates/typdoc-core/src/json_body.rs`: test renamed to `a_document_with_no_frontmatter_block_at_all_is_reported_as_missing_content_type` |
| #10 | `crates/typdoc-core/src/project.rs`: three `AlreadyExists` messages drop a decision number; one `#[allow]` reason about `body.mentions` updated |
| #11 | `crates/typdoc-core/src/namespace_lock.rs`: one test assertion message drops a decision number |
| #12 | `crates/typdoc-core/src/refs.rs` (assertion message), `namespaces.rs` (assertion message), `tests/common/mod.rs` (fixed host name): references to history dropped |
| #12 | `crates/typdoc-core/src/refs.rs`: test renamed to `a_body_link_double_colon_naming_no_configured_import_is_bad_prefix` |
| #14 | `crates/typdoc/src/registry.rs`: the `[reverse-scope]` entry of `KNOWN_GAPS` no longer points at the design |
| #15 | `crates/typdoc/tests/mv.rs` (assertion message) and `templates.rs` (`ignore` reason): a decision number and a CI date dropped |
| #15 | `crates/typdoc/tests/state.rs`: test renamed to `deriving_last_reissues_a_retired_key_silently` |
| #16 | Six `ignore` reasons in `config.rs`, `get.rs`, `namespaces.rs` and `validate.rs` drop a CI date; the panic message in `shell_examples.rs` names SPC-13 instead of a `design.md` section |
| #16 | Four tests renamed: `list_is_not_in_the_list_of_commands_the_binary_lacks`, `refs_target_refuses_a_ref_that_crosses_an_import_to_a_schema_the_qualified_target_excludes`, `refs_coded_by_path_warns_for_a_coded_document_of_an_import_referenced_by_path`, `a_bare_field_in_the_fixtures_is_only_an_unknown_field` |

No behavior changed: every test passed before and after every batch (1057 passed, 0 failed,
1 ignored).

## Notes
- The public-text check reports no match under `crates/`.
- The story is accepted outside the loop.
