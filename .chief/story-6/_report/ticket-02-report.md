# Ticket 02 Report

## Ticket
Pilot batch: every comment in `typdoc-fs`, `typdoc-testkit` and `typdoc-core/src/json_body.rs`
reviewed, and the design those comments cite moved into `docs/design/`.

## Outcome
done

## Notes
- Two new SPCs: SPC-10 "Writing files" (five Concurrency bullets moved out of
  `docs/migrating-design/design.md`) and SPC-11 "Design documents explained" (from the story 3
  contract; nothing deleted from `.chief/`).
- Citation case 1 (design already in `docs/design/`) does not occur in this scope; cases 2 and 3
  do. The full per-citation table and the Investigate list (seven items, none fixed) are in the
  pilot PR's description.
- The contract's first proof method was replaced before use: `rustc -Zunpretty=expanded` keeps
  ordinary `//` comments, so a comment-only change showed as a difference. The proof now strips
  comments from source text and compares each changed file; shown to report a planted one-token
  code change and to pass a comment-only change.
- Gates at head: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `scripts/test.sh` (1057 passed, 0 failed, 1 ignored), `typdoc validate`, the comment-only
  proof (12 files, 0 differ): all green.
- Later batches wait for the pilot's review; what it changes is applied to them.
