# Ticket 15 Report

## Ticket
The half of the query language that reaches through refs: `ref.all`, `ref.any`, `ref.none`, the same three under `refby`, and `$body` in either direction.

## Outcome
done

## Decision
Nothing the design or the contract asks for turned out to be impossible. The design does not say the following, so each is a reading that a later ticket may change, with the doubt that remains.

- **A condition through refs counts from what the field holds, so a dangling ref is counted and not skipped.** A ref that resolves to nothing counts as absent, as does a reached document that lacks the field, which is what keeps the pair complementary: over a set of six tickets where two hold a ref to a document that is not there, `ref.all(blocked_by).status=resolved` and `ref.any(blocked_by).status!=resolved` return four and two, with every document in exactly one half and the two holders of a dangling ref in the negated one. Skipping a dangling ref instead would put them in the first half as though everything they wait on were resolved, and the test for the pair fails when the code is changed that way.
- **Walking an arrow warns on stderr for each ref that does not resolve,** as the design asks, once per ref, in all three output shapes, and never on stdout. A query that walks no arrow prints nothing, so an ordinary listing stays quiet even in a project that holds dangling refs.
- **The scope of the condition after an arrow is the arrow's, not the outer selection's.** After `ref.*(f)` it is the schemas `f`'s `target` names, every schema when the target is `*`; after `refby.*(f)` it is the schemas that define `f`; for `$body` it is every schema. The field-name check that ticket 14 built follows that scope rather than `--collection`, and three tests separate the three cases.
- **The field `f` itself is looked for across the whole project, not the selected collections.** The design requires `f` to be defined in a schema of this project, with no words narrowing that to the current selection, and an arrow leaves the selection by its nature. Doubt: a later ticket that wants `f` narrowed to `--collection` would have to revisit this.
- **A `target` value that is neither `*` nor a list places no restriction,** the same reading ticket 10 took, so that one bad `target` is reported once by `schema.valid` and not again by every query.
- **The message for a name no schema declares is reused when `f` is no ref field anywhere.** It stays true in the case it is printed, but it does not say that the name exists as an ordinary field somewhere and is simply not a ref.
- **The wording of the stderr warning is this project's own,** since the design asks for the warning and gives no text.

## Doubt carried forward
- **A ref that resolves to a file in no collection is read under an empty schema.** No fixture reaches that branch: every ref in this ticket's fixture resolves to an indexed document, so the branch is reasoned and not run. Ticket 12's report left the same branch unexercised.

## Notes
- Run: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (562 passed and 1 ignored, from 539) and `scripts/check-public-text.sh` with the names list, all green, every cargo command under a memory ceiling.
- Re-run by hand, planted inside a function body, red with the error naming the banned call, removed: `std::fs::write` in `incoming_refs`. This ticket adds no guard of its own; the plant shows the existing bans reach its code.
- Checked by running, not by reading: the complementary pair over the set with dangling refs, the two halves and the whole; the stderr warnings, their absence when no arrow is walked, and their absence from stdout under `--json`.
- The pair was also checked by breaking it on purpose: with a dangling ref skipped instead of counted as absent, the test for the pair goes red. The code was restored afterwards.
- The fixture holds two documents whose `blocked_by` names a document that does not exist, a collection reachable only through a `target`, and a field that one schema declares and another does not, so that the scope tests can tell the three scopes apart.
