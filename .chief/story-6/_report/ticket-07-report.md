# Ticket 07 Report

## Ticket
Every comment in the write-command tests of `crates/typdoc/tests` (`new`, `set`, `mv`,
`mv_renumber`, `state`, `lock_contention`, `signals`, `templates`, `frontmatter_scalars`, and the
shared helpers) reviewed; each test file that covers an SPC now says so once at its top.

## Outcome
done

## Notes
- Comment lines in scope: 731 before, 257 after. Five comments the code clearly contradicted are
  corrected; nothing is listed as unclear.
- `//! Covers` lines: `new.rs`, `set.rs`, `mv.rs`, `mv_renumber.rs`, `state.rs`,
  `lock_contention.rs`, `signals.rs`, `templates.rs`, `frontmatter_scalars.rs`; the helpers in
  `common/` and `support/` cover no SPC and say nothing.
- Design moved: SPC-12 gains "A `number`" (two lines deleted from
  `docs/migrating-design/design.md`); SPC-13 gains a paragraph stating how a `number` is compared,
  written from the code. The open question about comparing numbers no primitive holds stays in
  `docs/migrating-design/` as it is.
- Also in this PR, as its own commit: an assertion message in `mv.rs` drops a decision number, and
  an `ignore` reason in `templates.rs` drops the date of a CI run. The proof reports those two and
  nothing else.
- Also as its own commit: a test in `state.rs` named after a decision number is renamed to
  `deriving_last_reissues_a_retired_key_silently`; its assertions are unchanged.
- Gates at head: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `scripts/test.sh` (1057 passed, 0 failed, 1 ignored), `typdoc validate` at every commit.
