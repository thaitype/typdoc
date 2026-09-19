# 11: What unit is `col` in `path:line:col`?

Type: wayfinder:grilling
Status: open
Blocked by: None (can start immediately)

## Question

`validate` prints `path:line:col` and `--json` returns the same fields. The parser gives byte offsets; a column can be counted in bytes, Unicode scalar values (chars), UTF-16 code units, or grapheme clusters. They differ on non-ASCII text: on a Thai line, research 3 measured the same link at column 5 by chars and column 11 by bytes.

Decide the unit, given that documents here are often Thai and that findings are meant to be clickable in editors and consumed by agents. Which convention common editors use for `path:line:col` was not verified in research 3; check it before deciding. Amend `docs/design.md` so the unit is stated once.

## Answer

