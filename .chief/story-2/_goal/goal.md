# Goal

Story 2 of the three that deliver typdoc v1. It builds the write path: the part of typdoc that changes a project rather than reporting on it. `docs/design.md` is the source of truth for every behaviour named here; this goal adds no requirement that the design does not state. The decisions taken before planning are in `docs/design-decision-phase-2/`, and the shared base those rest on is `docs/design-decision-phase-1/`.

When the story is done, a person or a program can point typdoc at a project story 1 can already read and:

- create a document with `new`, either under a key the tool allocates for a coded collection — the next number after the larger of the highest that exists and the highest ever issued, taken and recorded under the namespace's lock — or at a path they choose, with defaults and `auto` fields filled in and the document validated before anything is written;
- change fields with `set`, with `--if` deciding before the write so that a condition and the write it guards cannot be separated by another process;
- move a document with `mv`, which rewrites every ref this project holds to it, in frontmatter and in body links, in every namespace of the project, keeping each ref's written form; and move a coded document to another namespace under a new key with `mv --renumber`;
- rely on the output of all three: each has `--json` in the shapes the decisions fix, each prints its result as the same document object `get` prints, and each ends with an exit code from the design's table, including the three no read command can produce — 3 for an `--if` that was false, 4 for a lock not acquired, and 7 for a destination that already exists;
- be interrupted without losing a document: each file is written whole or not at all, a lock is released on the interrupt signals, and the process ends by the signal rather than with an exit code of its own.

The story is done when all three of these hold:

- **A write changes only the fields it is given.** Run over every document in the fixtures and over copies of the public `chief` and `typmem` repositories with a `.typdoc` folder written for the run, a `set` of one field leaves every other field reading back exactly as it did before. The differences in the text are only the ones the design's table names as lost — comments, blank lines, flow style, quote style, anchors, aliases and tags — and no value differs. This is the part of "a write is safe" a machine can check, and it is the one that catches a writer that quietly reinterprets a value on its way back to disk.
- **Two writers never issue one key.** Several processes running `new` at once against one namespace produce as many distinct keys as there were processes, with the state file's `last` equal to the highest of them, and none of them writing a document over another's.
- **A program can drive `new`, `set` and `mv` from their JSON and their exit codes**, without parsing text, for both the cases that succeed and the cases that refuse.

The story also empties the write half of the lists that record differences between the design and the binary: `new`, `set` and `mv` leave `UNIMPLEMENTED_COMMANDS`; exit codes 3, 4 and 7 leave `UNPRODUCED_EXIT_CODES`; and `frontmatter.transitions`, `state.malformed`, `state.behind` and `state.retired` leave `UNIMPLEMENTED_RULES`. An entry that outlives its work turns the suite red, so the lists are part of the story finishing rather than a note about it.

It writes documents, state files and locks, and nothing else. It makes no network request, fetches no schema, writes no `lock.json` and no `vendor/`, and takes no project lock; those belong to story 3. It adds no command that deletes a document, because v1 has none.

## Constraints that hold for the whole story

- Two differences between the design and the binary that story 1 left pinned are closed by this story, not worked around: `[number-text]`, where a `number` is printed converted out of its text rather than with the digits the document holds, and `[empty-value]`, where a field written with no value and one written as an empty string are read as the same thing. Both are on the write path's own account — the first because the write commands print `get`'s object and cannot be right while it is wrong, the second because a write must put back the form it found. Each is pinned today by a test on both sides, so closing it turns that test red and the entry has to go.
- Closing `[number-text]` changes every golden file that holds a number. That cost is accepted because nothing has been released; it is named here so that a large diff in the goldens is read as the change it is.
- Every rule the story builds has a fixture that turns it red, and the checks that keep that true are part of the story.
- The story writes the user's files. Where a decision could be read two ways, the reading that cannot damage a file is the one the story builds, and a partial or doubtful write is refused rather than reported afterwards.
- The lint that story 1 added still holds for everything this story writes: non-test code in `typdoc-core` may not `unwrap`, `expect`, `panic!`, `unreachable!`, `todo!` or `unimplemented!` without an `#[expect(...)]` whose reason is the evidence.
- Every line of the repository reads as its owner's own work (project rule 7); the story adds nothing that does not.
