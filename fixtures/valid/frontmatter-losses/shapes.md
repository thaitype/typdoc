---
title: 'Ship it'
subtitle: "already quoted"
# a comment inside the block, gone after any write
padded:    has extra spaces after the colon

tags: [alpha, beta]
original: &shared shared text
mirrored: *shared
typed: !Ref other-doc
---

# Every shape a write does not keep

One document holding a comment, a blank line, a flow list, both quote styles, extra spacing
after a colon, an anchor with its alias, and a tag, so the round-trip check has every shape
`docs/design.md` names something to hold against.
