# Ticket 03 Report

## Ticket
Every comment in `crates/typdoc-core/src/project.rs` reviewed, and the design its kept comments
cite moved into `docs/design/`.

## Outcome
done

## Notes
- Comment lines in `project.rs`: 1,265 before, 383 after. About 155 blocks reduced, 45 removed,
  4 corrected, 16 kept unchanged; none listed for a decision.
- Design moved: SPC-10 gains "Lock order" and "What is locked" (from the Concurrency section of
  `docs/migrating-design/design.md`); SPC-2 gains "What `new` and `set` check before they write"
  (from the Refs section). 4 lines deleted from `docs/migrating-design/design.md`, one
  cross-reference repointed to SPC-10.
- A second pass over the removed citations: every removed citation that pointed at design is now
  either cited by SPC key or recorded as held by one, and design that was only in
  `docs/migrating-design/` or `.chief/` is moved. New SPC-12 (JSON output), SPC-13 (queries),
  SPC-14 (refs), SPC-15 (schema fields); sections added to SPC-1, SPC-2, SPC-4, SPC-7, SPC-10.
  The PR's citation table has one row per removed citation. Five places where the design text
  and the code disagree, with no clear answer, stay in `docs/migrating-design/` and are listed.
- Every `decision N` cited in the file is a phase-2 decision except the two that name phase 1
  themselves.
- Also in this PR, as its own commit: the `json_body.rs` test that asserts `MissingContentType`
  for a document with no frontmatter block is renamed to say so. It is the one Rust change in the
  story that is not a comment; the comment-only proof reports that file alone, and only that
  function name differs.
- Also in this PR, as its own commit: three `AlreadyExists` messages a user sees no longer end in
  a decision number, and an `#[allow]` reason string that described `body.mentions` as not built
  is brought up to date. The proof reports these and the test rename, and nothing else.
- Seen and not changed: `mv_lock` builds `local` lock paths without checking the lock mode, while
  `set` and `new` refuse `git-common`; the design text for reverse `refs` includes imported
  projects, and `Project::refs` scans only this project. Both are left for later work.
- Gates at head: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `scripts/test.sh` (1057 passed, 0 failed, 1 ignored), `typdoc validate`, all green.
