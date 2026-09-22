# Contract

This contract does not restate the design. `docs/design.md` specifies the behaviour, and where the two differ the design wins (project rule 6). The contract says which parts of the design story 2 implements, which decisions bind it, what it decides that the design does not, and what it leaves open. The decisions are in `docs/design-decision-phase-2/`, and a ticket is named by its number there; a ticket of the earlier set is named as `phase 1, ticket N`. How the story is tested is in `testing-decisions.md`.

## What story 2 implements

| Part | Where the design specifies it | For this story |
| --- | --- | --- |
| `new` | `typdoc new`, State, Match templates | Allocation under the namespace's lock, defaults and `auto` fields filled, validation before the write, the file created with `O_EXCL`, the key printed bare on stdout |
| `set` | `typdoc set` | Fields written, `--if` decided under the same lock as the write, `auto: update` stamped when a value changes, `k=` removing a field |
| `mv` | `typdoc mv`, Across namespaces | The move and the rewrite of every ref this project holds, in every namespace of it; temp files prepared first and renamed in one run; the document moved last; refusals for the cases 15 and 16 name |
| `mv --renumber` | `typdoc mv` | The destination as the flag's value, the new number issued from the destination's `last`, that `last` written before the document appears, the new key printed |
| Writing frontmatter | Document files | The whole block rewritten by `yaml_serde` from the text the read path keeps; the losses the design's table names, and no value changed (20) |
| Locks | Concurrency | The namespace lock, `O_EXCL`, retry with backoff to `--lock-timeout` then exit 4, no takeover, the identity check before removal, one order for taking more than one (3), the `git-common` path and its hash (14) |
| Signals | Concurrency, Exit codes | `SIGINT` and `SIGTERM` caught, the lock released, the process ended by the signal (6) |
| The state file | State | Written by `new` and by `mv --renumber`; the form of a file typdoc creates; the rules `state.malformed`, `state.behind` and `state.retired` (13) |
| Atomic writes and temp files | Concurrency | The temp file's reserved name shape, the walker skipping it, a leftover as a `warn` finding with its place in the audit, the mode carried across, removal only under a lock (4) |
| Output | JSON output, Exit codes and errors | The shapes of `new`, `set` and `mv` (17); exit codes 3, 4 and 7 |
| Validation on write | Validation rules | `frontmatter.transitions`, the one rule the read core does not check |

Not built here: `pull`, the fetch adapter, `vendor/` and `lock.json` as files typdoc writes, and the project lock. There is still no command that deletes a document.

## Decisions that bind the story

| Ticket | Binds story 2 on |
| --- | --- |
| 1 | `mv` is not all or nothing; temp files first, renames last, the document last of all; the same command is the way back; what it cannot rewrite is a success with a report |
| 2 | No write crosses a project boundary; an imported project is read-only with no exception, and neither `mv` argument may carry a `project::` prefix |
| 3 | Locks are taken in the order of the lock files' own paths, the project lock first |
| 4 | What an interrupt leaves; the temp file's reserved shape; a leftover as a `warn` finding; removal only under a lock, never by age, and never failing the write; the mode carried and what is not |
| 5, 6 | `signal-hook`; one lock-acquisition path proved by a type; the handler only wakes a thread; the process ends by the signal; the window inside the creating call stays open |
| 7 | `Deps` gains file operations rather than document operations; one function above the seam holds the policy; `new` keeps `O_EXCL`, `mv` uses a plain rename; the clock gives an instant and an offset |
| 10 | A `number` is written unchanged and printed with the digits the document holds; the read side is fixed in this story |
| 11 | `typdoc mv FROM --renumber NAMESPACE`; the same namespace refused; the key printed; no cross-project renumber; `last` never lowered |
| 12 | Source and destination compared by file identity, not by text; one file means exit 7 |
| 13 | The form of a state file typdoc creates; `state.malformed`, `state.behind`, `state.retired`; nothing repaired and no entry removed; `--renumber` writes one state file |
| 14 | The `git-common` project hash, its stability, and the collision accepted |
| 15 | A destination that exists is refused at exit 7, checked under the lock, `new` creating with `O_EXCL` |
| 16 | Which moves across collections are refused, which are carried out and reported, and that the reported one exits 0 |
| 17 | The `--json` shapes of `new`, `set` and `mv`, including `unrewritten` and `findings` |
| 20 | `yaml_serde` writes frontmatter; a write rewrites the whole block; no re-read guard |
| 21 | No warning on a write that drops something; a field written with no value and one written empty are kept apart |
| phase 1, ticket 1 | Frontmatter is read into typed values; the writer is reached through the trait, which stays |
| phase 1, ticket 2 | The next number is the larger of the highest that exists and `last`, plus one, written back under the lock |
| phase 1, tickets 7, 9, 13, 14, 15, 16, 22 | The lock rules and the platform; the test strategy and which story builds what; namespaces; the exit codes and the criterion for adding one; how a document is named in `--json`; path and key arguments; paths compared with their case |

## What this contract decides that the design does not

