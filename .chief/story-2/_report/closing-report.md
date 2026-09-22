# Story 2 closing report

## What the story delivers, as measured

`new`, `set` and `mv` (including `mv --renumber`) write a project story 1 can already read: create
a document under an allocated or a chosen path, change fields under `--if`, and move or renumber a
document while rewriting every ref this project holds to it. Sixteen tickets, all resolved, one
report per ticket in `.chief/story-2/_report/`. `scripts/test.sh`, run under the memory ceiling with
`TMPDIR` on a disk-backed folder: 979 passed, 0 failed, 1 ignored.

The three lists that track differences between the design and the binary end this story at:
`UNIMPLEMENTED_COMMANDS` holding only `"pull"` (story 3's), `UNPRODUCED_EXIT_CODES` empty,
`UNIMPLEMENTED_RULES` empty — verified by reading `crates/typdoc/src/registry.rs` and
`crates/typdoc-core/src/rules.rs` directly. `KNOWN_GAPS` holds exactly two entries,
`[reverse-scope]` and `[import-anchor]`, also verified by reading `crates/typdoc/src/registry.rs`
directly: both were already there at the end of story 1, and this story's own goal says plainly it
does not touch either, so nothing here was expected to close them.

## The three criteria of the goal

**A write changes only the fields it is given.** Run automatically, as part of `cargo test`, over
every document in `fixtures/valid/`: a `set` of one field, then every other field read back and
compared with what it held before. `fixtures/valid/frontmatter-losses/shapes.md` was built to hold
every shape the design's table of losses names — a comment, a blank line, a flow list, both quote
styles, an anchor with its alias, a tag — so the check has something to bite on for each row of
that table. Run once more by hand, at the end of the story, against copies of two real, public
repositories the automatic check does not reach (method and full results below): 31 documents
across both, 31 with no value differing.

**Two writers that take the same lock never issue one key.** Made to happen twice over, not hoped
absent: deterministically, through the seam, with a fake file system paused at the one instant a
lost update could occur; and across real processes, with a shipped binary holding the lock and
several `new` processes started before it is released. Both were shown able to fail, by replacing
the fresh, under-lock read of `last` with a stale one and watching both tests go red with the exact
collision. Two worktrees under the `git-common` lock mode are outside this criterion, as the goal
itself says; what was built there instead is the narrower, buildable claim the design also makes —
two ordinary projects allocating the same key with nothing coordinating them, and `validate`
reporting the duplicate once both are visible together — since no write command wires that lock
mode yet, stated as narrower rather than passed off as the wider claim.

**A program can drive `new`, `set` and `mv` from their JSON and their exit codes.** Every one of the
three has `--json` in the shape `get` already prints, plus what each adds: `mv`'s `unrewritten` and
`findings`. Exit codes 3 (`--if` false), 4 (lock not acquired) and 7 (destination already exists)
are each produced by a test through a real command; goldens exist for all three commands, with a
fixed clock so an `auto` field's stamp can sit in a golden file.

## The acceptance run against real repositories

Run by hand against copies of `/home/thw-home/gits/thaitype/chief` (at `dbb8db5`, branch
`docs/v5-alpha-announcement`) and `/home/thw-home/gits/thaitype/typmem` (at `652e34e`, branch
`main`), each copied to a scratch folder outside every repository worked on, with nothing written
back to either original — verified by diffing each copy against its source outside the `.typdoc`
and `schemas` folders written for the run, and by `git status` on both originals showing no change
this run caused. The binary is `target/release/typdoc`, built from this story's branch.

A `.typdoc/config.json`, one collection per real folder of skill files, and one schema (`name` and
`description`, both strings, both required) were written into each copy, matching a set of
documents each repository actually has: every `SKILL.md` under `chief`'s `skills/`, and every
`SKILL.md` under `typmem`'s `skills/` and `.agents/skills/`. `typdoc validate --json` against each
copy, before any write, reports zero findings in both, so the schema fits the real corpus rather
than partly rejecting it.

For every matched document, in both copies: read with `typdoc get <path> --json`, write with
`typdoc set <path> name=<the value just read>` — a real write (a candidate is built, validated and
put through `write_atomically` under the namespace lock exactly as any other `set` is), with no
value actually changing — then read again with `typdoc get <path> --json`. Every field of every
document was compared, not only the field named in `--set`.

**Counts.** `chief`: 15 documents matched, 15 with every field reading back identical, 15 with the
file's bytes unchanged. `typmem`: 16 documents matched, 16 with every field reading back identical,
15 with the file's bytes unchanged and one changed. Combined: 31 documents round-tripped, 31 with
no value differing — the criterion the contract asks this run to check held on every document.

**The one value-safe difference, named exactly rather than summarized.** `typmem`'s
`skills/typmem-judge/SKILL.md` holds its `description` as a YAML folded block scalar
(`description: >-`, the text on the following, indented lines); after the write it is a single
line, single-quoted, holding the identical text — confirmed identical by the JSON comparison, not
only by inspection. The design's table of what a write loses (`docs/design.md`, the section on
writing frontmatter) names seven shapes: comments gone, blank lines gone, a flow list becoming a
block list, an unneeded quote dropped, extra spacing after a colon collapsing to one space, an
anchor and its alias expanded to the value at every site, a YAML tag dropped. A block scalar folded
across lines reformatted to a single-line quoted scalar is not one of the seven, and
`fixtures/valid/frontmatter-losses/shapes.md`, built to hold every shape that table names, does not
hold this one either — the automatic corpus check has never exercised it. The paragraph
introducing the table already frames the promise narrowly: the written *form* is kept only as "a
best effort" and is "not a promise"; only the *value* is promised, and here the value held exactly.
Read that way this is not a broken promise, but it is a real, table-uncovered shape that a fixture
never happened to hold, and it is named here rather than folded silently into "expected losses."

