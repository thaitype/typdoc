# Ticket 20 Report

## Ticket
The checks that run across the commands rather than inside one: the registry against the goldens and against the design's own headings, every exit code produced or listed, a printed name accepted back by `get`, and `examples/` validated as a reader receives it.

## Outcome
done

## Resolved out of order, on purpose
This ticket is blocked by the one before it, and that one is still open. It was built and resolved anyway, which was decided outside this ticket and is not a judgement made inside it. It is written here so that nobody reading the order later takes the rule about blocking edges to be one that is kept sometimes and not others: it was set aside once, knowingly, and the reason was that the open question in the ticket before it is about a number in one report's summary, not about anything this ticket checks.

The reach of that open question was measured rather than assumed: no golden file holds the shape of an audit's summary, and the only place that shape is asserted is the tests of `validate` and one line of usage in the harness for the design's shell examples. So if the answer changes that shape, what has to be redone is those, and not the cross-command checks in this ticket.

## Decision
Nothing the design or the contract asks for turned out to be impossible. The design does not say the following, so each is a reading that a later ticket may change, with the doubt that remains.

- **Most of this ticket already existed and was verified rather than rebuilt.** The registry is already checked against the design's command headings and exit-code table, and the three lists already hold exactly what this story does not build. What was missing and is now added is the registry against the goldens, in both directions: a command with no golden and a golden for no command each fail.
- **Exit code 6 is produced by a test, not listed.** A symbolic link that a collection matches is the case the earlier tickets described, it can be made deterministically here, and a test already ends with that code. The code itself is left alone: both earlier reports say 6 is probably wrong because a retry never fixes it, and that is a decision above this ticket.
- **Three differences between the design and the binary are now in a list a test keeps honest,** instead of living only in reports and comments where they are easy to lose. Each entry carries a short stable tag and a sentence, and each is pinned by a test that demonstrates the narrower behaviour and asserts that the list still names it. A gap that is later closed makes its own test fail, which is what forces the list to shrink rather than rot. They are: a reverse lookup does not enter an imported project; a namespace named after a URL scheme is not reported while an import alias of that name is refused; and a body link crossing an import has its file checked and its anchor not.
- **A fourth candidate was checked and is not a gap.** `refs.target` and `refs.codedByPath` across an import were closed by the ticket that read pinned schemas, and tests that predate this ticket already prove it. It is left off the list rather than listed out of habit.
- **`examples/` is a single self-contained project,** without the field of the design's worked example that reaches into a second project, so that a reader can copy one folder and have it validate. Doubt: a reader following the fuller example in the design still has to add the import themselves.

## A fault found while building the round trip
The test the contract asks for is that every string the design's table of arguments gives, for a document of a project with one namespace, one with several, and one reached through an import, is accepted back by `get`. Building the third case found a real fault: a path into an imported project with several namespaces was refused with a demand to name a namespace, though the design says the path form needs to know nothing about how many namespaces a project has. Only the key form has to name one. It is fixed where the scope of an imported project is worked out, and the key form still refuses as it should.

## What could not be shown
- The gap about reverse lookup could not be demonstrated by breaking the code and watching the test fail, the way the other two were. Widening the reverse scan to walk the imports does not resolve the ref, because a project loaded as an import does not load its own imports: that is the one-level rule this story decided. **This is worth carrying:** closing that gap is not "scan the imports too", because resolving a ref that points back into the importer needs the imported project's own import graph, and that collides with a rule already decided. The test's premise is checked instead by running the imported project on its own, where the one-level rule does not apply, so the difference it asserts is real and not an artefact of the fixture.

## Notes
- Run: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (665 passed and 1 ignored, from 655) and `scripts/check-public-text.sh` with the names list, all green, every cargo command under a memory ceiling.
- Re-run by hand, planted inside a function body, red with the error naming the banned call, removed: `std::fs::write` in the working out of an imported project's scope.
- Checked by running, not by reading: `examples/` validating clean with its three documents, a path into an import with several namespaces, and the three entries of the list of known gaps against the tests that pin them.
- The registry-against-goldens check was shown able to fail by renaming a golden folder, which failed it in both directions at once.
