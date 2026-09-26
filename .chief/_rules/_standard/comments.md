# Comments

A comment keeps reasoning the code, types, names, and tests cannot show. It does not narrate
what the code does. Decide each comment with the `comment-review` skill: keep, reduce, remove,
or investigate.

- `//` comments, and doc comments on items that do not appear on docs.rs: the skill applies in
  full.
- Doc comments on items that appear on docs.rs: may be shortened, never emptied.
- Cite design by SPC key, as in `design-docs.md`. A test file that covers an SPC says so once,
  at its top (`//! Covers SPC-7.`); a test file that covers none says nothing. Source cites an
  SPC only where the code would otherwise be changed wrongly. Do not tag every item.
- No history: no ticket, decision, or brief numbers, no dates, no story paths.
- A comment that disagrees with the code is not fixed in either direction during a cleanup; it
  is listed for a decision.
