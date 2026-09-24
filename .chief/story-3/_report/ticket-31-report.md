# Ticket 31 Report

## Ticket

M-19: `--set`/`field=value` did not implement design §Query's escaping grammar at all — `\*`
was kept literally instead of unescaping to `*`, and a bare unescaped `*` was silently accepted
instead of refused (`set` has no wildcard concept, unlike `--where`/`--if`).

## Outcome

done

## The fix

Added `unescape_set_value(raw: &str, split_on_comma: bool) -> Result<Vec<String>, Error>` in
`crates/typdoc-core/src/project.rs`, next to `apply_ops`. It walks `raw` once, char by char,
mirroring `query.rs`'s `parse_value` scanning shape but with a different bare-`*` policy: `\*`,
`\,`, `\\` unescape to their literal character; any other `\x` or a trailing lone `\` is
`Error::BadArgument`; a bare `*` is always `Error::BadArgument` (no wildcard/glob fallback, since
`set` has none). When `split_on_comma` is true (a list/`ref[]` field) an unescaped `,` starts a
new item; when false (a scalar field, nothing to split into) it stays a literal comma and the
whole value comes back as the single returned item.

`apply_ops` (same file) is now `fn apply_ops(..) -> Result<(), Error>`. It calls
`unescape_set_value(raw, is_list)?` once per `SetOp::Set`, using its result either as the list
items (`writer.set_list`) or the single scalar value (`writer.set_scalar`). All three call sites
found by `grep -rn "apply_ops("` were updated to propagate with `?`:
- `Project::set_loose` (`apply_ops(&mut writer, sets, &no_schema)?;`)
- `Project::set_collected` (`apply_ops(&mut writer, sets, schema)?;`)
- `new_block` (shared by both `new` code paths) — its own signature changed from
  `fn new_block(schema, sets, now) -> Result<String, String>` to
  `fn new_block(schema, sets, now, file: &Path) -> Result<String, Error>`, since it needed a
  `file` to build `Error::Frontmatter` for `writer.finish()`'s own error once `apply_ops`'s
  `Error::BadArgument` was propagated directly (no longer squashed into a `String`). Both
  `new_block` call sites (`new_coded`/uncoded `new` paths, around what were lines 1103 and 1215)
  were simplified from a `.map_err(|message| Error::Frontmatter {...})?` wrapper to a plain `?`,
  since `new_block` now returns the right `Error` variant itself.

Validation happens at write time (once the field's schema/list-vs-scalar-ness is known), not at
CLI-parse time in `parse_set_op` — the schema isn't resolved yet at that point (per the ticket's
documented constraint), so `parse_set_op` still just stores `raw` unprocessed.

## Verification

**Red-then-green, both repro cases** (`crates/typdoc/tests/set.rs`):
- `a_backslash_star_in_a_scalar_value_is_stored_as_a_literal_star` (`title=a\*b` → literal `a*b`,
  checked in the returned JSON and by reading the file back): failed against the pre-fix code
  (`left: "a\\*b"` vs `right: "a*b"`), passed after the fix.
- `a_bare_unescaped_star_in_a_set_value_is_refused_and_writes_nothing` (`title=x*y`): failed
  against the pre-fix code (exit 0 instead of the expected 1, value silently accepted), passed
  after the fix (exit 1, file unchanged).

Ran `cargo test -p typdoc --test set` against the code exactly as it stood before touching
`project.rs` (only the tests had been added) to capture this red state; both plus five more new
tests failed for the documented reasons before any fix code was written.

**Full test list from the ticket**, all green after the fix:
- `title=a\*b` → `a*b` (above)
- `title=x*y` → refused, exit 1 (confirmed `ErrorKind::BadArguments → 1` in
  `crates/typdoc/src/cli.rs`'s `exit_code`, not guessed — the ticket's own draft said "2", which
  was wrong; `an_if_with_a_ref_condition_is_refused_plainly`, an existing test, already asserted
  exit 1 for the same `Error::BadArgument` path, confirming the mapping independently)
- `title=a\,b` (scalar) → `a,b`, one field, no split
- `tags=a\,b,c` (list field) → `["a,b", "c"]`
- `title=a\\b` → `a\b`
- `title=a\qb` (unrecognized escape) → refused, exit 1, error message contains `\q`, file
  untouched
- `title=a\` (trailing lone backslash) → refused, exit 1, file untouched
- Same two representative cases (`\*` unescape, bare `*` refusal) through `new --set`, in
  `crates/typdoc/tests/new.rs` (`news_own_set_unescapes_a_backslash_star_to_a_literal_star`,
  `news_own_set_refuses_a_bare_unescaped_star_and_creates_nothing`), since `apply_ops`/
  `new_block` are shared with `set`

Every refused case's before/after file bytes (or file existence, for `new`) were asserted
unchanged.

**Gates**, run in this worktree only:
- `cargo fmt --check` — clean (after one `cargo fmt` pass to reflow the new code/tests)
- `cargo clippy --workspace --all-targets -- -D warnings` — clean
- `TMPDIR=/home/thw-home/.cache/typdoc-tmp scripts/test.sh` — `1010 test(s) passed across 56
  suite(s)`, 0 failed

**Regression check**: the full suite (1010 tests, including every pre-existing test that sets a
field value with no backslash or `*` in it — e.g. `a_comma_separated_value_replaces_a_list_field_
rather_than_appending_to_it`, `auto_update_is_stamped_only_when_a_value_actually_changes`, the
whole of `new.rs`) passed unchanged; a value with no escape character or bare `*` takes the exact
same `unescape_set_value` path it always would have (single pass, no escapes found, split only on
plain `,` for list fields — behaviorally identical to the old `raw.split(',')`/`raw.clone()`).

## Notes

- The ticket's suggested exit code for a refused escape (2) was explicitly flagged as unverified
  in the ticket itself ("confirm which exit code `Error::BadArgument` maps to elsewhere before
  asserting a number"). Checked: `Error::BadArgument` → `ErrorKind::BadArguments` → exit code
  **1**, matching `parse_ifs`'s own refusal path and the pre-existing
  `an_if_with_a_ref_condition_is_refused_plainly` test. All new tests assert exit 1, not 2.
- `apply_ops`'s scalar branch uses one `#[expect(clippy::expect_used)]` to pull the single item
  back out of `unescape_set_value`'s `Vec<String>` return — safe by construction since
  `split_on_comma: false` never pushes more than one item, documented in the `expect`'s own
  `reason`, matching this codebase's existing convention for justified `expect`s.
