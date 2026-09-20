# Ticket 3 Report

## Ticket
Golden files compared as parsed JSON with arrays in the declared order, hand-written assertion files that the generator cannot write, a generator that writes one named golden and nothing else, and the golden of `get`.

## Outcome
done

## Decision
Nothing blocked the build. The ticket left these open, and each is settled below, with the doubt that remains.

- **Layout.** A case is `fixtures/output/<command>/<case>/` with a hand-written `case.json` (project and command), a hand-written `assertions.json` (JSON pointer to expected value) and a generated `golden/stdout.json`. It is not `fixtures/golden/`: the guard is keyed on a folder named `golden`, and a root of that name would have accepted stray files. Doubt: ticket 20 checks the registry against the goldens and has to read the names under `fixtures/output/<command>`, so it depends on this layout.
- **Stdout only.** A case runs a command that ends with 0 and empty standard error, and pins standard output. The golden of the error object needs a `stream` key in `case.json`, which unknown-key refusal rejects today. That is for the ticket that builds the error golden.
- **The generator is an ignored test, not a binary.** The testkit cannot call `Command::new`, and the spawn helper is the one place that may. It shows as ignored in a plain run, with the command to run it in its reason. Doubt: this sits a little against "tests never skip silently". It is named in the run output and in `.chief/project.md`.
- **Assertions are checked against the live output, not against the golden,** and before a golden is written: output that breaks the assertions writes nothing. An empty or missing assertion file is red, and so is a missing golden.
- **The guard is name-based.** `write_golden` is public and refuses any target that is not `stdout.json` inside a folder named `golden`, that goes through `..`, or that is or sits behind a symbolic link; it writes a temporary file beside the target and renames it, so a hard link cannot reach an assertion file. The entry point, `regenerate`, takes one `<command>/<case>` id and has no id that means every case.
- **Strictness beyond the ticket:** a stray file under `fixtures/output/` is red (a `.gitkeep` there would be too), unknown keys in `case.json` are refused, and a case id is lowercase letters, digits, `-` and `_`.
- **`serde_json` with `preserve_order` in the testkit,** so a golden keeps the key order the binary prints. The comparison ignores key order anyway.
- **No per-array order declaration.** The design guarantees an order for every array in the shapes so far, so the comparison sorts nothing. If a shape arrives whose array has no declared order, this needs a way to say so.
- **The `get` golden is thin.** `valid/minimal` has no `code`, `key` or project, so this golden cannot catch a wrong value for those, and its assertions list every field the golden holds. Tickets 7 and 17 add the fixtures that can.

## Notes
- Run: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (112 passed and 1 ignored, from 79) and `scripts/check-public-text.sh` with the names list, all green.
- Re-run by hand, each planted, red, removed: the comparison made to accept everything, arrays sorted before comparing, and the generator's file-name check disabled. The real assertion file was unchanged after the last one.
- Shown by the ticket's own checks: a golden changed on purpose is red, an array in the wrong order is red, and a timestamp added to the `get` output turns both the golden and the two-runs test red. There is no `Clock` in story 1, so the no-clock check is a test that two runs print the same bytes.
- `cargo test -- --ignored` over the whole workspace makes the regenerate test panic with a message that it names one golden. It writes nothing.
