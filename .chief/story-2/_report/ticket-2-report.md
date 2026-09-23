# Ticket 2: a fixture whose command writes runs on a copy

Resolved. Commit `87cfc3b`. Four gates green on the committed tree, each re-run after the commit:
`cargo fmt --check` clean, `cargo clippy --workspace --all-targets -- -D warnings` clean,
`scripts/test.sh` 775 passed / 0 failed / 1 ignored (767 / 0 / 1 before), public-text check clean.

## Outcome

done

## What it does

`crates/typdoc-testkit/src/staging.rs` holds `stage(dir, spec)`, the one decision about where a
fixture's command runs. A declared read runs in the fixture's own folder, exactly as before. A
declared write is copied to a temporary folder and the copy is what runs, so a write never reaches
the repository's tree. `StagedFixture` owns the copy for the run's lifetime, so nothing is removed
while a command still needs it.

A spec is a write when the first word of its declared command is `new`, `set` or `mv`
(`FixtureSpec::is_write`). The classification reads the declaration only, never the binary: none of
the three commands exists yet, and the placement is enforced ahead of one existing so that the
ticket which adds a write command does not have to add the enforcement with it.

`crates/typdoc/tests/common/mod.rs` gained `spawn_fixture(dir, rule)`, which loads, stages, builds
the `Spawn` and runs it. The two tests that ran a fixture's declared command — `coverage.rs` and
`config.rs` — go through it, so no individual test decides where a fixture runs.
`validate.rs`'s accounting-invariant test was walked as well: it never reads `spec.command` and
always spawns its own `validate --audit --json`, which is a read whatever the fixture declares, so
it needed no change.

## Checked by running

- A write fixture pointed at a real committed fixture folder (`fixtures/valid/minimal`) stages
  outside it, and the folder's bytes are read fresh from disk before and after and are identical.
- The harness guard was shown red three ways, one mutation at a time, each restored: `is_write`
  forced to `false` (four tests red); the guard short-circuited (only its own test red, which shows
  the test is precise); and `stage` itself made to hand the guard the original folder and return it
  as the run location, which is the ticket's own failure mode end to end — it returned the refusal
  naming the path and the two tests calling it went red with that message.
- Every read fixture still runs as it did: the whole broken-fixture suite now goes through
  `spawn_fixture` and stays green at the same count.

## Decision

- **Issue:** no write command exists yet, so "after a write fixture runs" cannot be exercised by a
  real write through the built binary.
- **Options considered:** wait for ticket 9, 10 or 11 and build the loader with them; declare a
  fixture as a write and simulate the write's effect on the staged copy; add a fixture under
  `fixtures/broken/` for the write rule now.
- **Chosen:** simulate. The tests call `std::fs::write` on the staged copy's path — never through
  the spawn helper, so the rule that a process starts in one place is untouched — and then assert
  the repository's own fixture is byte-identical. A fixture under `fixtures/broken/` was not added:
  `frontmatter.transitions` is still in `UNIMPLEMENTED_RULES` and the broken-fixture coverage test
  refuses a fixture for a rule that is not built, so that fixture belongs to whichever ticket lands
  `set`.

## Notes

- `tempfile` is now an ordinary dependency of `typdoc-testkit`, not a dev-dependency: `stage` is
  library code called from other crates' test binaries, and a crate's own dev-dependencies are not
  visible to a crate depending on it. `.chief/project.md`'s stack line says so.
- `WRITE_COMMANDS` in `typdoc-testkit` is kept by hand and lists `new`, `set` and `mv`. It is not
  derived from the binary crate's list of unimplemented commands, because the dependency runs the
  other way and that list also holds `pull`, which is story 3. A ticket that adds a write command
  whose name is not one of the three must add it here.
- The guard cannot fire under correct code, because `stage` always hands it a freshly made
  temporary path. That is what the third mutation above is for.
