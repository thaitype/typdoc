# 15: Goldens for the write commands, and their exit codes

Type: implementation
Status: resolved
Blocked by: 9, 10, 11

## What this delivers

- Goldens for `new`, `set` and `mv`, with the injected clock so an `auto` field has a time a golden can hold, and the assertion files written by hand as before.
- `mv`'s golden carrying `unrewritten` and `findings`, including a move whose destination schema rejects the document and exits 0.
- Exit codes 3, 4 and 7 produced by tests, which takes them out of `unproduced_exit_codes`.
- `new`, `set` and `mv` out of `unimplemented_commands`, and the four rules out of `unimplemented_rules`.

## Done when

- Every array in the new shapes is compared in the order the design declares.
- The three lists hold nothing tagged for this story, and the checks in both directions still pass.
- A rule leaving `unimplemented_rules` has its fixture in the same change, so the suite is never red between the two.
