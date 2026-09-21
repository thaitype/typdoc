## Destination

Story 2's fog is clear when every decision the write path needs is made and written here, so that the story's goal and contract can be written without grilling any of it again: what `new`, `set`, `mv` and `mv --renumber` promise when they succeed and when they fail partway; what a run leaves on disk when it is interrupted; how locks are taken, ordered and released; what the frontmatter round trip guarantees byte for byte; and what the write commands print. Where a decision changes what `docs/design.md` says, the design is amended in the same change.

## Notes

- **Scope of story 2**, carried over from story 1's out-of-scope list: writing anything — `new`, `set`, `mv` and `mv --renumber`, the frontmatter round trip, locks, the state file's writing, and signal handling. Remote schemas (`pull`, `vendor/`, `lock.json` as a file typdoc writes, the project lock) are story 3.
- **Story 1 only read. Story 2 writes the user's files.** A read command that answers wrongly gives a wrong answer; a write command that behaves wrongly damages something the user owns. Decisions here lean towards making a partial or doubtful write impossible rather than towards reporting it after the fact.
- **`mv` has the widest blast radius of any command in v1.** One invocation renames one file and edits every file that refers to it, in every namespace of this project and in the projects it imports, keeping each ref's written form. A half-finished `mv` leaves a repository whose links partly point at the old path and partly at the new one. Tickets 1, 2 and 3 are about that, and they come first on purpose.
- **`docs/design.md` is the source of truth** (project rule 6): to deviate from it, amend the document first. Decisions here that change it are written into it in the same change.
- **Decisions from `docs/design-decision-phase-1/` that story 2 stands on, and does not reopen:** ticket 1 (read with `yaml_serde` into typed `String` fields, write with `yaml-edit` behind a trait of three operations, with a mandatory reparse guard and no automatic fallback), ticket 2 (next number = max(highest existing, `last`) + 1, written back under the lock, never reused), ticket 7 (lock rules, no takeover, exit 4), ticket 9 (the write seam lives in `deps`, `Clock` is story 2's, signals are tested by holding a lock in a shipped binary with no test code in it), ticket 13 (namespaces, and that a coded document cannot cross one except by `--renumber`), ticket 15 (one way to name a document in `--json`), ticket 22 (paths compare with their case).
- **The code as story 2 finds it.** `crates/typdoc-core/src/lock.rs` is `lock.json`, the pin file, not the namespace lock; the namespace lock has no module yet, and a second module named `lock` would read as the same thing. `Deps` holds `env` alone, so the write seam and `Clock` are both additions. `unimplemented_commands` and `unproduced_exit_codes` in the test suite hold what story 1 left undone; story 2 empties the write half of both, and an entry that outlives its work turns the suite red.
- **Open findings from story 1's review that land on the write path.** An integer past u64 in frontmatter is kept as text when read but becomes a float in `--json`, losing digits; an invalid `body.links` `ignore` glob is dropped silently; `links::find_on_line` is quadratic in the number of `[` on one line; the cycle walk in `refs.rs` recurses and overflows the stack on a long chain. The first is a data-loss risk once a command writes frontmatter back, and the third becomes a write-path cost because `mv` scans body links.
- **The lint from story 1's ticket 29 applies to everything story 2 adds:** non-test code in `typdoc-core` may not `unwrap`, `expect`, `panic!`, `unreachable!`, `todo!` or `unimplemented!` without `#[expect(clippy::<lint>, reason = "<the evidence>")]` naming the guard, the only constructor, or the caller that checks first.
- Tickets here use the inline `Type:`, `Status:` and `Blocked by:` lines, as story 1's decision tickets did.

## Decisions so far

<!-- index: one line per resolved ticket, enough to judge relevance, zoom the link for detail -->

- [Signal-handling crates](_tickets/5-signal-handling-crates.md): facts gathered, choice still open. `signal-hook` 0.4.4 is the only candidate that covers re-raising the signal (`low_level::emulate_default_handler`), moving the cleanup off the handler (`Signals`) and running without a runtime, in one dependency; `nix` 0.31.3 is the runner-up and `ctrlc` is ruled out by the re-raise requirement. Registering before the lock is created is possible with all of them, so that gap is closed by the order of the calls, not by the crate. Ticket 6 makes the choice.

## Not yet specified

- The text (non-`--json`) output of `new`, `set` and `mv`. The design shows `new` printing the bare key on stdout and says nothing about the other two.
- `--lock-timeout`: which commands accept it, and whether giving it to a command that takes no lock is bad arguments or is ignored. The design names the flag once, with a default of 5 s, and never says where it is accepted.
- The retry schedule behind "retries with backoff": whether the shape of the backoff is promised, and whether a caller can tell a timeout from a lock that was never contended.
- How `--set k=v` is parsed on the write path: the escape rules for a comma inside an array value, an empty value, a value that looks like a number, and whether `k=` removing a field is available to `new` as it is to `set`.
- Whether `set` may write a field that no schema in scope defines. Unknown fields are kept when read and reported by a rule; whether a write may create one is not stated.
- The order of filling defaults, filling `auto` fields, validating and taking the lock in `new`. The design gives the steps in one sentence but not which of them happen inside the lock.
- Distribution and versioning of the tool itself, and whether the suite must run from a published package. Unchanged from phase 1, blocking nothing here.
- Parked, blocking nothing and not raised again until someone takes it up: a macOS runner, and running the public-text check in CI or a hook.

## Out of scope

<!-- ruled beyond this story's destination — closed, never graduates -->

- Everything story 3 owns: `pull`, the fetch adapter, `vendor/` and `lock.json` as files typdoc writes, and the project lock that guards them. The one place they touch this story is the order in which a project lock and a namespace lock may be held, which the design already fixes and ticket 3 records rather than reopens.
- A command that deletes a document. v1 has nine commands and none of them deletes one; "a number is never reused after its document is deleted" describes a file removed by other means, not a command.
- Everything on the design's own "Out of scope for v1" list: editing body sections, SQL queries, saved query aliases, multi-hop ref traversal, imports of imports.
- Any release, packaging or distribution of typdoc.
- A claim that macOS is supported. Ticket 12 decides what a case-only `mv` does; it does not turn the code written for macOS into a supported platform, which needs a run on macOS that nobody has made.
