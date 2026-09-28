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

Second probe (run 36369483540, same runner image, NTFS): a rename through the file's own handle,
`SetFileInformationByHandle(FileRenameInfoEx)` with replace-if-exists, POSIX semantics and ignore-read-only.

- Over a read-only target: succeeds. With the temp file made read-only first, the result is read-only, so a
  carried read-only flag survives the replacement. Without the ignore flag: access denied (5).
- Over a target another handle holds open without share-delete: fails with os error 32 (sharing violation),
  which tells it apart from access denied; the target keeps its bytes. With share-delete: succeeds.
- 2000 replacements while a reader opened the target in a loop (about 25 000 opens): the reader never found
  it missing, for this rename and for `std::fs::rename` alike. Evidence, not proof, of atomicity.

Third probe (run 36404943850, runner process in Administrators at high integrity, NTFS): the replaced file's
DACL copied onto the temp file (`GetSecurityInfo` through a handle opened for `READ_CONTROL`, `SetSecurityInfo`
through one opened for `READ_CONTROL | WRITE_DAC`, protected or not as the old DACL was), then the POSIX rename.

- A restrictive protected DACL (`D:PAI(A;;FA;;;BA)(A;;FA;;;SY)(A;;FR;;;WD)`) is the result's DACL, exactly; with
  the read-only flag carried too, the result is read-only as well.
- An inherited DACL: the result has the same inherited entries, and gains the auto-inherited flag (`D:AI`), so
  it grants the same access.
- The owner denied reading the old DACL: the old file cannot be opened for `READ_CONTROL` (os error 5), so the
  copy stops and nothing is replaced. The owner denied changing the temp's DACL: the temp cannot be opened for
  `WRITE_DAC` (5), the copy stops, and the old file keeps its bytes.
- `SetSecurityInfo` needs `READ_CONTROL` on the handle as well as `WRITE_DAC`: with `WRITE_DAC` alone it
  refused every change (first run, 36404783071).
