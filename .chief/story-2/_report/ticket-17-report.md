# Ticket 17: a `Concept and Mental model` section in the README, separating namespace from collection

Resolved. Commit `2acd905`. `cargo fmt --check` clean, `cargo clippy --workspace --all-targets -- -D
warnings` clean, public-text check clean, all re-run on the committed tree. `scripts/test.sh` with
`TMPDIR` on a disk-backed folder: 979 passed / 0 failed / 1 ignored, unchanged — this ticket adds no
test. One transient run showed 977/2/1 during independent re-verification; re-run twice more, both
979/0/1 — a one-off flake from contending scratch activity, not a regression, confirmed by rerunning
clean.

## Outcome

done

## What it does

`README.md` gained `## Concept and mental model`, between "What a project looks like" and
"Commands" — the vocabulary the Commands table then applies to real commands. It follows the four
parts the ticket named, in order: a real two-namespace, two-collection example project; real
`typdoc list --fields namespace,collection,key,path` output showing `WF-1` twice, once per
namespace; a two-axis table; one closing sentence separating the two words.

## Checked by running

- Independently reproduced the exact example project (two namespaces `story-1`/`story-2`, a coded
  `_tickets` collection and an uncoded `_notes` collection in each, four documents) and ran the same
  `typdoc list` command against the release binary: the output matched the README byte for byte,
  confirming it was not hand-written.
- The three error ids the section's opening paragraph names (`config.namespace-name`,
  `config.collection-name`, `collections.overlap`) were checked against `crates/typdoc-core/src/
  rules.rs` and are real, registered rule ids.
- All four gates re-run clean on the final committed tree.

## Notes

- No guard plant applies: this ticket touches no code in `typdoc-core` or anywhere else, only
  `README.md` — the same reasoning tickets 15 and 16 already recorded for the identical situation.
- A code review found the intro paragraph and the closing sentence both defining the two words redundantly; fixed by trimming the intro to motivation and evidence only, leaving the closing sentence as the section's one definitional payoff.
- Collection identifiers were chosen as `_tickets`/`_notes` (the literal strings the ticket names),
  not `tickets`/`notes` in an underscored folder — the reading that makes the vocabulary show up
  everywhere a reader looks (folder, collection name, `list` output), not just in the folder name.
