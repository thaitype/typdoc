# 4: Signals

Type: implementation
Status: claimed
Blocked by: 3

## What this delivers

- `signal-hook` at an ordinary version requirement, recorded in `.chief/project.md`; `SIGINT` and `SIGTERM` caught.
- The handler only wakes a thread, since the identity check and the unlink are not async-signal-safe; the cleanup runs there; a second signal waits for it rather than cutting it short.
- Registration before the first lock file exists; the process ends by the signal, not with an exit code of its own.
- A shipped binary that holds a lock, with no test-only code in it.

## Done when

- A real `SIGINT` and a real `SIGTERM` are sent to the shipped binary while it holds a lock: the lock file is gone afterwards and the caller sees a process killed by that signal, not an exit code from the table.
- A second signal during the cleanup does not cut it short.
- The window inside the creating call is not tested as closed, because it is not: what is tested is that a run meeting a lock file nobody owns stops at exit 4 with the file named.
