# 8: `typdoc validate`: the report, levels, and the rules on frontmatter

Type: implementation
Status: resolved
Blocked by: 2, 3, 5, 7

## What this delivers

- The report of `validate` as JSON output describes: `summary` (`scope`, `strict`, `checked`, `findings`) and `findings` in the guaranteed order; exit 0, or 2 when a finding is an error.
- Rule levels merged from the defaults, `validation.global` and the collection; `--strict`; arguments giving `scope: paths`; `--schemas` giving `scope: schemas`; `--schemas` or `--audit` with arguments exits 1.
- The rules `frontmatter.parse`, `frontmatter.types` and `frontmatter.unknown`, with the empty block treated as frontmatter and a file with no block as not; the position of a `frontmatter.parse` finding as the contract says.

## Done when

- Each rule leaves `unimplemented_rules` and has fixtures in `broken/` with the exact expected set.
- A report for one document and a report for the whole project differ in `scope`, and `checked.paths` matches the `path` of the findings.
- `validate` leaves `unimplemented_commands`.
