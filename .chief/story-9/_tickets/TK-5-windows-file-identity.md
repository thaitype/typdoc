---
blocked_by: []
status: resolved
title: How can a Windows file's identity and link count be read, and what do they promise
type: wayfinder:research
---

# TK-5: How can a Windows file's identity and link count be read, and what do they promise

## Question

On Windows, with the Rust toolchain this repository pins (1.96, stable) and `windows-sys` (0.61, already in `Cargo.lock`): which calls give a file's volume, file id and link count from a path and from an open handle (`GetFileInformationByHandle`, `GetFileInformationByHandleEx` with `FileIdInfo`, ...), how wide each id is on NTFS and on ReFS, and what the link count and a path lookup report after the file is deleted while a handle is still open (POSIX delete semantics against the older delete-pending behaviour, by Windows version and file system). Also: can another process create a new file at that path while the old one is delete-pending.

## Answer

Sources: learn.microsoft.com (Win32, WDK, MS-FSCC, MS-FSA), Rust std at `1.96.0`, `windows-sys` 0.61.2.

- Identity comes from an open handle only: `GetFileInformationByHandleEx(FileIdInfo)` gives a 64-bit volume
  serial and a 128-bit file id (Windows 8 and later); `FileStandardInfo` gives `NumberOfLinks` and
  `DeletePending`. For a path, open a handle with access 0 and share read/write/delete (what std's
  `fs::metadata` does) and ask that. std's own accessors are unstable (`windows_by_handle`).
- Width: NTFS ids are 64 bits (the high half of the 128-bit form is zero). ReFS ids are 128 bits, and its
  64-bit index is documented as not unique (set to -1 when the id does not fit). Compare the 64-bit volume
  serial with the 128-bit id.
- A file deleted while a handle is open: `DeletePending` is true on the handle, and `NumberOfLinks` counts
  non-deleted links (so 0 for a lock file, by the spec). With the older semantics the name stays until the
  last handle closes and opening it fails with access denied; with POSIX semantics (Windows 10 1607 and
  later) the name goes at once and opening it is not found.
- std 1.96 opens files sharing delete, and `remove_file` calls `DeleteFileW`, falling back to a POSIX delete
  through a handle only on access denied.
- Not verified, to be measured (TK-12): which Windows versions make `DeleteFileW` POSIX by default; the link
  count a real NTFS handle reports after the delete; the error `CREATE_NEW` gets while the old name is
  delete-pending.
