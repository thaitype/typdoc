---
blocked_by:
- TK-16
- TK-17
status: open
title: Planted faults in the Windows installer and build turn CI red
type: implementation
---

# TK-20: Planted faults in the Windows installer and build turn CI red

## What this delivers

Evidence that the Windows installer's checks can fail: on a throwaway pull request, closed after, the installer
without its checksum check, and without its missing-asset message, each turn the Windows installer job red.

## Checks

Each planted fault turns its scenario red; the pull request is closed and its branch deleted.
