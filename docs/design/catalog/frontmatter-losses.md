---
title: Frontmatter losses catalog
content_type: json
explained_by: SPC-4
---

{
  "losses": [
    "Comments, anywhere in the block",
    "Blank lines between fields",
    "`tags: [a, b]`",
    "`title: 'Ship it'`, `status: \"no\"`",
    "`id:   WF-3`",
    "`&anchor` with `*alias`",
    "`!!str`, `!Ref`, any other tag"
  ]
}
