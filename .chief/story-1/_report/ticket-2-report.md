# Ticket 2 Report

## Ticket
The registries of commands and rules, the three lists of differences between the design and the binary, the coverage checks over `fixtures/broken/`, and the loader that fails outside a checkout.

## Outcome
done

## Decision
Nothing blocked the build. The ticket left these open, and each is settled below.

- **Config error ids are rule ids.** They sit in the same registry and the same list as the rules, so that `fixtures/broken/config.parse` can be checked and a config error added to the design is noticed. All 40 ids the design's tables name (rules and `config.*` together) are in `UNIMPLEMENTED_RULES` for now. Tickets 4, 5, 17 and 18 take them out as they build them, and the checks turn red until each has a fixture.
- **The lists start fuller than the contract's end state.** Only `get` is built, so `list`, `toc`, `refs` and `validate` are in `unimplemented_commands` and every rule is in `unimplemented_rules`. The tickets that build them say that each leaves the list.
- **Exit code 6 counts as produced.** A `.typdoc/collections/folder.json` that is a folder makes the read fail with EISDIR and the run ends with 6. It was run, it does not depend on running as root, and it does not depend on which files a run reads. Only 3 and 4 are in `unproduced_exit_codes`. If ticket 4 changes how such an entry is treated, that case goes red and needs another way to produce 6.
- **"Produced by a test" for exit codes** is the table inside `produced_exit_codes()` in `tests/coverage.rs`, one run per code. Codes produced by other tests (the symbolic-link exit 6 in `get.rs`) are not counted.
- **`fixture.json`** holds `command` and `trips`, unknown keys refused, and the folder's own rule must be in `trips`. A missing `fixtures/broken/` folder means no broken fixtures, which is safe because a built rule with no fixture is red. The format is written down only in `crates/typdoc-testkit/src/spec.rs`, not in the contract.
- **Refusals beyond the two named checks:** a listed entry the design no longer names, an entry present that the design does not name, and a `broken/` folder for a rule that is still listed. They follow from an entry that outlives its work being red at once. They could be read as more than the ticket asked for.
- **Structure:** a dev-only crate `typdoc-testkit` (loader, design reader, checks) and a library target on `typdoc`, so that integration tests reach the registry. Both are described in `.chief/project.md`. `typdoc-testkit` bans `Command::new` like the other crates.

## Notes
- Run: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (79 tests, 39 before) and `scripts/check-public-text.sh` with the names list, all green. Re-run by hand: a rule removed from the list and the registry (red), a `broken/` folder that names no rule (red), a command in the clap definition that is also listed (red), exit code 5 listed while produced (red); each was removed.
- **Not shown able to go red end to end:** a fixture that trips a second rule. The binary emits no rule ids yet, so this is shown only by the unit test on `exact_set`. `tripped_rules` has never run on a real `validate` output; it is tested on hand-written JSON in the shapes the design fixes. Ticket 8 is the first to exercise both.
- **Vacuous today:** the two checks over `broken/` have nothing to check while no rule is built. They are shown red only by the planted faults, and ticket 8 is where they start to carry weight.
- Not part of this ticket: validating `examples/` (testing decisions, item 1).
