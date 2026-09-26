# Comments

Follow the `comment-review` skill, including its `references/patterns.md`. This file adds only
what is specific to this repository.

## Where design reasoning lives

This repository has an authoritative place for design reasoning, so patterns.md §2a applies.

- The authoritative place is `docs/design/`: `spec/` (`SPC-n`) and `catalog/`. A comment whose
  reasoning is design cites the SPC by key (`SPC-7`), never by path or heading. A catalog entry
  is cited through the SPC named in its `explained_by`.
- Not authoritative, never cited: `.chief/` (a story's record), `docs/migrating-design/` (a
  working copy being emptied), `docs/archived-design/` (frozen). Reasoning found only there is
  moved into an SPC first, as `design-docs.md` describes, and then cited.

## Where a citation goes

- A test file that covers an SPC says so once, at its top (`//! Covers SPC-7.`). A test file
  that covers none says nothing.
- Source cites an SPC only where the code would otherwise be changed wrongly. Do not tag every
  item.

## Public API

patterns.md §6 treats doc comments on public API differently. Here, public API means the items
that appear on docs.rs for `typdoc-core`, `typdoc-fs` and `typdoc`: public, reachable from the
crate root, not `#[doc(hidden)]`, plus each crate's `//!`. When unsure, `cargo doc --no-deps`
decides. `typdoc-testkit` is `publish = false`, so none of its items count.
