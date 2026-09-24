# Ticket 10 Report

## Ticket

Set up this repo's `.typdoc/` project (M-11's final schema shape) and author the four catalog
documents plus the minimum spec documents `explained_by` needs.

## Outcome

done

## Decision

- **Issue:** whether `explained_by` is required, left open by the contract/ticket for the builder
  to decide while building.
- **Chosen:** required — matches M-11's phrasing (grouped with two required fields, no "only
  when..." qualifier). Authored four short, real `SPC-1..SPC-4` documents (`status: active`), one
  per catalog document, each genuinely explaining that document's content — not placeholders.
  Every `explained_by` ref confirmed resolving via `typdoc refs`/`validate`.
- **Own addition, flagged and accepted:** `spec.json`'s `superseded_by` field got `acyclic: true`
  — not asked for or forbidden by M-11, a reasonable guard against a supersession cycle. Low
  risk, kept.

## Notes

Verified independently, not just trusted: extracted `design.rs`'s five extraction functions
standalone, compiled with plain `rustc`, ran them against the live `docs/design/design.md`
(still at that path — ticket 13 hasn't moved it), and diffed the result against each catalog
document's actual JSON body with `jq` — empty diff on all five (rule ids, configurable split,
commands, exit codes, losses). Re-spot-checked `rules.md` myself before merging: 23 entries, 14
always-on + 9 configurable, matching the report exactly.

Built and used the real `v0.1.0` binary (isolated scratch build from `git archive v0.1.0`, no
worktree touched) to author every document through the actual CLI, per M-5 — no hand-authoring
workaround needed. `typdoc validate --json` on the finished project: zero findings across all 8
documents (4 catalog + 4 spec).

**Housekeeping catch, not a build defect:** this ticket's own `Status:` line was still `open` at
merge time — an earlier `sed` command of mine had targeted a fixed line number that no longer
matched after this file grew from earlier edits, so the "claim" update silently didn't take.
Caught by the build's own review (it correctly left ticket-status bookkeeping alone, flagged it
instead of guessing) and fixed here by line content rather than line number.

All three gates re-verified green after rebasing onto the current tip (tickets 17-21 in between —
no conflicts, disjoint files: this ticket touches no Rust code at all). Fast-forward merged into
`story-3-catalog-and-release`.

Unblocks ticket 11.
