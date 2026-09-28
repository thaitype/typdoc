---
blocked_by: []
status: open
title: How can Ctrl+C be caught on Windows, and how is it tested
type: wayfinder:research
---

# TK-7: How can Ctrl+C be caught on Windows, and how is it tested

## Question

On Windows, how a console process learns of Ctrl+C and Ctrl+Break (and console close), from which thread its handler runs and what it may do there, whether `signal-hook` 0.4 supports any of it, and how a test can deliver Ctrl+C to a child process it spawned (`GenerateConsoleCtrlEvent`, process groups, a new console) on a GitHub `windows-latest` runner.

## Answer
