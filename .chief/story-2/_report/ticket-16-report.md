# Ticket 16: the acceptance run and the closing report

Resolved. Four gates green on the committed tree, each re-run after the commit: `cargo fmt
--check` clean, `cargo clippy --workspace --all-targets -- -D warnings` clean, `scripts/test.sh`
with `TMPDIR` on a disk-backed folder: 979 passed / 0 failed / 1 ignored (unchanged from the
baseline, since this ticket adds no test), public-text check clean. One unrelated hit was found
before this ticket's own commit and fixed as a separate commit first (below).

## Outcome

done

## What this ticket is

Not a feature: the round trip of the contract's first criterion (`typdoc set` of one field, then
a read of every other field, compared with what it held before), already run automatically over
`fixtures/` by `cargo test`, run once more by hand against copies of two real, public
repositories the automatic check never reaches, plus the story's closing report. No seam, no
red/green step: the shipped binary and the write path are already built by tickets 1 to 15, and
this ticket exercises them rather than changing them.

## The acceptance run

The binary is `target/release/typdoc`, built from this branch's tip. Each repository
(`/home/thw-home/gits/thaitype/chief` at `dbb8db5` on `docs/v5-alpha-announcement`,
`/home/thw-home/gits/thaitype/typmem` at `652e34e` on `main`) was copied to a scratch folder
outside every repository this work touches, and nothing was written to either original:
confirmed both by diffing each copy against its original (identical outside `.typdoc/` and
`schemas/`, both added only in the copy) and by `git status` on the originals themselves showing
no change this run caused.

A `.typdoc/config.json`, one collection file per repository (two for `typmem`, since its skill
files sit in two different folders) and one schema were written into each copy, matching documents
each repository actually has: every `SKILL.md` under `skills/` (`chief`) or under `skills/` and
`.agents/skills/` (`typmem`), each holding a plain YAML frontmatter block of exactly two fields,
`name` and `description`, both strings, both present on every file. `typdoc validate --json`
against each copy, before any write, reports zero findings — the schema fits the corpus rather
than partly rejecting it. No other file in either repository (`AGENTS.md`, `README.md`, and so on)
was pulled into a collection, since neither carries the frontmatter shape the schema names.

For every matched document, in both copies: `typdoc get <path> --json` (before), `typdoc set
<path> name=<the value just read>` (a real write — the value does not change, only the round trip
through frontmatter is exercised), then `typdoc get <path> --json` (after). Every field of every
document was compared, not only the one named in `--set`.

**Results.** `chief`: 15 documents, 15 with every field reading back identical, 15 with the file's
bytes unchanged. `typmem`: 16 documents, 16 with every field reading back identical, 15 with the
file's bytes unchanged and one, `skills/typmem-judge/SKILL.md`, with its bytes changed. Across
both copies: 31 documents round-tripped, 31 with no value differing.

**The one file whose bytes changed.** Its `description` was written as a YAML folded block scalar
(`description: >-`, the text indented on the following lines); after the write it is a single-line,
single-quoted scalar holding the identical text (verified by the JSON comparison above: the two
reads are byte-for-byte identical). The design's own table of what a write loses (`docs/design.md`,
the section on writing frontmatter) names seven shapes — comments, blank lines, a flow list
becoming a block list, an unneeded quote being dropped, extra spacing after a colon collapsing to
one space, an anchor/alias expanded at every site that used it, a YAML tag dropped — and a block
scalar folded across lines is not one of them; the fixture built to hold every shape that table
names (`fixtures/valid/frontmatter-losses/shapes.md`, per the contract's testing decisions) does
not hold one either, so the automatic corpus check has never exercised this shape. The paragraph
introducing that table already frames the promise narrowly — the written *form* is kept only as
"a best effort" and "not a promise"; only the *value* is promised, and the value held exactly
here. Read that way, this is not a broken promise. It is still worth naming exactly, since it is a
real, table-uncovered shape a fixture never held, found only because this run used real files
rather than fixtures built to the table's own list.

The full method, results and this finding are also recorded in the story's closing report, since
they are the story's evidence and not only this ticket's.

## The closing report

`.chief/story-2/_report/closing-report.md` (new file). It records: the acceptance run above; the
contract items the ticket names as still undecided by the contract itself, each traced to the
report of the ticket that made the practical choice in the build (durability without `fsync`,
text output of `new`/`set`/`mv`, where `--lock-timeout` is accepted, the backoff schedule, how
`--set` parses, whether `set` may create an unknown field, the order inside the lock in `new`,
and exit code 6); that `KNOWN_GAPS` holds only `[reverse-scope]` and `[import-anchor]`, checked by
reading `crates/typdoc/src/registry.rs` directly; that no ticket in this story amended
`docs/design.md` (checked by `git log` over every `story-2/ticket-*` commit against that file, and
over every commit since ticket 1's claim, both empty); and what the story leaves untouched.

## A public-text hit found before this ticket's own commit

`.chief/story-2/_report/ticket-14-report.md` used a phrase matching the gate's generic pattern for
wording that reads as naming who confirmed something. Reworded to "verified by reading the code",
which the original sentence meant, and committed separately
(`fix(story-2/ticket-16): reword a public-text gate hit found before this ticket's commit`),
before this ticket's own commit, the same way ticket 15 fixed an identical hit in its own report.

## Checked by running

- `typdoc validate --json` against each copy before any write: zero findings in both, so the
  schema written for the run fits the real documents rather than partly rejecting them.
- The full round trip above, against the shipped binary, by hand: 31 documents, 31 with every
  field identical before and after, one with its bytes changed and every value in it unchanged.
- Both copies' originals unchanged: `diff -rq` against each original (excluding the `.typdoc` and
  `schemas` folders written for the run) reports no difference, and `git status` on each original
  repository shows nothing this run touched.
- All three lists and `KNOWN_GAPS`, read directly rather than assumed: `UNIMPLEMENTED_COMMANDS`
  holds only `"pull"`; `UNPRODUCED_EXIT_CODES` and `UNIMPLEMENTED_RULES` are both empty;
  `KNOWN_GAPS` holds exactly `[reverse-scope]` and `[import-anchor]`.
- `git log` confirms no commit tagged `story-2/ticket-*` touches `docs/design.md`, and no commit
  at all touches it since ticket 1's claim.

## Guards

This ticket adds no code to `typdoc-core` or `typdoc-fs` and no test — only two report files and
the one-line reword above — so the write-ban plant-and-revert demonstration does not apply, the
same reasoning ticket 15's own report gives for the same situation, and is not staged against
unrelated code to manufacture a false positive.

## Notes

- The scratch copies made for the acceptance run are not part of this commit and are not added to
  the repository, per the ticket's own instruction; they can be deleted after this report is read.
- Nothing in this ticket needed a decision the design leaves silent beyond the ones the acceptance
  run itself found (above); nothing here contradicts the design or the contract.
