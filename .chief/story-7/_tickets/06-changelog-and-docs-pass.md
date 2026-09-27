# 06: Changelog, upgrade note, and a docs pass

Type: implementation
Status: open
Blocked by: 01, 02, 03, 04, 05

## What this delivers

A user reading the changelog, the docs or the skill learns slugs end to end, and a user
upgrading learns the three ways a project that passed can now fail.

## Scope (contract: Documentation in the same PR)

- `CHANGELOG.md` `## [Unreleased]`: slugs, `new --slug`, `slug`, `refs.slug`, and the upgrade
  note with the three cases of `SPC-17` (A project from before slugs).
- `docs/explanation/keys-and-numbers.md`: the key identifies, the slug is for readers.
- A read-through of every doc and skill file touched by 01–05 for consistency, and a check that
  no text about names of documents without a code changed.
- Output: a `--json` assertion that `key` has no slug and `path` has it, if 01–05 did not
  already add one.

## Checks

Gates; `typdoc validate`; the public-text check over the whole branch diff.
