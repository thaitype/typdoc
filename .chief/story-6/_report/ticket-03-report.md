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
- Every `decision N` cited in the file is a phase-2 decision except the two that name phase 1
  themselves.
- Also in this PR, as its own commit: the `json_body.rs` test that asserts `MissingContentType`
  for a document with no frontmatter block is renamed to say so. It is the one Rust change in the
  story that is not a comment; the comment-only proof reports that file alone, and only that
  function name differs.
- Seen and not changed, since they are code or string literals, not comments: three
  `AlreadyExists` messages a user sees end in a decision number; an `#[allow]` reason string says
  `body.mentions` is not built by `mv`, which is out of date; `mv_lock` builds `local` lock paths
  without checking the lock mode, while `set` and `new` refuse `git-common`.
- Gates at head: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `scripts/test.sh` (1057 passed, 0 failed, 1 ignored), `typdoc validate`, all green.
