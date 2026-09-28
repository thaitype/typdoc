---
blocked_by: []
status: resolved
title: How can Ctrl+C be caught on Windows, and how is it tested
type: wayfinder:research
---

# TK-7: How can Ctrl+C be caught on Windows, and how is it tested

## Question

On Windows, how a console process learns of Ctrl+C and Ctrl+Break (and console close), from which thread its handler runs and what it may do there, whether `signal-hook` 0.4 supports any of it, and how a test can deliver Ctrl+C to a child process it spawned (`GenerateConsoleCtrlEvent`, process groups, a new console) on a GitHub `windows-latest` runner.

## Answer

Sources: learn.microsoft.com (console handlers, `GenerateConsoleCtrlEvent`, MS-ERREF), `signal-hook` 0.4.4 and
`ctrlc` 3.5.2 source, `windows-sys` 0.61.2.

- `SetConsoleCtrlHandler`: the system runs the handler on a new thread; returning true for Ctrl+C or
  Ctrl+Break keeps the process running; console close allows 5 seconds. `windows-sys` alone covers it
  (`Win32_System_Console`, `Win32_System_Threading`).
- `signal-hook`'s `iterator` is not built on Windows; its re-raise goes through the C runtime, which exits
  with code 3.
- A test cannot aim Ctrl+C at one child: `CTRL_C_EVENT` goes to every process on the console, the runner
  included. `CTRL_BREAK_EVENT` can be aimed at a child started with `CREATE_NEW_PROCESS_GROUP`.
- Windows has no "ended by a signal": after cleanup the handler would end the process with
  `ExitProcess(STATUS_CONTROL_C_EXIT)` (0xC000013A), and a parent sees `code() == Some(-1073741510)`.
- Not verified, to be measured (TK-12): Ctrl+Break to a child in its own process group on the runner; the
  exit code a default-handled Ctrl+C gives.
