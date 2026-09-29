---
blocked_by:
- TK-5
- TK-7
- TK-12
status: resolved
title: The docs and the skill say how names are written
type: implementation
---

# TK-8: The docs and the skill say how names are written

## What this delivers

The how-to, the command reference and the skill say how names are written, one way.

## Scope (contract: Spec)

- `docs/how-to/use-namespaces.md`, `docs/reference/commands.md`, `templates/skills/typdoc/references/project-layout.md` and the rendered skill. Standard mode.

## Checks

`render_skills.py --check`; the doc examples run.
