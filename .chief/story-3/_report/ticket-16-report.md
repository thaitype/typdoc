# Ticket 16 Report

## Ticket

Text output for `get`, `set`, and both forms of `new`: the shared labeled-block renderer, and
the error-path fix from hard-coded `true` to each command's real `json` flag.

## Outcome

done

## Notes

`document_text()` in `crates/typdoc/src/cli.rs` is the one shared renderer — `push_line()` for
each `name: value` line, `field_text()` for a field's value (reusing `Number::written()` for the
number-text rule, not reimplementing it). `get`, `set`, `new <path>`, and `new <CODE>` all call
it; `new <CODE>`'s bare-key output is gone, replaced per M-10(f)'s accepted change. Every
`failure`/`failure_text` call site in these four branches now passes the real `json` variable.

Verified directly (not just trusting the build report): the previously-reproducible bug —
`typdoc new "not a code and not a path"` with no `--json` printing the raw JSON error object —
is fixed; re-ran the before/after comparison myself after rebasing onto the current story branch
tip (tickets 9 and 14 had landed in between). Goldens exist for all four shapes plus the
regression test for the bug and extra error cases (not-found, validation failure, duplicate
target, malformed `--set`).

Rebased cleanly onto `story-3-catalog-and-release`'s tip (tickets 9, 14 in between — no
conflicts, disjoint files) and re-ran all three gates after the rebase, not just before it:
`cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and
`scripts/test.sh` (85 tests in the affected suite, 0 failed; full workspace doc-tests clean)
all green post-rebase. Fast-forward merged into `story-3-catalog-and-release`.

Ticket 21 (`mv`) depends on `document_text()` — it's a free function in
`crates/typdoc/src/cli.rs`, callable directly for the destination document's block.
