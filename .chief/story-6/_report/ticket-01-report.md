# Ticket 01 Report

## Ticket
Rules for comments and cited design: a new section in `.chief/_rules/_standard/design-docs.md`
and a new `.chief/_rules/_standard/comments.md`.

## Outcome
done

## Notes
- "As in step 2 above" became an explicit reference to item 2 of "Every change to behavior
  updates `docs/design/` in the same PR": appended at the end of the file, "above" would also
  take in "Order of work in a story", whose step 2 is a different step.
- The test-file line uses the contract's wording: a test file that covers an SPC says so once,
  at its top; a test file that covers none says nothing.
- Gates at this commit: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D
  warnings`, `scripts/test.sh` (1057 passed, 0 failed, 1 ignored), `typdoc validate`, all green.
