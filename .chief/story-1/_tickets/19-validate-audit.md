# 19: `typdoc validate --audit`

Type: implementation
Status: resolved
Blocked by: 9, 10, 11, 18

## What this delivers

- Audit mode: rules set to `off` reported as `info`, exit 0 unless the config is invalid, `summary.audit` and `summary.unreported`, and the `audit` object with `collections`, `uncollected` and `no_frontmatter`; a file with no frontmatter is listed and not evaluated.
- The text form of an audit.

## Done when

- The accounting invariant of the contract holds on every project in the fixtures, checked against an independent count.
- A test shows that `validate` and `--audit` treat a file with no frontmatter differently and that `no_frontmatter` never overlaps `uncollected`.
