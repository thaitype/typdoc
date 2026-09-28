---
blocked_by:
- TK-8
- TK-9
- TK-10
- TK-12
status: claimed
title: Writes on Windows keep the guarantees they keep on Unix
type: implementation
---

# TK-13: Writes on Windows keep the guarantees they keep on Unix

## What this delivers

`new`, `set` and `mv` write on Windows. A lock is released only while it is still this process's own; a
replaced document keeps its DACL and its read-only flag; a replace never leaves the name missing; a document
another program holds open is not replaced, and the message says so.

## Scope (map: TK-8, TK-9, TK-10, TK-12)

- `FileId::inode` is a `u128` (`typdoc-core`, CHANGELOG `[Unreleased]` Changed).
- `typdoc-fs`: `unix` and `windows` modules. Windows: identity from `FileIdInfo` and `FileStandardInfo`
  (delete-pending is no links); a path that cannot be opened, not found or access denied, has no identity; mode
  is the read-only flag; `rename` carries the destination's DACL to the source, then renames with POSIX
  semantics and ignore-read-only; a DACL that cannot be carried, or a sharing violation, stops it with its
  message.
- `windows-sys` as a Windows-only dependency.
- Tests: `write_seam`'s Windows mode; `windows_permissions.rs` (DACL and read-only kept, DACL not carried,
  held open).

## Checks

Every lock and write test passes on `windows-latest`; Linux and macOS counts unchanged.
