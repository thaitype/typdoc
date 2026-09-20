# Ticket 8 Report

## Ticket
The report of `validate` as JSON output describes it, rule levels merged and raised by `--strict`, the three scopes, and the rules `frontmatter.parse`, `frontmatter.types` and `frontmatter.unknown`.

## Outcome
done

## Decision
Nothing the design or the contract asks for turned out to be impossible. The design does not say the following, so each is a reading that a later ticket may change, with the doubt that remains.

- **A `frontmatter.parse` finding never carries a position, whatever caused it.** This is the default contract item 7 names, and it applies because the reader does not tell the kind of error: `yaml_serde` 0.10.7 gives a precise mark for a syntax error, no mark at all for a block holding a second document, and the start of the mapping rather than the offending line for a duplicate key, with nothing in its public surface to tell a precise mark from a backfilled one. Checked by running a standalone probe against the library, not by reading it. Doubt: a reader that later distinguishes the two would let syntax errors keep their position, and a finding that gains a position is a change a consumer can see.
- **`frontmatter.types` and `frontmatter.unknown` carry a `field` and no position.** Nothing maps a field back to a line through the reader yet, and the design lets a finding omit a position it does not know.
- **`--audit` is refused as not built yet (exit 1).** Audit mode is ticket 19; the design gives `--audit` a summary with `audit` and `unreported` and a report with an `audit` object, so a run that quietly returned the plain report would answer a different question from the one asked. The refusal follows the voice already used for the text output and for an imported project. `--audit` together with arguments still fails as bad arguments first, as the ticket asks. It is not in `registry.rs`: both lists there are checked against a structural marker in the design, a command heading and the exit-code table, and a flag of a command that exists has neither, so the refusal is pinned by a test instead. Doubt: a difference between the design and the binary that lives only in a test is easier to lose than one in a list that must shrink.
- **Every config error still stops the command, so none is a finding in the report yet.** Of the fifteen, only `config.parse` and `config.version` stop the reading of the config itself; the other thirteen are gathered and the rest of the config is still determined, but the end of the load turns any gathered error into a failure alike. `validate` loads a project exactly as `get` and `toc` do, so it never sees one. The place where an error that leaves checking possible would flow into the report instead is marked in the code.
- **Levels merge from the defaults, then `validation.global`, then the collection, and `--strict` raises warnings to errors last,** so the counts in `summary.findings` agree with the exit code. A rule set to `off` produces no finding at all rather than a finding of level `off`.
- **A document named twice in the arguments is checked once**, and `checked.paths` holds each path once, sorted, so it can be matched against the `path` of a finding.
- **An argument that names no document, or a key in more than one namespace, stops the command before any report,** as the design says, and is never a finding.
- **The scope of a whole-project run comes from the same namespace selection every other command uses**, not from the namespaces that happen to hold a document.

## Open against the design, carried to the end of the story
- **`config.legacy-file` stops every command today, and by the design's own test it should not.** The design decides what a config error does by one question, whether it makes checking impossible, and says an error that leaves checking possible is a finding in the report of `validate` and stops nothing. A legacy `.typdoc.json` beside the folder says nothing about whether `.typdoc/config.json` can be read. Checked by running: with the stray file present every command exits 2 and nothing is checked; removing that one file and changing nothing else makes the same project validate clean, which is what shows that the stray file never made checking impossible. This is not left silent anywhere: the answer for each error belongs to whoever adds it, the behaviour was set when the error was built, and changing it now would change what `get`, `toc` and `validate` do with a project that is otherwise readable. It is written down here rather than changed inside this ticket, and it is in the report at the end of the story.

## Notes
- Run: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (365 passed and 1 ignored, from 333) and `scripts/check-public-text.sh` with the names list, all green, every cargo command under a memory ceiling.
- Re-run by hand, each planted alone, red, removed: `std::fs::write` in `validate.rs` library code, and the same in its `#[cfg(test)]` module. No ban was added this ticket, so this shows the existing bans reach a new file.
- Checked by running, not by reading: `--audit` alone refused, `--audit` with arguments still refused for the conflict, `--schemas` unaffected, a plain report unchanged, and the `config.legacy-file` case above.
- The three rules left `UNIMPLEMENTED_RULES` and `validate` left `UNIMPLEMENTED_COMMANDS`; each rule has a fixture under `broken/` with its exact expected set, including a second `frontmatter.parse` document for a block that holds a second YAML document.
- Goldens for `validate` were regenerated by the ticket-3 generator; the hand-written `assertions.json` beside each is what the test checks.
- Not shown: an on-disk argument naming a file under a different project root than the first argument; it becomes exit 5, which was not tested against a second real project tree.
