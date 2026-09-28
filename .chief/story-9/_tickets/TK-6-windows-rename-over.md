---
blocked_by: []
status: resolved
title: What does replacing a file by rename do on Windows
type: wayfinder:research
---

# TK-6: What does replacing a file by rename do on Windows

## Question

On Windows, what does `std::fs::rename` (1.96) do when the destination exists: which system call, whether it is atomic, whether it fails when another process has the destination open (and with which share modes), what happens to a read-only destination, and which attributes and ACL the renamed file keeps (its own, inherited from the folder, or the replaced file's). Also what `ReplaceFileW` would keep instead.

## Answer

Sources: Rust std at `1.96.0` (`library/std/src/sys/fs/windows.rs`), learn.microsoft.com, MS-FSA, MS-FSCC.

- `std::fs::rename` calls `MoveFileExW(MOVEFILE_REPLACE_EXISTING)`; only on access denied does it retry with
  `SetFileInformationByHandle(FileRenameInfoEx)` and POSIX semantics, without the flag that ignores a
  read-only destination. Renaming over a read-only file fails (access denied). Over a file another process
  holds open, the plain rename fails; the POSIX retry can replace it when that process shares delete.
- The renamed file keeps its own security: a security descriptor is assigned at creation, not on rename. So
  today's temp-then-rename gives a replaced document the folder's inherited ACL and drops the old file's ACL,
  attributes and streams.
- `ReplaceFileW` keeps the replaced file's creation time, short name, object ID, DACL, encryption,
  compression and extra streams. Its limits: not atomic (errors 1176 without a backup name and 1177 leave
  nothing at the document's path); same volume only; the replacement must be closed; keeping the DACL needs
  `WRITE_DAC`, and the ignore flags drop it silently; owner and SACL are not in its list; no write-through;
  the file id is not kept; the target must exist, so a first write needs another path; read-only targets,
  ReFS, network shares, FAT/exFAT and links are not documented.
- Keeping a DACL without it: `GetSecurityInfo` on the old file, `SetSecurityInfo` on the temp before the
  rename (needs `READ_CONTROL` and `WRITE_DAC`; the owner usually cannot be copied by an ordinary user).
- `Permissions::readonly`/`set_readonly` are the read-only attribute alone.
- Not verified, to be measured (TK-12): a read-only target; a reader holding the target open with and
  without share-delete; `ReplaceFileW` on the runner.
