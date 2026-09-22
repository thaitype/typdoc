# Ticket 6: a field written with no value, kept apart from an empty string

Resolved. Commit `39bb13d`. Four gates green on the committed tree, each re-run after the commit:
`cargo fmt --check` clean, `cargo clippy --workspace --all-targets -- -D warnings` clean,
`scripts/test.sh` 822 passed / 0 failed / 1 ignored (820 / 0 / 1 before), public-text check clean.

## Outcome

done

## What it does

`Value` gained `Empty`, told apart from `Text(String::new())` by how the field was written: a bare
name (`reviewer:`) reads as `Empty`, and `reviewer: ''` still reads as `Text("")`. A literal `~` or
the word `null` stays ordinary text, because only a scalar with both YAML-null shape and empty raw
bytes is `Empty`. The exhaustive match forced every consumer into view: coercion, query matching and
emptiness, the enum-membership check, display, sorting, `ref_values`, the `list` table cell and
`--json` printing. All but the last treat `Empty` exactly as they treated an empty string; `--json`
prints `Empty` as `null`, which is where the two forms are told apart on purpose.

The writer puts back the form it found. `yaml_serde` has no way to emit a bare scalar at the serde
level, so a write serializes an `Empty` field as `name: null` and a pass afterward turns that whole
line into `name:`, matched only against field names this run marked `Empty` — safe because
`Text("null")` is always quoted by the library's own inference, so an unquoted `name: null` line can
only be this pass's own output.

`[empty-value]` left `KNOWN_GAPS` in this commit.

## Checked by running

- The test that pinned the gap is turned round, not deleted, and now asserts `null` against
  `Text("")`'s `""`; it still checks the two invariants the design names — a bare required field is
  present with no finding, and a bare `number` field still fails its type.
- A document holding each form round-trips byte for byte, and a fixture already holding a bare field
  (`fixtures/valid/field-types/records/unknown.md`) still validates with the same finding after a
  write touches the block.
- Four things were shown able to fail, one at a time, each restored: the read-side branch that tells
  the forms apart; the write-side pass that bares the line, caught through the real fixture and not
  only a synthetic case; the new type-fitting arms, caught by a bare string field gaining findings it
  should not have; and the fixture-parity test, caught by changing the finding's severity.
- The existing write ban reaches the new pass: `std::fs::write` planted inside it, in library code
  and then separately in test code, was refused both times, and removed both times.

## Notes

- Every consumer outside the writer was walked and changed to keep `Empty` meaning the same as
  before: `coerce`/`fits` (unchanged fit for `string`/`enum`/`ref`/`other`, still fails
  `number`/`bool`/`date`/`datetime`), `query`'s emptiness check and text value, `validate`'s
  enum-membership check and display, `project`'s sort value and `ref_values`. The compiler's own
  exhaustiveness check found four of these by refusing to build, which is stronger evidence than
  reading the call sites would have been alone.
- `new`, `set` and `mv` still do not exist, so nothing on the write side needed updating there; they
  inherit the corrected `Value` when built.