## What the contract left undecided, and what the build actually chose

The contract (`.chief/story-2/_contract/testing-decisions.md`) leaves eight things undecided. None
of them was closed by amending the contract or the design; each was decided as a practical,
build-time choice, recorded in the report of the ticket that made it, still visible by reading the
shipped code today:

- **Durability without `fsync`.** The design says plainly that whether typdoc syncs before renaming
  is not decided (`docs/design.md`, Atomic writes). Ticket 1 put `sync` on the `Fs`/`WriteHandle`
  trait and its one real implementation, and calls it from nowhere: no write path syncs before its
  rename today, and the operation being present rather than absent is what lets the decision go
  either way later with no interface change.
- **The text (non-`--json`) output of `new`, `set` and `mv`.** Built only where the design gives a
  worked example, refused elsewhere as not built yet, one command at a time: `new`'s coded form
  prints the bare key (the design's own `# stdout: WF-3`), built in ticket 9; `new`'s path form
  refuses text output, since the design shows none for it. `mv --renumber` prints the bare key the
  same way (`# stdout for --renumber: WF-4`), built in ticket 12; a plain `mv <path>` refuses text
  output, built in ticket 11. `set` has no worked example in the design to build against, and its
  text output is refused in every case, decided in ticket 10's build. A refusal names itself
  plainly ("the output without `--json` is not built yet") rather than guessing a shape the design
  never gave.
- **Where `--lock-timeout` is accepted.** The design names the flag once, with a 5 s default, and
  never lists which commands take it. Ticket 3 left `acquire` taking a `Duration` with nothing
  wiring a flag to any command. Ticket 11 wired it for `mv` first, in practice, since at that
  branch point no earlier write command's flag had merged yet; by the end of the story all three
  write commands (`new`, `set`, both forms of `mv`) accept `--lock-timeout`, each with the same 5 s
  default.
- **The backoff schedule.** Ticket 3: 10 ms, doubling to a 200 ms cap, further capped by whatever
  time is left to the caller's deadline. Not promised anywhere in the design or in typdoc's own
  documented behavior, so it can change without breaking a promise made to a caller.
- **How `--set` parses.** Ticket 10 built the one parser every write command's `--set`/`k=v`
  arguments share: `k=v` sets a field, `k=` removes it. Which side of `=` decides how the value is
  read is the field's type in the schema, not the syntax: a `list` or `ref[]` field splits its raw
  text on commas (an empty raw text gives an empty list, the same convention `list --where` and the
  design's own worked examples use); every other field, known to the schema or not, is set as the
  whole raw text, unsplit, since there is no type to say a comma should be read as a separator.
- **Whether `set` may create an unknown field.** Yes, unconditionally, decided in the same piece of
  ticket 10's build: a field the schema does not name is still written, as a scalar, the same way a
  known field is. `frontmatter.unknown` reports it afterward; the write itself does not refuse it.
- **The order inside the lock in `new`.** Ticket 9: allocate the candidate number in memory,
  validate the candidate, and only if it is valid, check the destination does not already exist and
  write the state file, then create the document. Validating first means a bad `--set` never burns
  a number; writing state before the document appears means a crash between the two only ever skips
  an ordinary number, matching `mv --renumber`'s own explicit rule for the same ordering.
- **Exit code 6.** Ticket 4 mapped it to a startup failure of the signal handler's own installation
  (`signals::install()`), the closest meaning the design's table has for a failure of the
  environment at startup, and marked it plainly, in the code's own comment, as not verified: nothing
  in this environment can make that installation call fail to watch the failure branch actually run.
  That comment is unchanged today (`crates/typdoc/src/main.rs`) — exit 6 is still the closest table
  entry rather than a checked one.

## Whether the design was amended, and what the story left as it found it

No ticket in this story amended `docs/design.md`. Checked directly: no commit tagged
`story-2/ticket-*` touches that file, and no commit at all touches it from ticket 1's claim commit
onward. Every "resolve decision N" commit that does touch the file predates ticket 1's claim — it
belongs to the planning that produced this story's contract, not to a ticket's own build. Project
rule 6 requires a deviation from the design to change the design first; since the design was never
changed, nothing the story built deviates from it. Wherever the design leaves a detail silent — an
argument order the design's own decision-map calls "not yet specified", a lock's own wiring, how a
parser splits a value — the build chose a reading and recorded it in the ticket's own report (the
section above), without the design's words needing to change to permit it.

What the story leaves exactly as it found it: the read path story 1 built (`get`, `list`, `toc`,
`refs`, `validate`), the project, schema and config file formats, and the two entries `KNOWN_GAPS`
already held at the end of story 1 — `[reverse-scope]` (a reverse lookup scans this project's own
namespaces only, not those of a project it imports) and `[import-anchor]` (a body link across an
import has its target file checked and its `#anchor` left unchecked). The goal states plainly that
this story does not touch either, and nothing in the sixteen tickets did.

## Not done, on purpose

`pull`, remote schemas reached by a test, the offline tripwire, and the project lock are story 3's,
named in `UNIMPLEMENTED_COMMANDS` and in the testing decisions' own "Built by the other stories"
section. v1 has no command that deletes a document, so none was added here.
