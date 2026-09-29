---
blocked_by:
- TK-12
status: open
title: match and ignore refuse glob syntax they do not take
type: implementation
---

# TK-13: match and ignore refuse glob syntax they do not take

## What this delivers

A collection's `match` and the body link `ignore` refuse `?`, `[..]`, `\`, `!` and `,` in an entry as a config
error that names the pattern, as `namespaces` does; before, they read them literally and matched nothing,
silently (G1, TK-11).

## Scope

- The config check of `match` and of `body.links`'s `ignore`, and the glob spec's subset table. Strict mode.
- A test for each refused character in each place, and a planted pattern (`notes/q?.md`) that loaded before and
  now fails with the pattern named.
- CHANGELOG `[Unreleased]` Changed (TK-10).

## Checks

Gates; every `fixtures/valid` project still loads; the planted pattern fails.
