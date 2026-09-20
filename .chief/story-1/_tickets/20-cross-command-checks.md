# 20: Checks across all commands

Type: implementation
Status: open
Blocked by: 6, 12, 14, 17, 19

## What this delivers

- The registry checked against the goldens and against the headings of the Commands section; every exit code of the design's table produced by a test or listed in `unproduced_exit_codes`; the round trip of a printed name into `get`; `examples/` validated as a user receives it.

## Done when

- The lists contain exactly what the story does not build (`new`, `set`, `mv`, `pull`, exit codes 3 and 4, `frontmatter.transitions`), and exit code 6 is either produced by a deterministic test or listed.
