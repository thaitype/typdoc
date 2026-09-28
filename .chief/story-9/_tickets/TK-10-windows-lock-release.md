---
blocked_by:
- TK-5
- TK-9
status: open
title: 'Lock release on Windows: is the identity check as strong as on Unix'
type: wayfinder:grilling
---

# TK-10: 'Lock release on Windows: is the identity check as strong as on Unix'

## Question

Given TK-5 and TK-9: does a lock's release on Windows tell its own file from another process's as surely as `stat`/`fstat` do on Unix, including a lock deleted as stale by a user while held; and if not, what is weaker and is the user told. Decides the Windows forms of `identity`, `identity_at` and `same_file`.

## Answer
