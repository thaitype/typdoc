---
blocked_by: []
status: resolved
title: PRN-11 and the name grammar in one spec
type: implementation
---

# TK-2: PRN-11 and the name grammar in one spec

## What this delivers

PRN-11 and one spec for the name grammar, which SPC-2 and SPC-14 point to instead of restating it.

## Scope (contract: Spec)

- `docs/design/principles/PRN-11.md` from the draft in the brief.
- A new spec, the name grammar, `follows: [PRN-11]`: the context table, the reading order, the three forms, the round trip.
- SPC-2, SPC-14 point to it; SPC-12 gains `ref`. Standard mode.

## Checks

The design-document tests and `typdoc validate` of the repository's own project pass.
