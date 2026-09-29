---
blocked_by: []
status: resolved
title: CI validates the repository's own typdoc projects
type: implementation
---

# TK-1: CI validates the repository's own typdoc projects

## What this delivers

A job in `ci.yml` that builds typdoc from the pull request and runs `validate` at the repository root and in
`.chief/`, so a broken ref in either project turns the pull request red.

## Scope (contract: CI)

- `ci.yml` only; standard mode.
- Both projects pass today (exit 0, no finding).

## Checks

- Red once: a throwaway pull request with a deliberately broken ref in each project; the job names the finding.
- Green on the story's pull request.

## Result

Job `typdoc validate (repository projects)` in `ci.yml`. Shown on a throwaway pull request: red with a broken ref at the root (step "validate the project at the repository root"), red with a broken ref only in `.chief/` (step "validate the project in .chief"), green once both were removed. Green on the story's pull request.
