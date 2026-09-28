---
blocked_by: []
status: resolved
title: An absolute path on this system is a path, not a namespace prefix
type: implementation
---

# TK-11: An absolute path on this system is a path, not a namespace prefix

## What this delivers

On Windows, `typdoc get C:\notes\a.md` reads that file; it no longer reads `C` as a namespace prefix.

## Scope

- `Argument::parse`: an argument that is absolute on this system (`Path::is_absolute`) is on disk, checked
  before any `project::` or `namespace:` prefix is read. On Unix that is the leading `/`, already on disk.

## Checks

Unit test under `cfg(windows)` (`C:\…`, `C:/…`, `\\server\share\…`); `get.rs`
`an_absolute_path_names_the_project_by_itself_and_wins_over_typdoc_dir` passes on Windows; Linux count unchanged.
