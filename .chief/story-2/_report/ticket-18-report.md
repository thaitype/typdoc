# Ticket 18: write down how to resolve a merge conflict in a state file

Resolved. Commit `0ea13ea`. `cargo fmt --check` clean, `cargo clippy --workspace --all-targets -- -D
warnings` clean, `scripts/test.sh` with `TMPDIR` on a disk-backed folder 979 passed / 0 failed / 1
ignored (unchanged — this ticket adds no test), public-text check clean, all re-run on the committed
tree.

## Outcome

done

## What it does

`docs/projects.md`'s `## State` section gained one short paragraph, directly after the paragraph
naming the three rules that read the file, matching the ticket's own placement instruction — next to
the file's description, not in the full design document, since the reader who needs this is holding
a conflict marker rather than reading front to back. The rule comes first, the reason after it, the
sentence about not reverting last, per the ticket's own shape: on a merge conflict, take the higher
`last`, never a side, and never revert the file to an older version. The reason names the failure
mode the ticket spends most of its own words on: the lower side's lost number may already belong to
a deleted document, `last` is the only remaining record it was ever used, and `typdoc new` hands it
out again to the wrong document with nothing afterward for `validate` to find.

## Checked by running

All four gates on the final committed tree. No test applies — this is guidance in a documentation
page, not a rule in the code, exactly as the ticket asks for. No guard plant applies either: this
ticket touches no code in `typdoc-core` or anywhere else, the same reasoning tickets 15, 16 and 17
already recorded for the identical situation.

## Notes

- A code review caught a wording defect before committing: the reason sentence's first draft read as
  a bare imperative sitting right next to "never take a side," which could look self-contradicting to
  someone skimming mid-merge — exactly the failure mode the ticket warns against. Reworded to
  "keeping the lower side lets `typdoc new` hand it out again" to remove the false-imperative
  reading.
- **This ticket's build hit a real, unrelated public-text gate failure and correctly stopped rather
  than bypassing it**, per brief rule 2: `.chief/story-2/_report/ticket-17-report.md` (committed in
  ticket 17's own commit, `7ebce7e`) used the phrase "confirmed by rerunning," which matches the
  gate's generic `\bconfirmed by\b` pattern — an oversight in that report, written after ticket 17's
  last public-text check and never re-checked before committing. Fixed separately, scoped to ticket
  17's own report (`65b4f3e`), before this ticket's own commit.
