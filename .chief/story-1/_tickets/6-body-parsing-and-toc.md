# 6: Body parsing, line and column mapping, slugs, and `typdoc toc`

Type: implementation
Status: resolved
Blocked by: 1, 3

## What this delivers

- `pulldown-cmark` 0.13.4 with `default-features = false`, the frontmatter cut first and its offset added back; our own mapping of offsets to line and column, counted as the design's line rule says, and our own slugger.
- Headings with `level`, `text`, `slug`, `line` and `end`; `typdoc toc <path> [--depth n] --json` as JSON output describes, ordered by `line`.

## Done when

- Fixtures for a Thai line and an emoji line with the expected `col`, a file with no final line ending, `\r\n` endings, a lone `\r`, a heading with nothing under it, an empty slug and duplicates.
- Slug fixtures built from GitHub's renderer (ticket 4); a golden for `toc`.
- `toc` leaves `unimplemented_commands`.
