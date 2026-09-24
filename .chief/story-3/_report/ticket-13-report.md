# Ticket 13 Report

## Ticket

Archive the old design documents: move `docs/design/design.md`, `design-decision-phase-1/` and
`design-decision-phase-2/` to `docs/archived-design/` (frozen record), copy the same three to
`docs/migrating-design/` (working copy for later tickets to progressively empty), and fix every
live reference to the old paths.

## Outcome

done

## Verification

**`rg` before (three exact targets from the ticket text), across the whole repo:**

- `docs/design/design.md` — hits in `.chief/story-1/`, `.chief/story-2/`, `.chief/story-3/`
  (contract/goal/brief/map/tickets/reports), and live code/docs: `.chief/project.md` (x2),
  `README.md`, `docs/commands.md` (x2), `docs/projects.md`, `docs/development.md` (x2, as
  relative links `design/design.md`), `crates/typdoc/tests/shell_examples.rs` (x2),
  `crates/typdoc-testkit/src/fixtures.rs` (x1).
- `design-decision-phase-1` / `design-decision-phase-2` — hits in `.chief/story-1/`,
  `.chief/story-2/`, `.chief/story-3/`, plus live code: `crates/typdoc-core/tests/body_links.rs`,
  `crates/typdoc-core/src/project.rs`, and `docs/development.md` (relative links).

Also caught, by broadening the search past the ticket's three literal strings, two more relative
`design/design.md` links inside `docs/commands.md`/`docs/projects.md`/`docs/development.md` that
the exact `docs/design/design.md` string missed (they omit the `docs/` prefix since they're
already inside `docs/`).

**Moves and copies:**

- `git mv docs/design/design.md docs/archived-design/design.md`
- `git mv docs/design/design-decision-phase-1 docs/archived-design/design-decision-phase-1`
- `git mv docs/design/design-decision-phase-2 docs/archived-design/design-decision-phase-2`
- `cp -r` of the same three into `docs/migrating-design/`, then `git add`.
- `diff -r docs/archived-design docs/migrating-design` — identical, confirmed before committing.
- 49 renames (`git mv`) + 49 new files (`cp` + `git add`) — matches exactly.
- Nothing inside either tree was edited (internal cross-references between phase-1/phase-2, e.g.
  `docs/archived-design/design-decision-phase-2/_map.md`'s mention of `design-decision-phase-1`,
  are untouched, as instructed).

**Live references updated** (path only, no other content changes except one, noted below):

- `.chief/project.md` (source-of-truth line, and the `typdoc-testkit` directory-structure line)
- `README.md`
- `docs/commands.md` (x2), `docs/projects.md`, `docs/development.md` (x3: the intro line and both
  decision-record links)
- `crates/typdoc/tests/shell_examples.rs:89` (a present-tense "as it appears in" reference)
- `crates/typdoc-testkit/src/fixtures.rs:16`
- `crates/typdoc-core/tests/body_links.rs:3`
- `crates/typdoc-core/src/project.rs:1520`

**`rg` after:** re-ran all searches (the exact three strings, plus the bare `design/design.md`
relative-link form). Remaining hits are only in `.chief/story-1/` and `.chief/story-2/` (left as
their own frozen records, per the ticket), `.chief/story-3/`'s own contract/goal/brief/map and
ticket/report files (also treated as frozen planning/build records — not one of them is a "go
read this file now" pointer; each narrates history or the moves this and later tickets will make,
and rewriting a ticket file for the very ticket built from it, or a settled report of a merged
ticket, would be revisionist), and one deliberately-untouched historical line,
`crates/typdoc/tests/shell_examples.rs:234` ("Before this ticket [12], each was found
automatically by walking `docs/design/design.md`" — past tense, describing the pre-ticket-12
mechanism as it stood when it ran against the file at its then-real path; changing that path would
misdescribe when the mechanism last operated on it). `docs/design/spec/SPC-{1,2,3,4}.md`'s
`migrated_from:` frontmatter already pointed at `docs/archived-design/design.md#...` before this
ticket (written that way by ticket 22, anticipating the move) — confirmed unchanged and correct.

**One incidental correction while touching `.chief/project.md`'s `typdoc-testkit` line:** it still
described the crate as "the loader of `fixtures/` and `docs/design/design.md`, the reader of what
the design names" — a mechanism ticket 12 already deleted (`design.rs`/`design_text()`). Left as a
straight path swap it would have stayed false regardless of path, so corrected the wording to say
the extraction mechanism was removed in ticket 12, while keeping this ticket's edit scoped to that
one line (not a broader project.md audit).

**Gates**, all in this worktree:

- `cargo fmt --check` — clean, no output.
- `cargo clippy --workspace --all-targets -- -D warnings` — clean, no warnings.
- `TMPDIR=/home/thw-home/.cache/typdoc-tmp scripts/test.sh` — first run: 997 passed, one target
  (`lock_contention`) failed
  (`n_processes_racing_one_lock_issue_n_distinct_keys_with_no_document_overwritten`) — the same
  machine-load-sensitive concurrency flake ticket 12's report already documented. Reran that one
  test alone: green (`1 passed; 0 failed`, 7.59s). Reran the full suite: **998 tests across 56
  suites, 0 failed.** No lock/concurrency code was touched by this ticket.

## Notes

- `docs/migrating-design/` now holds an untouched copy of all three; ticket 22 (or whichever
  ticket progressively empties it into `docs/design/spec/`/`docs/design/catalog/`) starts from
  this identical copy.
- `docs/archived-design/design.md`, `design-decision-phase-1/`, `design-decision-phase-2/` are the
  frozen record now; nothing outside `.chief/story-1/`/`.chief/story-2/`/`.chief/story-3/`'s own
  planning-and-report files points at the pre-move paths.
- `.chief/story-3/`'s own contract/goal/brief/map/tickets/reports still say `docs/design/design.md`
  etc. in places (they're describing the story's plan and history, written before or during this
  ticket) — left alone deliberately, same treatment as `.chief/story-1/`/`.chief/story-2/`, since
  none of them are live navigation pointers.
