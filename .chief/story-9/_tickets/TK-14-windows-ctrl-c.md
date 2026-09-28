---
blocked_by:
- TK-7
status: claimed
title: Ctrl+C on Windows removes the locks and ends with STATUS_CONTROL_C_EXIT
type: implementation
---

# TK-14: Ctrl+C on Windows removes the locks and ends with STATUS_CONTROL_C_EXIT

## What this delivers

Ctrl+C, Ctrl+Break or closing the console while typdoc holds a lock removes the lock and ends the process with
`STATUS_CONTROL_C_EXIT`, as SPC-3 says for Windows.

## Scope (map: TK-7)

- `signals.rs`: `SetConsoleCtrlHandler`, the handler releases every held lock and calls `ExitProcess`;
  `signal-hook` becomes Unix only.
- Tests: `console_interrupt.rs`, sending Ctrl+Break to a child started in its own process group.

## Checks

The interrupt tests pass on `windows-latest`; `signals.rs` still passes on Linux and macOS.
