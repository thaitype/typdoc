# Ticket 35 Report

## Ticket

M-22: `mv`/`mv --renumber` never detected a plain-text mention of the key being moved, so
`unrewritten` stayed empty even when one existed — a later `validate` run was the only thing that
ever caught it. `UnrewrittenReason::Mention` existed as a placeholder enum variant but nothing
ever constructed it.

## Outcome

done

## The fix

Added `Project::mv_reverse_mentions(&self, from: &RefName) -> Result<Vec<UnrewrittenRef>, Error>`
in `crates/typdoc-core/src/project.rs`, called once from the end of `mv_reverse_scan` via
`unrewritten.extend(self.mv_reverse_mentions(&reverse.document)?)`. It:

- Returns immediately (no file read at all) when `from.key` is `None` — a path-identified
  document has no key for a mention to ever name.
- Otherwise iterates every document in `self.index` (sorted by path, matching the existing
  reverse-refs scan's own ordering), reads each file's raw text, computes `body.mentions`'s own
  `inlineCode`/`fencedCode` options via `rule_options`/`bool_option` (the holder's own collection's
  validation config), and calls `links::mentions(&text, inline_code, fenced_code)`.
- For each mention, strips a `namespace:` prefix the same way `mention_missing` does
  (`written.rsplit(':').next()`) and compares the remainder directly against `from`'s own key
  (never `mention_missing` itself, which asks "is this key currently missing from the index" — the
  wrong question before the move has happened, since the key still resolves at scan time).
- For every match, pushes an `UnrewrittenRef { reference: RefsReference { other:
  Resolved(self.ref_name_of(path)), field: "$body", written: mention.written.clone(), position:
  Some(Position { line, col }) }, reason: UnrewrittenReason::Mention }`.
- A document whose frontmatter fails to parse is skipped silently (`let Ok(mentions) = ... else {
  continue }`), the same as `refs --reverse`'s own reverse loop and `Project::incoming_refs`
  already treat a malformed document — its own `frontmatter.parse` finding is a different rule's
  job.

**Reuse decision (the tradeoff the ticket flagged):** I did not extend `Project::incoming_refs`.
That function is unrelated machinery — it is used only by `list`'s `refby.*` query filter, not by
`mv_reverse_scan` at all (which instead calls the public `self.refs(from, scope, true, None,
env)`, itself a *third*, separate whole-project loop). Extending `incoming_refs` would not have
saved `mv` a read, since `mv_reverse_scan` never calls it today; extending the public `refs()`
reverse-scan loop instead would reshape a method `refs --reverse` also depends on, for a concern
only `mv` has. Both existing loops also discard each document's raw text after parsing it into
fields/body, and `links::mentions` needs that raw text (frontmatter included) for its own
line/col math. Given that, I added a clearly-separate second pass, but bounded its cost: it never
reads a single file when `from` has no key (true for every plain `mv`, see below), and only runs
on the heavier `--renumber` path, which is already writing a new state file and allocating a key.

**Confirmed, not assumed: plain `mv` has nothing to report here.** `links::mentions`'s own scan
(`mention_shape`/`looks_like_key_shape` in `crates/typdoc-core/src/links.rs`) only ever recognizes
`^[A-Z][A-Z0-9]*-\d+$`-shaped tokens (optionally prefixed) — never a path. Separately, `Project::mv`
itself refuses outright, before `mv_reverse_scan` ever runs, whenever `from_key` is `Some` ("a
coded document's path is fixed by its key... use `mv --renumber` instead") — so a plain `mv`'s
`from` is *always* uncoded by the time the reverse scan runs. Since a mention can only ever name a
key, and plain `mv` can never touch a document that has one, plain `mv` structurally can never
produce a `reason: mention` entry. Proved directly in
`a_plain_mv_never_reports_a_mention_since_the_moved_document_has_no_key`
(`crates/typdoc/tests/mv.rs`): an uncoded document is moved while an unrelated coded document's
key is mentioned in plain text elsewhere in the project — `unrewritten` stays `[]`.

## Verification

**Red-then-green proof:** wrote the new tests first, then `git stash push -- crates/typdoc-core/
src/project.rs` to run them against the pre-fix code:

```
renumber_lists_a_plain_text_mention_of_the_moved_key_in_unrewritten ... FAILED (unrewritten: 0, expected 1)
renumber_prints_the_mention_entry_in_the_text_golden ... FAILED (unrewritten: none, expected 1 entry)
renumber_still_rewrites_a_formal_ref_while_separately_reporting_a_mention ... FAILED (unrewritten: 0, expected 1)
a_subsequent_validate_run_agrees_with_the_mention_mv_already_reported ... FAILED (mv side: reason was Null, expected "mention")
```
22 of the other pre-existing/negative tests in that file still passed unchanged. `git stash pop`
restored the fix; the same run is now:
```
running 26 tests ... test result: ok. 26 passed; 0 failed
```
(`cargo test -p typdoc --test mv --test mv_renumber`, both files, 24 + 26 = 50 passed, 0 failed.)

**Full test list added** (`crates/typdoc/tests/mv_renumber.rs` unless noted):
1. `renumber_lists_a_plain_text_mention_of_the_moved_key_in_unrewritten` — the exact repro,
   `--json`: one `unrewritten` entry, `reason: "mention"`, `field: "$body"`, `written: "WF-5"`,
   `line: 5`/`col: 5`; holder file byte-for-byte unchanged (never rewritten).
2. `renumber_prints_the_mention_entry_in_the_text_golden` — same repro, text-mode golden.
3. `renumber_does_not_report_a_mention_of_an_unrelated_key` — negative: a different key mentioned
   elsewhere is not reported.
4. `renumber_still_rewrites_a_formal_ref_while_separately_reporting_a_mention` — the same holder
   carries both a formal `see: WF-5` ref and an unrelated plain-text mention; the ref is rewritten
   to `story-3:WF-1` as always, the mention is left as written and reported separately.
5. `a_subsequent_validate_run_agrees_with_the_mention_mv_already_reported` — with `body.mentions`
   turned on (`"level": "warn"`), `mv --renumber`'s report and a follow-up `validate --json` both
   agree: `mv` says `reason: mention`, `validate` says `body.mentions ... WF-5 not found` at the
   same holder/path.
6. `crates/typdoc/tests/mv.rs`:
   `a_plain_mv_never_reports_a_mention_since_the_moved_document_has_no_key` — plain `mv`'s own
   confirmation, described above.

**Gates**, run at the end from a clean worktree state:
- `cargo fmt --check` — clean (one formatting pass needed after the initial write; re-ran clean).
- `cargo clippy --workspace --all-targets -- -D warnings` — clean, no warnings.
- `TMPDIR=/home/thw-home/.cache/typdoc-tmp scripts/test.sh` — `1026 test(s) passed across 56
  suite(s) on linux`, 0 failed.

## Notes

- Docs updated: `docs/reference/commands.md` (`unrewritten`'s reason list now reads
  `imported-project`, `mention`, `links-rule-off`); `CHANGELOG.md`'s existing `mv` bullet under
  `[0.2.0]` amended in place with the same reason list and a note that `mention` is found and
  reported at move time. `skills/typdoc/references/commands.md` already documented `mention`
  correctly (written ahead of this bug fix, per the design) — left untouched.
- Also found and fixed `docs/how-to/move-and-rename.md`, which was not in the ticket's explicit
  doc list but directly described the old (buggy) behavior as permanent design ("Mentions... are
  never refs, so `mv` never rewrites them... `typdoc validate` reports mentions... and you can fix
  them by hand" — with no `mention` reason listed at all). Updated it to add the `mention` bullet
  and reword the trailing paragraph to describe `validate` as agreeing with `mv`'s own report,
  not as the only place a mention is ever caught.
- `mv_reverse_mentions`'s per-document ordering is holder-path order, same as the formal-refs
  scan; mention entries are appended after all formal-ref-derived entries (`LinksRuleOff`) rather
  than interleaved by path when a single holder has both kinds. No existing or new test depends on
  a specific relative order between the two kinds, and I found no design text mandating one, so I
  left it as the simpler two-pass append rather than merging and re-sorting.
