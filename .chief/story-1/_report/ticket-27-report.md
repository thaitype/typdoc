# Ticket 27 Report

## Ticket
A ref or a body link that reaches an existing file outside every namespace folder
(`up: ../README.md` in a project with `namespaces` set) stopped `refs`, `list --where
'ref.any(f)'`, `ref.all(f)` and `refby.any(f)` with exit 101 and a panic in `ref_name_in`, which
expected a namespace with an empty folder that only the default single namespace has. The same
happened through an import, when a project pointed a ref at the root file of an imported project
with several namespaces. `validate`, `get` and `toc` did not stop.

## Outcome
done. Such a ref resolves like any other and the command goes on. The file is named by its path
alone: `refs` prints it with no `namespace` field, and a `ref.*` or `refby.*` condition treats it
as a document with no namespace.

- The scenario, run before the change on the unmodified tree: two namespaces `story-1` and
  `story-2`, a collection `*.md`, a `README.md` at the project root and `story-1/a.md` with
  `up: ../README.md`. `refs story-1/a.md --json` exited 101 with the panic at `ref_name_in`, and so
  did `refs --reverse`, `list` with `ref.any(up)`, `ref.all(up).path=...`, `refby.any(up)` and
  `ref.any($body)`, and both `refs notes/p.md` and `list --where 'ref.any(up)'` through an import.
  After the change every one of them exits 0 with nothing on stderr.
- `RefName.namespace` is `Option<String>`, and so is `Document.namespace`. `ref_name_in` no longer
  has an `expect`: it gives `None` when no folder matches and no namespace has an empty folder.
  Nothing else changed for a file inside a namespace, and with one namespace (`default`) the name
  is what it was.
- `--json` leaves `namespace` out for such a file (`reference_json`, and `document_name` for the
  document asked about), the way it already leaves out `key` and `project`.
- 16 tests are added: 11 through the built binary in `crates/typdoc/tests/outside_namespaces.rs`,
  5 in `typdoc-core` for `ref_name_in`, `RefName::is_same_document` and the `namespace`
  pseudo-field. The 10 that reach a root file failed on the unmodified tree with the panic (exit
  101); the control with one namespace passed before and after.

## Decision if any
Decided where the design is silent, with the doubt that remains:

- **`ref_name_in` returns a `RefName` whose `namespace` is optional, not an `Option<RefName>`.**
  The document exists in every such case, so the name is always known; only one of its parts is
  missing.
- **`Document.namespace` is optional too.** A `ref.*` condition reads the document it reaches as a
  `Document`, and the design says such a file has no namespace, so the `namespace` pseudo-field
  holds no value for it. A `list` candidate and a `get` result are always in a namespace, so their
  output is unchanged. Doubt: the query language reads an absent value and an empty one the same
  way for a string field (`namespace!=*` is true for both, and an ordering comparison on a string
  is refused), so no `--where` expression can tell the two apart. A unit test in `typdoc-core`
  pins the absence, which no run of the binary can.
- **`refs` prints no text form.** The command has only `--json`, so there is no text form to
  decide for a name with no namespace.
- **Two shapes of `Option` stay unreachable and are covered only by the type.** The document that
  `refs` is asked about and a `list` candidate are always indexed, hence always in a namespace, so
  `document_name` and `sort_value` take the optional value but are never given `None` today.

## Notes
- Consumers of `RefName` and of its `namespace` that were read: `Project::refs` (both directions,
  and the `--reverse` comparison through `is_same_document`), `evaluate_ref_condition` for
  `ref.*` and `refby.*` (and `RefEvalCtx.me`), `incoming_refs`, `evaluate_reached` and
  `evaluate_reached_target`, the three callers `ref_name_of`, `ref_name_of_resolved` and
  `resolve_import_outcome`, `refs_json`, `reference_json`, `document_name`, `document_json`, the
  `list` table cell and sort key for `namespace`, and `query::field_value`. `Toc` keeps a plain
  `namespace`, since a `toc` document is always indexed. The `fixtures/output/refs` goldens and
  the shell examples name only files inside a namespace and are unchanged; the whole suite passes
  with them as they were.
- `is_same_document` compares the optional namespace as it is: two names with no namespace and the
  same path are one document, and a name with none is never the same as one with a namespace. It
  cannot yet be asked about a root file from the command line, since `refs` needs an indexed
  document, so a unit test holds it.
- Seen and not changed: `list` leaves out a document with no frontmatter block at all when a
  `ref.*` condition is present (a file made only of a body link is not listed), which is how it
  behaves before this ticket; the tests give such a document an empty block.
- Not changed, and left open: the paragraph "Naming a document" in `docs/design.md` (under JSON
  output) still lists `namespace` among the parts every name has, which is no longer true for a
  reached file outside every namespace folder; it wants the same clause the Namespaces list now
  has. A document that a `ref.*` condition reaches outside every collection also still reads with
  an empty `collection` and `schema` (a string, not an absent value), as it did before.
- Checks run before the commit: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D
  warnings`, `scripts/test.sh` (729 passed, 0 failed, 1 ignored; 713 before, 16 added). A call to
  `std::fs::write` planted inside `ref_name_in` is refused by clippy as a disallowed method, and
  was removed.
- Mutations, each restored and compared with the saved copy: an empty-string namespace in
  `ref_name_in`, `reference_json` and `document_name` always printing `namespace`, `field_value`
  giving an empty namespace, `is_same_document` ignoring the namespace or the path, the folder
  prefix match without its `/`, an import named with this project's namespaces, and the original
  `expect`. Each is failed by a test added here, except the two unreachable shapes named under
  Decision (`document_name` and `sort_value` given `None`).

## Checked again after the build, by running
- `scripts/test.sh`: 729 passed, 0 failed, 1 ignored; `cargo fmt --check` and clippy with `-D warnings` clean.
- The two projects that stopped with exit 101 before, run on the final binary: a project with two namespaces and a root `README.md` (`refs` for a frontmatter ref and for a body link, `list` with `ref.any`, `ref.all`, `refby.any` and `ref.any($body)`), and a single-namespace project that imports a multi-namespace one and points at its root file (`refs`, `list --where 'ref.any(up)'`). All exit 0 with nothing on stderr, and the printed reference has `path` and no `namespace`.
- The design paragraph "Naming a document" now says `namespace` is absent for such a file.

