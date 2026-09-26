# Goal

Every comment in `crates/` keeps only reasoning the code, types, names, and tests cannot show,
and cites design by SPC key instead of by history.

From a reader's perspective, when the story ends:

- A comment explains a constraint, an invariant, an assumption, or an intent that is not visible
  in the code. Comments that narrate what the code does are gone.
- No comment in `crates/` points at history: no ticket, decision, or `M-N` numbers, no story
  paths, no `.chief/`, no `docs/design.md`, `docs/migrating-design/` or `docs/archived-design/`.
- Where a comment's reasoning is design (what typdoc does, or why), that design is in
  `docs/design/` and the comment cites it by key (`SPC-7`). A design section that a comment
  cited and that lived only in `docs/migrating-design/` or `.chief/` has moved into an SPC,
  describing what the code does today, and is deleted from `docs/migrating-design/`.
- A test file that covers an SPC says so once, at its top (`//! Covers SPC-7.`). Source cites
  an SPC only where the code would otherwise be changed wrongly.
- Doc comments on items that appear on docs.rs are still there, possibly shorter, unless they only
  restated the item's name or its `#[error]` message, which docs.rs shows anyway.
- Every place where a comment and the code disagree is listed, with what the comment claims,
  what the code does, and where, and has a decision. None of them is fixed in this story.
- The two rules that make this hold for later changes are in `.chief/_rules/_standard/`:
  a new section in `design-docs.md`, and a new `comments.md`.

No behavior changes. A Rust change in this story is a comment change only.

## Out of Scope

- Fixing any bug or mismatch the review uncovers. It is listed for a decision, not fixed.
- A checker that validates SPC keys cited in source. Citations are checked by reading, at the
  end of each batch.
- Moving `docs/migrating-design/` sections that no comment cites.
- Comments outside `crates/` (`scripts/`, workflows, docs).
- A release or a version bump.
