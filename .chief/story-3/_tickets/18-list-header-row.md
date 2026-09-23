# 18: `list`'s header row

Type: implementation
Status: claimed
Blocked by: None (can start immediately)

M-10g's answer, built. `list` already produces text output (`list_outcome`); this ticket adds
one thing to it.

## The work

Add a header row above `list`'s existing table: `path` (or `key`, for a coded collection), then
`title`, then each `--where` field, in that order — matching the column order the table's rows
already use. No header when the result is empty (exit 0, as today — the empty case is unchanged,
not given a header-only line). `list --ids` is unchanged: one key or path per line, no header.

## Tests

- Existing golden tests for `list_outcome` are updated for the header row.
- An empty-result case (already covered today) is re-confirmed to print nothing, not a
  header with no rows under it.
- `list --ids` output is unchanged, confirmed by its existing tests still passing without
  modification.

## Done

- `list`'s text output has the header row described above, in every non-empty case, for both a
  coded and a path-identified collection.
- `--ids` behavior is bit-for-bit unchanged.
