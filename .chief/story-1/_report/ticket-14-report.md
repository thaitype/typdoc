# Ticket 14 Report

## Ticket
`typdoc list`: choosing documents by collection, code and query, the fields shown, the sort, the limit, and the counts that say how much was found.

## Outcome
done

## Decision
Nothing the design or the contract asks for turned out to be impossible. The design does not say the following, so each is a reading that a later ticket may change, with the doubt that remains.

- **A field name is checked against every schema in scope before any document is read.** A name no schema in scope declares is an error; a name some schema declares is absent in a document whose own schema lacks it, and absence then follows the ordinary rules. Evaluating one condition against one schema cannot tell those apart, so the check belongs here, where the schemas of a scope are known. Both halves are tested: a name nothing declares, and a name one collection has while another lacks it, over a set spanning both.
- **`--sort` does not check its field against the schemas, and `--where` does.** The design makes only the direction an error under Sorting and states no rule for the field, while the unknown-name rule sits with the query conditions. The harm that rule guards against is a query that silently answers nothing; an unknown sort field changes no document, no count and no total, only the order. Doubt, and it is user-visible: a typo in `--sort` is silent, and the documents come back in key or path order as though nothing was asked.
- **`--collection` and `--code` together select the union of what each names,** since both name the same kind of thing and an intersection would make combining them almost always empty. An unknown collection name or code is refused rather than narrowing the scope to nothing, which is how a typo is caught elsewhere.
- **`--fields` replaces the columns the query would have shown, rather than adding to them.**
- **`--ids` and `--json` are refused together,** since the JSON shape has no place for a bare list of ids.
- **A document whose frontmatter cannot be parsed is left out of `list` and out of `total`, with no error.** `list` has no findings to report it under; `validate` is where it is reported. Doubt: a project with a damaged document quietly lists fewer documents than it holds, and only `validate` says why.
- **The order without `--sort`, and the tiebreak after every `--sort` key, is key order then path order,** with a key compared by its code and then by its number, as the decision base settles. A document with a key sorted against one without falls back to comparing the key's text with the path's, which nothing states.
- **A sort field whose declared type differs between two schemas ties, and a stored value that never fit its type sorts as missing,** rather than ordering across types. Missing sorts last in both directions.
- **The default table prints no header and separates columns by two spaces,** neither of which the design gives.

## Notes
- Run: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (539 passed and 1 ignored, from 513) and `scripts/check-public-text.sh` with the names list, all green, every cargo command under a memory ceiling.
- Re-run by hand, planted inside a function body, red with the error naming the banned call, removed: `std::fs::write` in `Project::list`. This ticket adds no guard of its own; the plant shows the existing bans reach its code.
- Both of the ticket's own criteria were shown able to fail before being trusted: with `total` computed after the limit, the test that `truncated` is true exactly when `total` exceeds the number listed goes red; with the table's column widths measured from the listed rows only, the test that a printed value does not change with `--limit` goes red. The code was restored after each.
- Checked by running, not by reading: both halves of the scope-wide field check, and `total` holding at four under no limit, a limit of one and a limit of ninety-nine, with `truncated` following it.
- One golden for `list`, with the `assertions.json` beside it written by hand.
- The message for a field no schema declares was reworded here. It was written when only one schema was ever in hand and read "the schema in scope"; this ticket made it reachable with several schemas in scope, where it miscounted what it was describing.
