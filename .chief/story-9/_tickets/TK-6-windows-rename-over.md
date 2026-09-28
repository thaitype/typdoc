---
blocked_by: []
status: open
title: What does replacing a file by rename do on Windows
type: wayfinder:research
---

# TK-6: What does replacing a file by rename do on Windows

## Question

On Windows, what does `std::fs::rename` (1.96) do when the destination exists: which system call, whether it is atomic, whether it fails when another process has the destination open (and with which share modes), what happens to a read-only destination, and which attributes and ACL the renamed file keeps (its own, inherited from the folder, or the replaced file's). Also what `ReplaceFileW` would keep instead.

## Answer
