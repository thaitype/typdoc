# Ticket 16 Report

## Ticket
A harness that runs the design's own shell examples through real shells and checks that what a shell hands the tool is what the example means.

## Outcome
done

## Decision
Nothing the design or the contract asks for turned out to be impossible. The design does not say the following, so each is a reading that a later ticket may change, with the doubt that remains.

- **Every example found in the design is either run or named as a template with a reason.** Sixty-five texts are found; twelve hold a character outside the safe set and carry a value written by hand; twenty-one are templates, each with its reason (a placeholder, a value known only at run time, one line of a multi-line loop, or printed output); the remaining thirty-two hold nothing unsafe and are run as their own words. Two tests keep both directions honest, so a declaration with no example and an example with no declaration are each red.
- **`TYPDOC_NAMESPACE='*'` is named a template rather than given a declared value,** because a bare assignment's value is neither split nor expanded whether or not it is quoted, so a harness that claimed the quotes mattered there would be asserting something untrue. Checked by running under both shells, with a control in the same directory: as an assignment the star stays a star, while the same star as an argument expands to the three files that are there. Doubt: the design gives this example as an assignment with no command after it, and says nothing about how to test one.
- **Process spawning lives in one function**, which carries the one narrow allowance, and the crate's ban catches it anywhere else. The existing spawn helper could not serve this ticket: it starts the built binary with arguments it already holds, and what is being tested here is what a shell does to those arguments before the tool is reached.
- **A shell in the design's list that is missing turns the suite red** with a message that names both ways to fix it, rather than skipping quietly. The list is `sh` and `bash` because this host has those two and no zsh.
- **A trailing comment on a fenced line is stripped before the line is read as an example.**
- **Seven of the twelve declared examples lose their quotes into a shell syntax error rather than into a value that is silently wrong,** because they hold brackets. The remaining five cover the quieter case the design's Quoting paragraph is really about, where the command still runs and passes something else. Both are real behaviour, and the test says which is which rather than treating the syntax errors as if they proved the quieter case.

## Notes
- Run: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (579 passed and 1 ignored, from 562) and `scripts/check-public-text.sh` with the names list, all green, every cargo command under a memory ceiling.
- Re-run by hand, planted inside a function body, red with the error naming the banned call, removed: `std::process::Command::new` in the example reader of the test kit.
- The harness was checked by making it fail, not only by watching it pass: with one declaration removed while its example stayed in the design, the coverage test goes red and names that example. The file was restored afterwards. The build also showed red, and restored, an example whose declared value was made to equal its unquoted result, a declaration with no example, and a shell in the list that is not on this host.
- The directory the unquoted variants run in holds a file whose name matches the star in the examples, so that a missing quote shows up as an expansion rather than as nothing.
- This ticket adds no code to the read core, so the bans there are not exercised by anything new; the bans that do apply to its own code were shown able to go red.
