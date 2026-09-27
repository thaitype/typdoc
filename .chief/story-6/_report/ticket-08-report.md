# Ticket 08 Report

## Ticket
Every comment in the read-command tests and the rest of `crates/typdoc/tests` (`validate`,
`imports`, `list`, `refs`, `get`, `toc`, `body`, `arguments`, `config`, `namespaces`,
`outside_namespaces`, `schemas`, `examples`, `golden`, `coverage`, `shell_examples`) reviewed;
each test file that covers an SPC now says so once at its top. The last comment batch.

## Outcome
done

## Notes
- Comment lines in scope: 958 before, 365 after. Eight comments the code clearly contradicted are
  corrected; nothing is listed as unclear.
- `//! Covers` lines on every file in scope that covers an SPC; `examples.rs` checks the shipped
  example project and says nothing.
- Design moved: SPC-13 gains "Quoting in the shell"; SPC-14 gains "Canonical form" and "Body links
  that are not refs"; SPC-12 gains the order of outgoing refs. 14 lines deleted from
  `docs/migrating-design/`.
- Also in this PR, each as its own commit: seven strings that pointed at history (six `ignore`
  reasons, one panic message) no longer do; four test names that said how the behavior changed
  are renamed for what they check. The proof reports those and nothing else.
- After this batch, `crates/` has no public-text hit and no history citation; the one match of
  the history search is a `.chief/note.md` fixture path in a leading-dot path test.
- Gates at head: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `scripts/test.sh` (1057 passed, 0 failed, 1 ignored), `typdoc validate` at every commit.
