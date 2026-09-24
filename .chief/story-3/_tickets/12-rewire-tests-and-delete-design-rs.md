# 12: Rewire the three `design.rs` callers onto the catalog documents, delete `design.rs`

Type: implementation
Status: resolved
Blocked by: None (M-13 landed 2026-09-24 — see ticket 24's Answer)

**Unblocked 2026-09-24.** `crates/typdoc-testkit/src/fixtures.rs` is a fourth reader of
`design.md`, not named in this ticket's first draft (Aria caught it before build started) —
`locate()` uses `docs/design/design.md` as its repo-root marker, `design_text()` reads it. Ticket
24 (M-13) found a fifth, much bigger one — `crates/typdoc-testkit/src/shell_examples.rs` and
`crates/typdoc/tests/shell_examples.rs` extract and run `design.md`'s own shell examples — and
resolved it (option 1: hand-list, no fifth catalog document). All of it is this ticket's scope
now: `design.rs`, `fixtures.rs`'s two functions, and `shell_examples.rs`'s extraction mechanism
all get removed together, since M-13's answer means nothing needs `design_text()` any more once
this ticket is done — no more "coordinate with ticket 24" caveat.

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
- `design_text()` is removed entirely (M-13 settled this — nothing needs it once
  `shell_examples.rs`'s extraction mechanism is also gone, below). Also remove `fixtures.rs`'s own
  self-test that calls it (`design_text().starts_with("# typdoc")` or similar).

**Ticket 24/M-13's part, built here too — `shell_examples.rs` (both crates):**
- Delete `crates/typdoc-testkit/src/shell_examples.rs` in full: `examples()`,
  `quoting_paragraph_spans()`, `is_example()`, and their helpers.
- In `crates/typdoc/tests/shell_examples.rs`: the hand-declared list (`declared_texts()` and
  whatever holds the already-hand-written expected values) becomes the *only* list — remove the
  call into `typdoc_testkit::shell_examples::examples()`/`quoting_paragraph_spans()` and the two
  tests that only existed to check extraction against the declared list or against the quoting
  paragraph (remove them, don't adapt them — there's nothing left for either to check). The
  sh/bash runs against the stand-in binary, for whatever's left in the hand-declared list, stay
  exactly as they are.
- For every "safe" example (no shell-unsafe character) that only ever reached the harness through
  extraction — never hand-declared, since a safe example's expected value was computed
  automatically by whitespace-splitting — decide per example: hand-list it (keeps that example's
  coverage) or drop it (not worth a hand-written entry). Record which examples were kept vs.
  dropped, and why, in this ticket's report — this is a real judgment call ticket 24 explicitly
  left to this ticket, not a detail to wave through silently.

`rg 'typdoc_testkit::design\b'`, `rg 'design_text'`, `rg '\bdesign\.rs\b'`, and
`rg 'typdoc_testkit::shell_examples'` (outside history) all return nothing once this ticket is
done.

## Tests

- All three rewired tests pass, reading the catalog documents ticket 10 created.
- Each still turns red on the same class of drift it caught before: add an id to
  `docs/design/catalog/rules.md` with no matching code, and `rules.rs`'s test fails; remove one
  from `commands.md` or `exit-codes.md` with the code unchanged, and `coverage.rs`'s test fails;
  edit an entry out of `frontmatter-losses.md` that a fixture still demonstrates, and
  `frontmatter.rs`'s test fails. Confirmed by planting each drift once and reverting it — not
  assumed from the old tests having had the property.

## Done

- `design.rs` and `typdoc-testkit/src/shell_examples.rs` no longer exist. `rg` for either, for
  `typdoc_testkit::design`, for `design_text`, and for `typdoc_testkit::shell_examples` finds
  nothing outside history.
- `fixtures.rs`'s `locate()` uses a marker that isn't a design document.
- All three rewired tests pass against the catalog documents and are re-confirmed able to catch
  drift.
- `crates/typdoc/tests/shell_examples.rs` still runs every example on its hand-declared list
  (kept "safe" examples included, per this ticket's own judgment call) through a real shell
  against the stand-in binary — the same coverage shape as before, minus the markdown extraction.
