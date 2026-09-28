---
blocked_by:
- TK-5
- TK-6
- TK-7
status: resolved
title: Measure on windows-latest what the research could not confirm
type: wayfinder:task
---

# TK-12: Measure on windows-latest what the research could not confirm

## Question

Run on a `windows-latest` runner, with NTFS, and record what happens: a lock file deleted by another process
while held (link count and `DeletePending` on the held handle, opening its path, the error `CREATE_NEW` gets at
that path); `std::fs::rename` over a read-only file and over a file another process holds open with and
without share-delete; `ReplaceFileW` over a document (what it keeps, whether it needs the replacement closed);
Ctrl+Break to a child started in its own process group, and the exit code the parent sees. How the probe runs
(a probe test on this pull request, removed after, or elsewhere) is the director's call.

## Answer

Measured on `windows-latest` (Windows Server 2025, 10.0.26100; `C:` and `D:` NTFS), with a throwaway probe on a
separate pull request, closed without merging (run 36369183353, repeated in 36369270665). Files in `%TEMP%`.

- A lock file removed by `std::fs::remove_file` while its handle is held: the name goes at once (the path is
  not found, os error 2), so deletion is POSIX by default here. The held handle then reports `links=0`,
  `delete_pending=true`, and the same 128-bit id as before. `create_new` at the path succeeds while the old
  handle is still open, and gets a new id.
- File ids have a zero high 64 bits on NTFS.
- `std::fs::rename` over a read-only target: access denied (os error 5), the target keeps its bytes. Over a
  target another handle holds open without share-delete: access denied (5). Over one held with share-delete
  (std's default): succeeds, and the open reader still reads the old bytes. A plain rename leaves the target
  with the temp file's id, as on Unix.
- The runner process has a console (`AllocConsole` is refused, access denied). Ctrl+Break sent with
  `GenerateConsoleCtrlEvent` to a child started with `CREATE_NEW_PROCESS_GROUP` reaches it: with a
  `SetConsoleCtrlHandler` handler, the handler ran its cleanup and `ExitProcess(STATUS_CONTROL_C_EXIT)` gave
  the parent `code() == Some(-1073741510)`; with no handler, the default handling gives the same code.
