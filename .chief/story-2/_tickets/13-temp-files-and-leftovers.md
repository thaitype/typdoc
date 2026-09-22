# 13: Temp files and what a run leaves behind

Type: implementation
Status: resolved
Blocked by: 1

## What this delivers

- The reserved temp-file shape as one constant in `typdoc-core`, read both by the code that creates such a file and by the walker that skips it (decision 4).
- The walker skipping that shape by rule, before `match` is consulted.
- A leftover reported at `warn`, and counted in the audit among the entries that were not read, beside the symbolic links and the invalid names.
- A leftover inside a lock's scope removed by a command holding that lock, never by age, and a removal that fails never failing the write.

## Done when

- A file with the reserved shape is skipped under a `match` of `*` and under one of `*.md`.
- The accounting invariant of story 1 holds with the new count included, on every project in the fixtures.
- A leftover left by a stopped run is reported once, then removed by the next command that holds the lock.
- A removal made to fail leaves the write successful.
