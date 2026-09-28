---
blocked_by:
- TK-5
status: resolved
title: 'File identity width: support ReFS 128-bit ids or not'
type: wayfinder:grilling
---

# TK-9: 'File identity width: support ReFS 128-bit ids or not'

## Question

`FileId.inode` is a `u64`. ReFS file ids are 128 bits. Support ReFS (widen the core type, a change to a published crate's public API) or not, and if not, what typdoc does on ReFS: refuse to write there, or compare the low 64 bits and accept a collision it cannot rule out. Owner's decision, through the director.

## Answer

Decided by the owner: `FileId` is widened to hold a 128-bit file id, so ReFS is supported and NTFS ids fit as
they are. It changes `typdoc-core`'s public API, so `CHANGELOG.md` records it under `[Unreleased]`, Changed.
