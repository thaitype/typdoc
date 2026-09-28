---
blocked_by:
- TK-5
status: open
title: 'File identity width: support ReFS 128-bit ids or not'
type: wayfinder:grilling
---

# TK-9: 'File identity width: support ReFS 128-bit ids or not'

## Question

`FileId.inode` is a `u64`. ReFS file ids are 128 bits. Support ReFS (widen the core type, a change to a published crate's public API) or not, and if not, what typdoc does on ReFS: refuse to write there, or compare the low 64 bits and accept a collision it cannot rule out. Owner's decision, through the director.

## Answer
