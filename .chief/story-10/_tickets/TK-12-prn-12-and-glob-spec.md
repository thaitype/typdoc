---
blocked_by:
- TK-2
- TK-11
status: open
title: PRN-12 and one glob spec
type: implementation
---

# TK-12: PRN-12 and one glob spec

## What this delivers

PRN-12, "a name points at one thing; a pattern selects many", and one spec for the glob syntax (`*`, `**`, `,`,
`!`, `\`, the leading-dot rule). SPC-1, SPC-7, SPC-13 and SPC-17 point to it and state only which subset they take,
and why. The spec states what the code does; a difference the owner decides to remove is changed in code in its own
ticket.

## Scope

Standard mode. Spec and principle only.

## Checks

The design-document tests and `typdoc validate` of the repository's own project pass.
