# 02: Changelog and docs

Type: implementation
Status: open
Blocked by: 01

## What this delivers

A user reading the changelog, the docs or the skill learns the five endings, and a user of the
`typdoc-core` library learns that `acquire` and `Env` changed.

## Scope (contract: Documentation in the same PR)

- `CHANGELOG.md`: `## [Unreleased]` with Fixed and Changed.
- `templates/skills/typdoc/references/exit-codes.md`, rendered with `scripts/render_skills.py`;
  any user doc that lists the endings.
- `.chief/project.md`: `libc` as an ordinary dependency of the binary crate.

## Checks

Gates; the render check; the public-text check over the branch diff.
