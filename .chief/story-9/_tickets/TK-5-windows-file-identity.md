---
blocked_by: []
status: open
title: How can a Windows file's identity and link count be read, and what do they promise
type: wayfinder:research
---

# TK-5: How can a Windows file's identity and link count be read, and what do they promise

## Question

On Windows, with the Rust toolchain this repository pins (1.96, stable) and `windows-sys` (0.61, already in `Cargo.lock`): which calls give a file's volume, file id and link count from a path and from an open handle (`GetFileInformationByHandle`, `GetFileInformationByHandleEx` with `FileIdInfo`, ...), how wide each id is on NTFS and on ReFS, and what the link count and a path lookup report after the file is deleted while a handle is still open (POSIX delete semantics against the older delete-pending behaviour, by Windows version and file system). Also: can another process create a new file at that path while the old one is delete-pending.

## Answer
