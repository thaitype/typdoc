# Ticket 15: goldens for the write commands, and their exit codes

Resolved. Commit `6453d23`, merged `4b5c624` with no conflicts. `cargo fmt --check` clean,
`cargo clippy --workspace --all-targets -- -D warnings` clean, public-text check clean, all re-run
on the merged tree. `scripts/test.sh` with `TMPDIR` on a disk-backed folder: 975 passed / 0 failed /
1 ignored (971 before this ticket).

## Outcome

done

## What it does

`new`, `set` and `mv` already had goldens with no `auto` field, chosen deliberately by the tickets
that built them because the shipped binary's clock is real and a spawned process could not be given
a fixed one. This ticket closes that gap: `TYPDOC_FIXED_CLOCK`, an environment variable the shipped
binary itself reads before building `Deps`, holds an instant with an offset that every `auto:
create`/`auto: update` stamp in that run uses instead of the machine's own time. Unset, nothing
changes. A malformed value panics naming the variable rather than silently falling back to the real
clock. It is not part of the documented CLI; its own doc comment says so. `typdoc-testkit`'s golden
`Case` gained the same `env` map broken fixtures already carry, so a golden case declares the
variables its run needs.

Two new goldens use it: `fixtures/output/new/auto` (a `datetime`/`auto: create` field) and
`fixtures/output/set/auto` (`auto: update`, stamped only because the `set` in question is a real
value change). `fixtures/output/mv/schema-mismatch`, which already covered a destination schema
rejecting the document at exit 0, was extended rather than duplicated: the schema gained an
`auto: moves` field and a second required field, and two holder documents with body links in a
collection with `body.links` off give two `unrewritten` entries — the only one of the three reasons a
plain `mv` can produce, the other two needing `--renumber` or `[reverse-scope]`.

`coverage.rs`'s exit-code table gained an entry for 3, reusing the existing `set --if`
false-condition scenario. `UNPRODUCED_EXIT_CODES` is now empty.

## Checked by running

- The golden comparison is genuinely order-sensitive, not coincidentally so: swapping the two
  `unrewritten`/`findings` entries in the golden, leaving the assertions untouched, turned the golden
  test red with the exact mismatch named; restored.
- `TYPDOC_FIXED_CLOCK` was smoke-tested by hand through the real built binary before being wired into
  any fixture — a run with it unset against one with it set to `2001-02-03T04:05:06+07:00`.
- All three lists were verified, not assumed: `UNIMPLEMENTED_COMMANDS` still holds only `"pull"`
  (untouched, already clean, both its consumers re-checked and passing); `UNIMPLEMENTED_RULES` was
  already empty (confirmed by reading it, both its consumers re-checked and passing) — so the "a rule
  leaving the list brings its fixture in the same change" criterion does not apply to this ticket,
  stated rather than skipped silently; `UNPRODUCED_EXIT_CODES` is now empty, its one consumer passing
  both directions.
- This ticket adds no code to `typdoc-core` — everything lives in the `typdoc` bin crate,
  `typdoc-testkit`, or fixtures — so the write-ban plant-and-revert demonstration does not apply, and
  is not staged against unrelated code to manufacture a false positive.

## Decision

- **Issue:** no existing mechanism bridges an in-process fixed clock (`FixedClock`, reached through
  `deps.clock`) and a golden, which always runs the shipped binary as a real subprocess that never
  sees `deps` directly.
- **Chosen:** an environment variable the binary itself reads, named `TYPDOC_FIXED_CLOCK` for
  consistency with the project's other `TYPDOC_`-prefixed variables, none of which fit this use
  (checked first). Lives entirely in the bin crate, which has no write-ban `clippy.toml` list to
  except from.

## Notes

- A Standards-axis review caught a doc comment claiming the variable was read directly rather than
  through `Env`, which is not what the code does; corrected before committing.
- No crate dependency was added; the stack list in `.chief/project.md` needed no change.
