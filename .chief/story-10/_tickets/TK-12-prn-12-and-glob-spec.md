---
blocked_by:
- TK-2
- TK-11
status: resolved
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

## Decided before the spec (TK-11)

The spec states G2 to G5's reasons, and G1 as `match` and `ignore` refusing what they do not take (built in
TK-13). G6: SPC-1 and `docs/reference/project-files.md` say that `ignore` matches the target's path from the
project folder, with `**/assets/**` as the example.
