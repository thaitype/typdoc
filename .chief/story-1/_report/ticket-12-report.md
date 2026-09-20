# Ticket 12 Report

## Ticket
`typdoc refs`: the refs a document holds, the documents that hold refs to it through a reverse index, `--field` in both directions, and unresolved references with their reason.

## Outcome
done

## Decision
Nothing the design or the contract asks for turned out to be impossible. The design does not say the following, so each is a reading that a later ticket may change, with the doubt that remains.

- **A reference carries either `path` or `unresolved`, never both and never neither.** A dangling ref is listed with its reason and no `path`; a resolved one carries `path`, `namespace` and, for a coded document, `key`. A test walks every reference of every case and asserts exactly one of the two is present, rather than checking one example.
- **A body link with no path at all is no ref** and is left out of `$body`, the same as a link with a URL scheme. Ticket 11 read such a link as a self-reference with nothing to check, and a document that referred to itself would otherwise turn up in its own reverse scan. Doubt: the design never says what the far end of such a link is, so this is read from what it leaves out.
- **Refs come in the order the document writes its fields, not the order the schema declares them,** and the fixture is written so the two orders differ; a fixture where they agree would prove nothing.
- **`--reverse` scans every namespace of the project whatever the scope,** because the design gives the reverse direction no scope of its own; a scope narrows only how the document being asked about is found.
- **A reverse scan lists no unresolved reference,** since a reference that resolves to nothing has no far end to be listed under.
- **A reference-style body link is its own entry at the position where it is used,** not at the position of its definition, and a definition nothing uses is checked but is not in `$body`. Doubt: the fixture's body links are all inline, so the reference-style case rests on ticket 11's own tests rather than on a golden here.
- **A ref that resolves to a file in no collection takes the namespace whose folder is the longest prefix of its path,** falling back to a namespace with no folder of its own. Not exercised: every ref in the fixture resolves to an indexed document, so this branch is reasoned rather than run.

## Notes
- Run: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (484 passed and 1 ignored, from 469) and `scripts/check-public-text.sh` with the names list, all green, every cargo command under a memory ceiling.
- Re-run by hand, planted inside a function body, red with the error naming the banned call, removed: `std::fs::write` in `document_out_refs`. The build added no guard and only read calls, so no plant was owed; it was run anyway to show the bans reach this ticket's code.
- Checked by running, not by reading: both directions, `--field` in each of them, `--field $body`, the shape of a dangling reference, and that `refs` is no longer in the list of commands the binary lacks.
- A golden for each direction, with the `assertions.json` beside it written by hand.
- The order of a reverse listing is sorted on the path rather than taken from the index's own iteration, which promises no order.
