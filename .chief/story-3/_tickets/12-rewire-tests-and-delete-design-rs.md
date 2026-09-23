# 12: Rewire the three `design.rs` callers onto the catalog documents, delete `design.rs`

Type: implementation
Status: open
Blocked by: 11, 24

**Scope correction, 2026-09-23 (Aria caught this before build started):** `crates/typdoc-testkit/src/fixtures.rs`
is a fourth reader of `design.md`, not named in this ticket's first draft — `locate()` uses
`docs/design/design.md` as its repo-root marker (line 15), and `design_text()` (line 44) reads
it. Both are covered below now. **Blocked by 24 as well as 11**, added after a broader sweep
found a much bigger fifth: `crates/typdoc-testkit/src/shell_examples.rs` and
`crates/typdoc/tests/shell_examples.rs` extract and run design.md's own shell examples against a
real binary — see ticket 24, not resolved yet.

## The work

Rewire, in place, keeping each test's own coverage property (a catalog entry added with no
matching code, or code added with no matching catalog entry, still turns the test red):

- `typdoc-core/src/frontmatter.rs`'s test
  `every_shape_the_design_names_as_lost_is_held_by_some_document_in_the_corpus` — reads
  `docs/design/catalog/frontmatter-losses.md` through ticket 11's helper instead of
  `typdoc_testkit::design::frontmatter_losses`.
- `typdoc-core/tests/rules.rs` — reads `docs/design/catalog/rules.md` instead of
  `typdoc_testkit::design::{always_on_rule_ids, configurable_rule_ids, rule_ids}`; `configurable`
  read from the entry's own field rather than which of two lists it was in.
- `typdoc/tests/coverage.rs` — reads `docs/design/catalog/commands.md` and
  `docs/design/catalog/exit-codes.md` instead of `typdoc_testkit::design::{command_names,
  exit_codes}`.

Then delete `crates/typdoc-testkit/src/design.rs` entirely, and its module declaration and any
now-unused re-exports.

**`fixtures.rs`'s two design-reading functions:**
- `locate()` uses `docs/design/design.md`'s existence as one of its two repo-root markers
  (alongside the `fixtures/` folder). Replace it with a marker that isn't a design document —
  `Cargo.toml` at the root, or `.chief/`, or another folder every checkout has that isn't itself
  spec content. Pick one that can't be confused for "reading the design."
- `design_text()` is removed entirely once nothing calls it — but check first: as of this
  ticket's own draft, `crates/typdoc-testkit/src/shell_examples.rs`,
  `crates/typdoc/tests/shell_examples.rs`, and `fixtures.rs`'s own self-test
  (`design_text().starts_with("# typdoc")`) all call it too, and none of those is decided yet
  (ticket 24). Do not remove `design_text()` out from under ticket 24's still-open work; if
  ticket 24 resolves to keep some form of it, coordinate rather than deleting and re-adding.

`rg 'typdoc_testkit::design\b'`, `rg 'design_text'`, and `rg '\bdesign\.rs\b'` (outside history)
return nothing once this ticket and ticket 24's build work are both done — this ticket alone may
not be able to make all three true if ticket 24 isn't finished first.

## Tests

- All three rewired tests pass, reading the catalog documents ticket 10 created.
- Each still turns red on the same class of drift it caught before: add an id to
  `docs/design/catalog/rules.md` with no matching code, and `rules.rs`'s test fails; remove one
  from `commands.md` or `exit-codes.md` with the code unchanged, and `coverage.rs`'s test fails;
  edit an entry out of `frontmatter-losses.md` that a fixture still demonstrates, and
  `frontmatter.rs`'s test fails. Confirmed by planting each drift once and reverting it — not
  assumed from the old tests having had the property.

## Done

- `design.rs` no longer exists. `rg` for it and for `typdoc_testkit::design` finds nothing outside
  history.
- `fixtures.rs`'s `locate()` uses a marker that isn't a design document.
- All three tests pass against the catalog documents and are re-confirmed able to catch drift.
- `design_text()` is removed only if ticket 24's resolution no longer needs it; otherwise this
  ticket's report says exactly what still calls it and why, and ticket 24 owns removing it.