1. **The commands and rules the binary has.** The registry gains `new`, `set` and `mv`, which leave `unimplemented_commands`; exit codes 3, 4 and 7 leave `unproduced_exit_codes`; and `frontmatter.transitions`, `state.malformed`, `state.behind` and `state.retired` leave `unimplemented_rules`. A rule leaving that list brings the requirement that it has a fixture which turns it red, so the fixture lands in the same change as the rule.
2. **Where the file system may be touched.** Story 1 made "the read core changes nothing" a property of the code: `typdoc-core`'s `clippy.toml` disallows every function that writes. Story 2 must write, and the ban does not simply go: it narrows. The disallowed list stays as it is, and exactly one module — the one that implements the write half of the seam — carries an `#[expect(clippy::disallowed_methods, reason = ...)]` naming itself as the seam, in the shape the lint from phase 1's ticket 29 already requires. Every other module of `typdoc-core` remains unable to write, and that is checked by the lint rather than by care. Default, if a per-module expectation proves impossible with the lint as configured: the seam's implementation moves to its own crate, which is the only crate allowed to write, and the ban on `typdoc-core` stays whole.
3. **A fixture whose command writes is never run in the repository.** `fixture.json` already declares the command to run, and every fixture today declares a read. A fixture that declares a write is copied to a temporary directory and the copy is what runs, so the fixture in the repository is never modified. The loader enforces this rather than each test remembering: a fixture whose command is a write and which is about to run in the repository's own tree is a failure of the harness, not a silently mutated fixture. `frontmatter.transitions` is the first rule that needs this, because it is checked on write and cannot be reached by `validate`.
4. **The two pinned gaps close here.** `[number-text]` closes because the write commands print `get`'s object and cannot be right while it is wrong; `[empty-value]` closes because a write puts back the form it found. Each is pinned today by a test on both sides — the binary's present behaviour and what the design asks for — so closing it turns that test red and its entry is removed in the same change. Closing `[number-text]` rewrites every golden that holds a number; the goldens are regenerated by the generator, whose guard still holds, and the assertion files stay hand-written.
5. **`Clock` in `deps`.** `deps` gains `Clock`, which gives an instant and the offset to write it in, at one second. The injected clock in tests returns a fixed instant, so a golden file can hold a time. `std::time::SystemTime::now` and `chrono`'s clock functions join the disallowed methods of `typdoc-core`, in the same shape as `std::env::var`, so that a time cannot be read except through `deps`.
6. **The lock is a value, not a convention.** Acquiring a lock returns a value that no other code can construct: it has no public constructor and no public fields, and the acquire function is the only function that returns it. Every function that writes takes it by reference, so a write outside the lock does not compile. What proves it is in `testing-decisions.md`; that it is proved rather than asserted is the part that belongs here, because the signal test in a shipped binary is a test of every command only if there is one path for it to be a test of.
7. **The reserved temp-file shape lives in one place.** The shape decision 4 fixed is a constant in `typdoc-core`, and both the writer that creates such a file and the walker that skips it read that one constant. A second spelling of the shape is the way the walker comes to miss what the writer makes.
8. **What a write does when the document is outside every namespace.** Story 1 decided that a document in no namespace folder is reachable by a ref and is named by its path alone. A write command reaches it the same way: `set` and `mv` accept it, `new` cannot create one, since `new` requires a collection and a file outside every namespace is in none. Default, because the design does not say it: a `set` on such a file is an ordinary write, and its `--json` leaves `namespace` out as `get` already does.

## Not decided by this contract

- **Durability.** A temp file and a rename give atomicity, not durability: without an `fsync` before the rename a power cut can leave the new file in place and empty. Whether typdoc syncs before renaming, at what cost, and whether the containing directory is synced too, is left open by decision 4 and is not settled here. It is written down rather than left silent, and the ticket that builds the write may raise it.
- **The text output of `new`, `set` and `mv`.** The design gives `new` a bare key on stdout and says nothing about the other two. `validate`'s text output does not exist yet either. The shape of the non-`--json` output is decided by the ticket that builds each command, and recorded there.
- **`--lock-timeout`:** which commands accept it, and whether giving it to a command that takes no lock is bad arguments or is ignored.
- **The retry schedule** behind "retries with backoff", and whether a caller can tell a timeout from a lock that was never contended.
- **How `--set k=v` is parsed on the write path:** the escapes for a comma inside an array value, an empty value, a value that looks like a number, and whether `k=` removing a field is available to `new` as it is to `set`.
- **Whether `set` may write a field that no schema in scope defines.**
- **The order of filling defaults, filling `auto` fields, validating and taking the lock in `new`.** The design gives the steps in one sentence and not which of them happen inside the lock.
- **How exit code 6 is produced in a test**, unchanged from story 1: if no deterministic way is found it stays in `unproduced_exit_codes`.
- **Crates that `.chief/project.md` does not list.** The ticket that needs one chooses it and adds it there. `signal-hook` is chosen by decision 6 at an ordinary version requirement, not an exact pin.

## Constraints

- The workspace and the toolchain are unchanged: Rust 2024 on stable, `crates/typdoc-core` the library and `crates/typdoc` the binary, with `crates/typdoc-testkit` for the harness.
- Linux is the platform that is run and claimed. The story decides what a `mv` does where a file system does not tell two spellings apart, which is a behaviour and not a claim about a platform.
- Before every commit, `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` and the public-text check pass. Tests run offline and never skip silently when a file is missing (project rules 1 and 2).
- Non-test code in `typdoc-core` may not `unwrap`, `expect`, `panic!`, `unreachable!`, `todo!` or `unimplemented!` without an `#[expect(...)]` whose reason is the evidence, and a write path has more places where that is tempting than a read path has.
- Every command has `--json` and uses the exit codes of the design's table (project rule 5).
- The story writes the user's files. Where a decision could be read two ways, the reading that cannot damage a file is the one built.
