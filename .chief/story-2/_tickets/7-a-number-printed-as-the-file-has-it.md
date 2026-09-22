# 7: A number printed with the digits the document holds

Type: implementation
Status: claimed
Blocked by: None (can start immediately)

This ticket is on its own and its change is its own commit. Closing this gap rewrites every golden
that holds a number, and a diff that size mixed with other work hides the other work.

## What this delivers

- A field typed `number` is printed in `--json` with the digits written in the document, not with a value converted out of them (decision 10). `1e3` prints as the document has it, and two numbers that differ in their last digit print differently.
- The known gap `[number-text]` is removed from `KNOWN_GAPS` in this change.
- Every golden holding a number is regenerated, with the generator's guard unchanged and the assertion files still written by hand.

## Done when

- The test that pins the gap on both sides turns red in its old direction and is turned round, not deleted.
- Values outside the range a primitive holds come back whole: the pairs measured in decision 10 are covered, including two that print the same today.
- Comparison is untouched, which is decision 22 and not this story; a test records that it is still as it was, so the change is not read as fixing it.
- No golden changes in a way the printing rule does not explain.
