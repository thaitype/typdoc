# 3: The namespace lock, and one path to it

Type: implementation
Status: claimed
Blocked by: 1

## What this delivers

- The lock file created with `O_EXCL`, holding pid, hostname and timestamp; retry with backoff to `--lock-timeout`, then exit 4 with the message the design specifies; no takeover at any age.
- The identity check before removal — `stat` on the path against `fstat` on the handle held since creation — and the report when it does not match.
- Acquiring returns a value with no public constructor and no public fields, returned by no other function; every function that writes takes it by reference (decision 6).
- The order of decision 3 for taking more than one lock, the project lock first; the `git-common` path and the project hash of decision 14, with `--git-common-dir` canonicalized before use.
- The module is named so that it is not read as `lock.rs`, which is `lock.json`, the pin file.

## Done when

- A compile-fail test shows that the lock value cannot be constructed outside its module and that a writing function cannot be called without one; each case has a variant that does compile, so the harness is shown to work.
- Exit 4 is produced by a test, with the message naming the path, pid, host and age.
- The hash is checked against a worktree of the same repository holding the project at the same relative path, and against one holding it elsewhere, which are a different project on purpose.
- A lock another process created is never removed, at any age, shown by a test.
