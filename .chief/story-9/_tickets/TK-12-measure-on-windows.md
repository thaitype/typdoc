---
blocked_by:
- TK-5
- TK-6
- TK-7
status: open
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
