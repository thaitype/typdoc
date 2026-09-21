# 8: `yaml-edit` on anchors, tags, and the values story 1 keeps as text

Type: wayfinder:prototype
Status: open
Blocked by: None (can start immediately)

## Question

The write half of the frontmatter round trip rests on `yaml-edit`, pinned to an exact 0.3.x and restricted to three operations: set a scalar, append or remove a list item, add a key. It was chosen because it was the only crate in the survey that preserved comments, quote style and flow style, with a byte-identical no-op round trip. Two things were never probed, and were written down as work for a prototype:

- **Anchors and aliases.** A frontmatter block with `&anchor` and `*alias` in it. What happens when a field that is an alias is set? When a field elsewhere in the document anchors a value that the edited field refers to? An editor that expands an alias while setting an unrelated key has rewritten a part of the file nobody asked it to touch.
- **Tags.** `!!str`, `!!binary`, and a custom tag such as `!Ref`. Story 1's ticket 28 made a tagged value read as text rather than fail, so documents with tags now pass through the read path and reach the write path.

Two more cases have joined them since:

- **Integers past 64 bits**, which story 1 also made read as text. A `set` on a neighbouring field must leave such a line byte-identical.
- **The block scalar fusion** already known from the first survey: replacing a block scalar fuses two lines (`note: newb: 2`). The reparse guard catches it. Establish whether it is the only such case or the first one found.

Build the smallest thing that answers these: a handful of real frontmatter blocks carrying anchors, aliases, tags, a block scalar, a long integer, a flow list and comments, each edited with one of the three operations, with the result compared byte for byte against what was intended. Record what `yaml-edit` does in each case, and whether the reparse guard catches it when the result is wrong.

The point is not to grade the crate. It is to know which of these cases the guard turns into a rejected write, because every one of those is a `set` that a user cannot complete, and the contract has to say so rather than let them find out.

## Answer

<filled in on resolve>
