# Ticket 13 Report

## Ticket
The scalar half of the query language: the grammar, the escaping, coercion by type, the pseudo-fields, the glob, and absence and negation.

## Outcome
done

## Decision
Nothing the design or the contract asks for turned out to be impossible. The design does not say the following, so each is a reading that a later ticket may change, with the doubt that remains.

- **The seam is `parse_query` and `evaluate`.** Parsing reads one condition with no schema; evaluating takes one schema and one document. `list` calls the second once per candidate document, with the schema that document was read under. `ref.*` and `refby.*` are not parsed here and fail as a syntax error, because a field name holds no dot; the grammar grows for them in ticket 15.
- **Absence follows the design in both directions, and the ordering comparisons are its exception.** Something absent fails every positive condition and satisfies every `!=`, so `k=v` and `k!=v` split a set with nothing left over and nothing counted twice; but `k<v` and `k>=v` both leave an absent document out, which is what the design says and is not the general rule. A value stored as written because it never fit its declared type counts as absent for an ordering comparison too, since there is no number, date or datetime there to compare.
- **A value that cannot be coerced is an error only where the design says so.** For `=` and `!=` against a number, bool, date or datetime, a value that does not coerce compares unequal rather than failing the query; the enum case and the case the design names stay errors.
- **A glob is matched against the value as written,** since globs are never coerced. A number renders canonically, so a number written in scientific notation may render differently from what the file holds. Not exercised by any test and named by no example in the design.
- **An empty alternative in a list (`status=open,`) is the same error as a wholly empty value.** The design names only the whole-value case. Doubt: this widens a rule the design states narrowly.
- **The "did you mean" hint strips every space and retries the parse.** It is right for the design's own example, a space around the operator; for a space inside a field name it can suggest something that parses but means little.
- **A field name unknown to the one schema in hand is an error here, and that is not yet the design's full rule.** The design makes a name unknown to *every* schema in scope an error, while a document whose own schema lacks it counts as absent. Evaluation sees one schema and cannot tell those apart, so the distinction belongs to whoever assembles the schemas of a scope: ticket 14 must ask, before evaluating per document, whether any schema in scope defines the name, and treat a document whose own schema lacks it as absent rather than letting this error fire.

## Notes
- Run: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (513 passed and 1 ignored, from 484) and `scripts/check-public-text.sh` with the names list, all green, every cargo command under a memory ceiling.
- Re-run by hand, each planted alone inside a function body, red with the error naming the banned call, removed: `std::fs::write` in `evaluate` (library code) and the same in a test of `tests/query.rs` (test code). This ticket adds no guard of its own; the plants show the existing bans reach its code.
- The tests for absence were checked by making them fail on purpose, not only by watching them pass: with an absent value made to satisfy an ordering comparison, the test for a document without the field and the test for the pair that leaves it out both go red; with a misfit stored value made to satisfy one, the misfit test and the same pair go red. The code was restored after each. A test added to pin a rule is worth only as much as the change it catches.
- One test per error the grammar names, ten in all, each reaching exactly one failure. No fixture files: the grammar and its evaluation read nothing from disk, so schemas and documents are built by hand in the tests.
- The pair that splits a set is asserted per document, not per query: every document is in exactly one half, and the halves sum to the whole.
