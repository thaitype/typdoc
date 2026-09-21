# Ticket 22 Report

## Ticket
A reverse lookup counted a ref that had left the project: a ref that resolved into an imported
project was compared with a document of this project by path alone, so `refs --reverse` and a
`refby.*` condition in `list` both reported a same-path document of this project as the thing
pointed at.

## Outcome
done. Both readers now compare the target of each ref with the document asked about by project,
namespace and path (`RefName::is_same_document`). The shape of the ticket (project `a` importing
`b`, `a/notes/pointer.md` holding `see: b::notes/target.md`, a `notes/target.md` in both) now
answers "nothing points at `a/notes/target.md`" from `refs --reverse` and from `refby.any(see)`.
The output shapes did not change.

## Decision if any
- **The reference printed by the reverse direction carries no `project`, and should not.** The
  control tests compare the whole reference object, so they also pin that no `project` is
  printed. A reference in the `in` direction is the name of the document that holds the ref. A reverse lookup
  scans this project's own documents only (the recorded `[reverse-scope]` gap, left as it was), so
  the holder is always a document of this project, and the design gives a document of this project
  no `project`. Before this change the `written: b::notes/target.md` of a wrongly reported entry
  was the only sign it had left the project; with the fix such an entry is no longer reported at
  all, so there is nothing left for `project` to say. It will be needed when the gap is closed and
  a holder can be a document of an imported project.
- **Namespace is part of the comparison, and no input can tell that it is.** Within one project a
  path belongs to one namespace, and a document of an imported project already differs in
  `project`, so leaving the namespace out of the comparison changed no result in the whole suite
  (run: 688 passed, 0 failed with the clause removed). It is kept because the key of the index is written as
  project, namespace and path, and it is not covered by a test that can turn red for it.

## Notes
- Tests are in `crates/typdoc/tests/imports.rs`, run through the binary: for each of `refs
  --reverse` and `list --where 'refby.*'`, a fault case (the ref is `b::notes/target.md`) and a
  control (the ref is `target.md`, which resolves from the folder of `pointer.md` to
  `a/notes/target.md`), for a frontmatter ref and for a body link. The two projects differ only in
  where the ref goes: `b::notes/target.md` and `target.md` resolve to the same path, so a
  comparison that ignores the project cannot pass the fault case, and a comparison that drops every
  cross-project or every ref cannot pass the control. Each fault case first shows that the ref
  resolves (`refs notes/pointer.md` gives `project: b` and no `unresolved`), so a `not-found`
  cannot stand in for the fault.
- Run on the tree before the change, the four fault cases failed (the reverse ref and `refby.any`
  each reported `notes/target.md`) and the four controls passed. After the change all eight pass. A ninth test puts a condition after
  `refby.any(see)` (`.path=notes/pointer.md`) through the same pair; the suite is 688 passed, 0
  failed, 1 ignored.
- Mutations, each restored afterwards: path-only comparison in `refs` alone turned the two
  `refs --reverse` fault cases red; path-only in `refby.*` alone turned the two `list` fault
  cases and the ninth test red; a comparison that ignored the project turned all four fault cases red; a comparison
  that was always false turned the four controls red.
- Readers of the index checked: `Project::refs` (reverse branch, text and `--json`, with and
  without `--field`), `Project::incoming_refs` and `evaluate_ref_condition` (every `refby.*`
  quantifier and the nested condition), and the `ref.*` direction, which does not read the reverse
  index and already dispatches by project. `validate` and `get`/`toc` do not read it. `mv` is not
  built. The golden case `refs/reverse` and the shell examples that use `refby` still pass
  unchanged.
- The recorded gap `[reverse-scope]` and its test are unchanged: a reverse lookup still does not
  enter an imported project.
- Guard: a call to `std::fs::write` planted inside `RefName::is_same_document` turned
  `cargo clippy --workspace --all-targets -- -D warnings` red with `use of a disallowed method
  std::fs::write`, and was removed.
- `refby.all(f).EXPR` has no test of its own; it goes through the same comparison as `any`. Run by
  hand on a two-project scratch shape with `refby.all(see).path=notes/other.md`: with the ref
  inside this project `notes/target.md` is not listed (its one incoming ref fails the
  condition); with the ref into the import it is listed, since nothing here points at it and
  `all` holds for an empty set.
