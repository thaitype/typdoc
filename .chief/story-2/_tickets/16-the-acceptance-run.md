# 16: The acceptance run and the closing report

Type: implementation
Status: claimed
Blocked by: 4, 6, 7, 12, 13, 14, 15

## What this delivers

- The round trip of the contract's first criterion run by hand against copies of the public `chief` and `typmem` repositories, each with a `.typdoc` folder written for the run and not committed to them.
- The story's closing report, recording that result, what differs from the design, and what the story leaves open.

## Done when

- The run is recorded with its counts, and any value that differs is named rather than summarized.
- The report lists what the contract left undecided and is still undecided: durability without `fsync`, the text output of the three commands, where `--lock-timeout` is accepted, the backoff schedule, how `--set` parses, whether `set` may create an unknown field, the order inside the lock in `new`, and exit code 6.
- `KNOWN_GAPS` holds only `[reverse-scope]` and `[import-anchor]`.
- The report says what the story built that differs from the design, and what it left as it found it.
