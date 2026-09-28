---
blocked_by:
- TK-5
- TK-9
- TK-12
status: resolved
title: 'Lock release on Windows: is the identity check as strong as on Unix'
type: wayfinder:grilling
---

# TK-10: 'Lock release on Windows: is the identity check as strong as on Unix'

## Question

Given TK-5 and TK-9: does a lock's release on Windows tell its own file from another process's as surely as `stat`/`fstat` do on Unix, including a lock deleted as stale by a user while held; and if not, what is weaker and is the user told. Decides the Windows forms of `identity`, `identity_at` and `same_file`.

## Answer

Decided, from TK-5 and TK-12: a Windows file's identity is its `FileIdInfo` (64-bit volume serial, 128-bit file
id), read from a handle. A lock is still this process's own while its handle reports links above zero and no
delete pending, and the path, opened with share-all, has the same identity. A path that cannot be opened (not
found, or access denied under the older delete semantics) is not ours, and nothing is removed. On the runner a
lock removed by another process while held shows links 0 and delete-pending on the held handle, and a new lock
at the path has another id, so this is as strong as on Unix. The id's width waits on TK-9.
