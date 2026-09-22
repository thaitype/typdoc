# 5: Writing frontmatter

Type: implementation
Status: open
Blocked by: 1

## What this delivers

- `yaml_serde` writes the frontmatter block, from the text the read path already keeps (decision 20). A write rewrites the whole block; there is no line editing and no re-read guard.
- The writer sits behind the trait phase 1's ticket 1 put in front of it, so the choice can be revisited without touching call sites.
- The losses the design's table names are what is lost, and nothing else: comments, blank lines, flow style, quote style, spacing, anchors, aliases and tags.
- The fixtures are extended until they carry every shape in that table.

## Done when

- The round trip of the contract's first criterion runs over every document in the fixtures: a `set` of one field leaves every other field reading back exactly as before, and no value differs.
- The table of losses is read from `docs/design.md` and compared with the corpus, so a shape in the table that no fixture holds is a gap the suite reports.
- A value the format holds exactly is written back exactly, including every literal YAML would otherwise reinterpret.
