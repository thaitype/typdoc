# Ticket 24 Report

## Ticket
Make `validate --audit` agree with the design's paragraphs **Loading** and **Audit**: `audit.collections`
holds one entry for every collection of the project, including one that ends with no document, and
`audit.overlapping` is a list of `{ "path", "collections" }` naming the collections that match each
file. The text form shows the same.

## Outcome
done. Before the change, on two files `a.md` and `b.md` with a collection `notes` matching `*.md` and a
collection `skills` matching `a.md`, the audit listed `notes` with 1, left `skills` out, and gave
`overlapping` as `["a.md"]`. Now it lists `notes` 1 and `skills` 0, and `overlapping` is
`[{ "path": "a.md", "collections": ["notes", "skills"] }]`. The text form prints a line for each
collection, `skills  0 files   clean` among them, counts every collection in its header, and writes
each overlapping path followed by its collections, `a.md (notes, skills)`.

Nothing else moved: `collections.overlap` is still an error and no rule picks a winner;
`summary.checked.documents`, `summary.unreported.*`, `summary.overlapping` and the `documents` of every
collection that was listed before keep their values; a file matched twice is in the number of no
collection.

Suite: 695 passed, 0 failed, 1 ignored (688 before, plus seven new tests). `cargo fmt --check`, clippy with
`-D warnings` and the public-text gate are clean.

## Decision if any

- **What a collection is across namespaces.** Decided: a collection is one file in
  `.typdoc/collections/`, and its `documents` is the sum over every namespace the audit reports. A
  collection is listed in every audit, whatever `--namespace` narrows it to; a namespace that is out of
  scope contributes nothing to the number, so a collection whose files are all in another namespace
  shows 0. The code already read the collections as one list shared by all namespaces (one `Loaded` per
  file, one `Member` per file, the index naming a collection by its position in that list), so this
  adds no second reading. Doubt: the design says "each collection of the project" and does not say
  whether a narrowed run lists the collections of the namespaces it left out; the reading here is that
  the project's collections do not depend on the scope, so a reader can compare two runs.
- **Text form of an overlap.** Default: `path (name, name)` for each file, separated by commas, then the
  count of files, on the existing line `matched by more than one collection: ...`. The design gives the
  text form no example of this line, so the layout is this ticket's own.
- **Sorting in the audit builder.** The names in an overlap and the list by path are sorted in the audit
  builder although the index already yields them in that order (collections load sorted by name and
  the index walks paths in order). The sort is kept so the order the design promises does not depend on
  a property of another module; no test can tell the sorted output from the unsorted one today, and
  none is written that pretends to.

## Notes

- **Seams.** All new tests run the built binary through the spawn helper, at `validate --audit`
  (`--json` and text), because the audit object and its text are what a person or a program sees. Cases:
  the shape of the ticket (`skills` with 0, named collections); a collection that matches nothing, with
  names sorting before and after the one that holds a document; a file matched by three collections
  beside one matched by two, which distinguishes carrying every name and the right names per file from
  carrying the first two or another file's; a file matched twice in no collection's number, with
  one file of each other kind (checked, no frontmatter, uncollected) so the summary numbers are pinned as
  well (a collection counting the overlap would read `notes` 3 and `skills` 1, not 2 and 0); the text
  form, for one overlap and for several; two namespaces, with and without `--namespace`.
- **Consumers of the audit object, each walked.** The invariant test
  (`the_accounting_invariant_holds_on_every_fixture_project`): its independent count of `.md` files reads
  only `summary` numbers and is unchanged; it now also reads the new shape of `audit.overlapping` (length
  equals `summary.overlapping`, every `path` a file on disk, at least two collection names) and checks
  that the collections' numbers add up to `checked.documents` plus `unreported.no_frontmatter`, which
  is the statement that a file matched twice is in no number; `broken/collections.overlap` is the fixture
  that gives it a non-empty list. The golden files: no case under `fixtures/output/` runs `--audit`
  (`validate` has `clean`, `one-document`, `one-finding` and `schemas-only`), so none holds
  `overlapping` or `collections` and none was regenerated; the regenerator and its guard are untouched.
  The shell examples: the only audit line is the illustrative `typdoc audit: 3 collections, ...` output,
  which is classified, not run, and the design is not edited. The text form in `crates/typdoc/src/cli.rs`
  (`audit_text`, `audit_json`) changed as above. The two existing tests that read an overlap
  (`audit_counts_an_overlapping_file_beside_unreported_not_inside_it` and the text-form one) were moved to
  the new shape by hand. In the JSON test no number changed. The text test gained the `two  0 files   clean`
  line and its header moved from 1 collections to 2, the one number that moved, because `two` matches only
  a file `one` also matches and is now listed. The builder in `project.rs` (`audit_report` and the overlap
  scan in `validate`) changed as above. `docs/getting-started.md` shows a one-collection audit with
  documents in it, whose output is the same; `docs/commands.md` says the audit lists files in no
  collection and files with no frontmatter, which stays true and is not extended here.
- **Guard.** A `std::fs::write` planted in `audit_report` failed clippy with the error "use of a
  disallowed method `std::fs::write`" and the note "the read core changes no file", and was removed.
- **Mutations.** Each was made on a copy-backed file and restored: listing only collections that hold a
  document (7 tests red); leaving the collections of an overlap out (6 red); counting an overlapping file
  in each of its collections' numbers (6 red); the text form leaving out the names (2 red; the several-overlaps case was added after this run) and counting
  only collections with documents in its header (2 red). Two survive: removing the sort of the names in
  an overlap and of the list by path, and removing the sort of collections by name. Each is redundant
  with an order the load and the index already give, as said under Decision, so no test reaches a branch
  where it matters.
