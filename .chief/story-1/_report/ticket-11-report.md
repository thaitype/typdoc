# Ticket 11 Report

## Ticket
The body side of refs: which links are checked, `$body` as a ref field, and the rules `body.links`, `body.anchors` and `body.mentions`.

## Outcome
done

## Decision
Nothing the design or the contract asks for turned out to be impossible. The design does not say the following, so each is a reading that a later ticket may change, with the doubt that remains.

- **A collection's `validation` merges into `validation.global` key by key inside a rule's own setting, not one whole setting at a time.** The design says a collection file merges key by key so that it states only what differs, and its two worked examples compose into exactly this case: the project sets `body.links` to an error level with an `ignore` list, and a collection sets `body.links` to a warn level and nothing else. That collection keeps the `ignore` and changes only the level. Replacing the whole setting would mean a collection that wants a different level has to restate every option or lose it silently, which is the opposite of stating only what differs. Checked by running, with a control: a link into an ignored folder is silent under both a collection that names the rule and one that does not, while a link outside it is reported at the collection's own level.
- **A rule's setting is always an object.** A bare level (`"body.links": "error"`) is refused when the config is read, so the merge never has two shapes to reconcile. Checked by running.
- **A definition that nothing uses is checked but is not a use,** and a duplicate label leaves the first definition active while later ones are reported, with labels compared without regard to case and with runs of whitespace treated alike.
- **Text that looks like a link is found by a same-line, first-bracket and first-parenthesis reading,** not by a second implementation of the link grammar. Every case the decision base names is covered, including that `version 1.2` is not reported and that an undefined `[t][ref]` is not reported. Doubt: it accepts the known false positive of a bracket inside the link text, and the decision base already recorded `ask Mr.Smith` as one it reports on purpose.
- **A definition inside fenced code is not a definition,** as the design says, and neither is a link inside fenced or inline code a link.
- **`[t]()`, an empty destination, is read as a self-reference with nothing to check** and is silent, like `[t](#)` with no fragment. The decision base left this open and the design never settles it; the quieter reading was chosen. Not covered by a fixture.
- **`ignore` uses the same glob shape as a collection's `match`,** rather than a second dialect. A value that is no valid glob matches nothing and is not reported: the design's table has no config error id for a malformed `ignore`, and none is invented here. Doubt: a typo in an `ignore` glob is silent, so a rule keeps reporting what the writer meant to ignore.
- **A body link with an import prefix is reported with the same wording as a target that is missing,** since the design gives exact wording only for the missing case.
- **Heading text is scanned for mentions,** since nothing says to leave it out.

## Notes
- Run: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (469 passed and 1 ignored, from 409) and `scripts/check-public-text.sh` with the names list, all green, every cargo command under a memory ceiling.
- Re-run by hand, planted inside a function body, red with the error naming the banned call, removed: `std::fs::write` in `percent_decode` in `links.rs`.
- Checked by running, not by reading: the option merge above with its control, the level override still taking effect outside an ignored path, a bare level refused as a config error, and the three rules' fixtures.
- A body link that carries an anchor is matched against a recorded move by its path alone, since a move records a path with no fragment. Before this was fixed, such a link was reported as an ordinary missing target and the move was never mentioned; the test for it fails against the earlier code.
- `$body` is not yet wired to the query grammar or the `refs` command: the scan keeps what those need, and they are tickets 12 and 15.
- The three ids left `UNIMPLEMENTED_RULES`, each with a fixture under `broken/` whose expected set is written by hand, and one clean fixture holds every case that must produce nothing.
