---
blocked_by: []
status: resolved
title: 'CI runs once per change: push to main and pull requests'
type: implementation
---

# TK-3: CI runs once per change: push to main and pull requests

## What this delivers

A pull request's branch is built once, by the pull request, not a second time by its push.

## Scope (contract: part 2, CI triggers)

- `ci.yml` and `publish-check.yml`: `push` limited to `main`; `pull_request` to `main` as before.
- No other workflow runs on a push to a branch other than `main` (release branches publish through
  `workflow_dispatch`, tags through `release-rc.yml`).

## Checks

The next push to this pull request's branch starts one run per workflow, from `pull_request`.
`newest release tag is in main` keeps its `push` to `main` condition.
