# Ticket 04 Report

## Ticket
Every comment in the `typdoc-core` write path (locks, the write seam, state, `mv`, frontmatter,
templates, and their tests) reviewed, and the design those comments cite moved into
`docs/design/`.

## Outcome
done

## Notes
- Comment lines in scope: 1,053 before, 484 after. Ten comments the code clearly contradicted are
  corrected; two mismatches with no clear answer are listed in the PR.
- Design moved: SPC-10 gains "Taking a lock", "When a lock is not acquired" and "The window when
  a lock is taken", and a paragraph on the held lock; SPC-4 gains "What a write keeps" and "How a
  write is made"; SPC-1 gains "Lines and columns"; new SPC-16 covers pinned remote schemas.
- Before the comment commits, one docs-only commit removes ticket and `M-N` numbers and
  change-narration from SPC-5 and repoints three references to `design.md` sections (SPC-2,
  SPC-7, SPC-8) to the SPCs that hold them.
- The PR's citation table has one row per hunk the citation audit lists (58).
- Also in this PR, as its own commit: a test assertion message in `src/namespace_lock.rs` no
  longer names a decision number; its reason is unchanged. The proof reports that difference and
  nothing else. A message in `tests/namespace_lock.rs` that mentions the design is left as it is.
- Gates at head: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `scripts/test.sh` (1057 passed, 0 failed, 1 ignored), `typdoc validate` at every commit, the
  comment-only proof (19 files, 0 differ). No `trybuild` expectation changed.
