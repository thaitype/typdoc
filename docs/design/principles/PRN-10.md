---
title: The spec states what the code does today
status: active
---

**The spec says what the code does today.** Design that is not built yet, or that the code does
differently, is not written into the spec as if it held.

## Why

The spec is where code comments, tests and people look for the reason behind a behavior. A spec
that describes intentions mixed with facts cannot be trusted for either: a reader cannot tell which
sentences the binary honours.

## What follows

- Design moved into the spec is rewritten to what the code does, and a difference between the old
  text and the code is listed for a decision instead of being settled silently
  (`.chief/_rules/_standard/design-docs.md`).
- A change to behavior updates the spec in the same pull request.

## Where it stops

A spec may name what is known to be missing ("`pull` is not built"), as long as it says so.
