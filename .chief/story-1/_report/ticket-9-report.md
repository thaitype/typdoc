# Ticket 9 Report

## Ticket
The rules that judge a project rather than one document's frontmatter: `schema.valid`, `collections.overlap`, `keys.unique` and `filename.pattern`.

## Outcome
done

## Decision
Nothing the design or the contract asks for turned out to be impossible. The design does not say the following, so each is a reading that a later ticket may change, with the doubt that remains.

- **A file matched by two collections is taken out of the index, and the three commands answer it differently.** The design says such a file is an error and is never settled by precedence, so no command may pick one of the two. `get` and `toc` refuse it with an id-less exit 2 that names both collections and points at the rule, because they must answer with one schema and there is none. `validate` does the opposite: the overlap is a rule, so it is a finding in the report and the run carries on. The design names only two things that stop `validate` before a report, an ambiguous key and an argument that names no document, and an overlap is neither; naming one overlapping document alongside a good one must not cost the good one its report.
- **An overlapping file is counted nowhere as checked.** It adds nothing to `checked.documents` and nothing to `checked.paths`, because nothing about it was checked against a schema. Its namespace still appears in `checked.namespaces`, which says what the run covered rather than what it read. Doubt, and the one place this strains the design: `checked.paths` is described as being there so that it can be matched against the `path` of a finding, and the `path` of a `collections.overlap` finding is deliberately not in it. Counting the file instead would be worse, because a reader would take its frontmatter to have been looked at when it never was.
- **`required`, `acyclic` and `override` are read tolerantly.** A value that is not a boolean no longer stops the schema from being read; it reads as false and is reported under `schema.valid`. This is the relaxation ticket 5 recorded as needed, so that a wrongly typed option is a finding rather than a stop, as an unknown type, `auto` or `target` name already was.
- **An `extends` cycle is now rejected** under `schema.valid`, rather than ending the chain silently as ticket 5 left it.
- **What counts as an invalid option:** a wrongly typed boolean, an option that does not apply to the field's type, an unrecognized field type or `auto` value, a bare `target` that is not `*`, and any key on a field that is no option at all. Not included: whether the value of `default` fits the field's type. Nothing asks for that check and it is a different piece of work; doubt: a `default` of the wrong type is silently accepted until something asks for it.
- **Duplicate schema names and codes are compared across the schemas that collections name directly**, since those are the ones that share the project's names. Two collections naming the very same schema file are not a duplicate here; for a coded collection that is already `config.coded-schema-shared`, and for an uncoded one it is allowed.
- **A `collections.overlap` finding carries `path` and `namespace` and no `collection` or `key`.** Which collection the file belongs to is exactly what is in doubt, so neither is guessed.
- **`keys.unique` reports every document that shares the key**, not the ones after the first: nothing makes one of them the correct one. Two namespaces that share a key stay clean, since keys are per namespace.
- **A `filename.pattern` finding carries `namespace` but no `collection` or `key`,** because the file is no document. Doubt: the design's finding shape can be read as binding `namespace` to the same condition as `collection` and `key`, in which case a file that is no document would carry neither; `namespace` is treated here as a fact about where the file is.
- **Left out on purpose:** a qualified `target` that names a schema missing from an imported project needs imports, which is ticket 17.

## Doubt carried forward
- **`filename.pattern` looks at a coded collection's folder only when `{key}` is the last part of its template.** A template like `tickets/{key}/index.md` puts each document in its own folder, and "a file in the collection's folder" then has no one meaning, so no file is checked against such a collection. Nothing in the design forbids that template, and every worked example in it puts `{key}` last, so there was no example to check a wider rule against. Not generalized rather than generalized wrongly; the places that would have to change are marked in the code.

## Notes
- Run: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (380 passed and 1 ignored, from 365) and `scripts/check-public-text.sh` with the names list, all green, every cargo command under a memory ceiling.
- Re-run by hand, each planted alone, red, removed: `std::fs::write` inside `Index::overlap` (library code), which failed the crate and its tests alike, and the same call inside the `#[cfg(test)]` module of `schema.rs`, which failed only the test target while the library still compiled, which is what shows the test-code ban was reached on its own. A first attempt at the library plant put the call outside any function and failed to compile for an unrelated reason; a run that ends red is not a guard that went red, and it was redone inside a function body.
- Checked by running, not by reading: an overlapping file under `validate` alone, beside a good document, and under the whole-project scan, with the counts each time; `get` on the same file still refusing with exit 2; and that a document with the same frontmatter fault as its overlapping neighbour is the only one of the two that produces a finding.
- The `broken/collections.overlap` fixture gained a second document with a frontmatter fault. With one empty file it could not have caught either of the two faults found in review, since there was nothing to miscount and no finding that could go missing.
- Each rule left `UNIMPLEMENTED_RULES` and has a fixture under `broken/` with its expected set written by hand.
