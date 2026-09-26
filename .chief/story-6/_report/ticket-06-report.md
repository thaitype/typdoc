# Ticket 06 Report

## Ticket
Every comment in `crates/typdoc/src` and in the binary crate's `Cargo.toml` and `clippy.toml`
reviewed, and the design those comments cite moved into `docs/design/`.

## Outcome
done

## Notes
- Comment lines in scope: 520 before, 182 after. Six comments the code clearly contradicted are
  corrected; four mismatches with no clear answer are listed in the PR.
- Design moved: SPC-3 gains "An interrupted run". Two lines deleted from
  `docs/migrating-design/design.md`.
- The clap doc comments on `Cli` and `Command` are `--help` text, so they are behavior, and are
  unchanged.
- Also in this PR, as its own commit: the `[reverse-scope]` entry of `KNOWN_GAPS` no longer points
  at the design; its meaning is unchanged. The proof reports that string and nothing else.
- The PR's citation table has one row per hunk the citation audit lists (46), and lists the
  citations the audit's pattern misses.
- Gates at head: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `scripts/test.sh` (1057 passed, 0 failed, 1 ignored), `typdoc validate` at every commit.
