# Ticket 10 Report

## Ticket
The forms a ref may take in frontmatter, resolved through the index of names as they are on disk, and the six rules that judge them.

## Outcome
done

## Decision
Nothing the design or the contract asks for turned out to be impossible. The design does not say the following, so each is a reading that a later ticket may change, with the doubt that remains.

- **A ref's form is decided row by row as the design's table gives it, and the conditions in that table are not the same for every row.** A bare key is a key only when it matches the shape and the code exists in this project; a key after a sibling prefix needs only that the namespace exists. The asymmetry is what tells a bare `WF-3` from a relative path of that name, a question a prefix has already settled, so both conditions do work and neither is redundant. A key-shaped ref whose code exists nowhere is therefore a relative path, not a key that fails to resolve. Doubt: a reader who takes the two rows to promise the same gate would expect `story-2:WF-5` to be refused when no schema has the code `WF`; here it is looked up in `story-2` and reported `not-found` if it is not there.
- **A prefix that names no namespace is `bad-prefix` and never falls back to a relative path,** which is what the design asks for when it says the two syntaxes never fall back to each other.
- **Every `name::` ref is `bad-prefix` in this story.** Imports are ticket 17, nothing here reads an alias or an imported project, so no such ref can resolve yet, and refusing by form keeps the promise that a form is never decided by what happens to exist. Doubt: when imports land this branch has to be replaced rather than extended, because `bad-prefix` for a real alias would then be wrong.
- **A cycle is looked for within one field name at a time,** never by joining the edges of two different fields into one graph. The design speaks of a cycle formed through a field, and keeping fields apart means an unrelated field cannot make another field's chain look circular. Doubt: a cycle that runs through two different acyclic fields is not looked for, and no fixture exercises one.
- **`refs.acyclic` is reported when documents are named as arguments, not only for a whole-project run,** filtered to the documents actually named. A document named on its own could otherwise sit on a real cycle and be reported clean, which is the same fault as counting a document that was never checked.
- **A `target` value that is neither `*` nor a list places no restriction,** since `schema.valid` already reports it as an invalid option; reporting it again under `refs.target` would name one schema fault twice under two rules.
- **A relative ref is resolved from the document's own folder, or from the namespace folder when `refBase` says so,** as the design's last table row gives it. Worth knowing when reading a fixture: a ref written `notes/x.md` inside `notes/a.md` means `notes/notes/x.md` under the default, not the file beside it.

## Doubt carried forward
- **A ref that resolves to a file matched by two collections** falls through to the case of a file that exists but belongs to no collection, because an overlapping path is not in the index. That follows from ticket 9 rather than from a decision made here, and no fixture exercises it.

## Notes
- Run: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (409 passed and 1 ignored, from 380) and `scripts/check-public-text.sh` with the names list, all green, every cargo command under a memory ceiling.
- Re-run by hand, planted inside a function body, red with the error naming the banned call, removed: `std::env::var` in `classify` in `refs.rs`.
- Checked by running, not by reading: a ref whose case differs from the file's is `not-found` while the same ref in the file's own case is clean; a cycle reported for the whole project and for one named document, with the counts each time; and the finding shape of `refs.resolve`.
- The six ids left `UNIMPLEMENTED_RULES`, each with a fixture under `broken/` whose expected set is written by hand. `import-absent` is not built: it belongs to ticket 17.
