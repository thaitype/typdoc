# Goal

This story ships **v0.2.0**, without remote schemas: no fetch adapter, no project lock, no
`lock.json`/`vendor/` writes, no `pull` or `pull --check`. A remote schema with no pin stays
reported as `config.schema-unpinned`, as it is today. `v0.1.0` is already tagged at the point
story 2 merged.

## No program reads markdown to extract spec as machine data

Not to import it, not to check against it — comparing a generated table against a markdown
document is the same violation as importing one. `crates/typdoc-testkit/src/design.rs`, which
today parses `docs/design/design.md` into five sets — always-on rule ids, configurable rule ids,
command names, exit codes, and the table of what a write does not keep in a document's
frontmatter — is removed entirely, along with every line of code that reads it.

In its place: two kinds of documents, both managed by typdoc itself against a `.typdoc/` project
set up in this repository for the purpose.

- **`docs/design/catalog/`** — one typdoc document per concept (rules, commands, exit codes,
  frontmatter losses), each with a JSON-only body and a frontmatter field declaring that its body
  is JSON. The rules document holds one list of rule entries, each carrying a field for whether
  it is configurable, rather than two separate lists that could hold the same id twice or not at
  all and still parse. A helper reads such a document's body into typed data, deciding whether
  and how to validate it from that declared field, never from the document's location. The three
  places that read the old parser today — a test in `typdoc-core/src/frontmatter.rs`,
  `typdoc-core/tests/rules.rs`, and `typdoc/tests/coverage.rs` — read the new documents through
  this helper instead, still only from test code, and the coverage they already provide continues
  unchanged: a rule, command, exit code, or lost-shape row added to the catalog is a row the
  matching test reads too.
- **`docs/design/spec/`** — prose explaining the same rules, for people; code never reads it.

`docs/design/design.md` and the two decision registries at
`docs/design/design-decision-phase-1/` and `docs/design/design-decision-phase-2/` move, whole and
unedited, to `docs/archived-design/`, and nothing there is ever edited again — a record of what
was decided and when, frozen at the point of the move. A separate copy goes to
`docs/migrating-design/`, a working copy from which text is deleted as it is moved into the new
document layout; this story removes the code's dependency on markdown, it does not require
moving every remaining piece of content out of that working copy.

## A long ref chain no longer crashes the tool

`cyclic_nodes`'s inner walk (`crates/typdoc-core/src/refs.rs`) recurses one call per document
with no depth limit, and a long chain through an `acyclic` field overflows the stack — with no
cycle required at all, a one-way chain is just as deep. The walk becomes an explicit iterative
one over a heap-allocated stack, the same algorithm and the same output for a two-node cycle, a
self-loop, a chain with no cycle, and a cycle with a tail, with no ceiling on how long a chain can
be.

## Every command works without `--json`, and every value it prints is labeled

Today `new` (for a path-identified target), `get`, `set`, `refs`, `toc`, plain and `--schemas`
`validate`, and plain `mv` each refuse their output without `--json`, exiting 1 with "the output
without `--json` is not built yet." By the end of this story none of them do, under one
principle that holds for all of them: text output always shows field names — no bare, unlabeled
value.

- `get` prints a labeled block, one `name: value` line per field — `path`, `collection`,
  `schema`, `namespace` (and `key` when coded), then the frontmatter fields in file order. A
  `number` field prints with the document's own digits, the same rule the write path already
  holds itself to.
- `set` and `new` — both the path-identified form and, deliberately replacing today's built
  behavior, the coded form — print that same labeled block for the document as it now stands. A
  caller that only wants the bare key `new` used to print reads it out of `--json` instead, the
  same as any other field.
- `toc` is a table with a header row: `line`, `end`, `level`, `heading`, one row per heading.
- Both forms of `mv` — plain, and (deliberately replacing today's bare-key output) `--renumber` —
  print the destination's `get`-shaped block, then three lines always present: `rewritten: N refs
  in M documents` (a count); `unrewritten:` with its own count and one line per entry (the
  project, document, field, and written form); and `findings:`, listing its entries or the word
  `none`. `mv --json` gains the full list behind that `rewritten` count — one entry per rewritten
  ref, each naming the document, field, and its value before and after — additive to every field
  `mv --json` already prints. There is no `--verbose` flag: the detail lives in `--json`, and a
  human at a terminal already has it in `git diff`.
- `list` also changes: it gains a header row above the table it already prints (`path` or `key`,
  `title`, each `--where` field), present except when the result is empty; `--ids` is unchanged.
- `refs` and plain/`--schemas` `validate` needed no change to satisfy the principle: `refs`
  already names each ref's field inline, and `validate` already prints one line per finding,
  using the same finding shape `--json` and `--audit` already expose.

Every shape above that changes what the design currently states is written down as a change,
not a silent deviation: `docs/design/design.md` moves to `docs/archived-design/` this story and
is never edited again, so the correct, current shape for anything changed here is recorded in
`docs/design/spec/` and in whichever user docs already show that command's output
(`docs/commands.md`, `docs/getting-started.md`, `README.md`), not as a fix to the archived copy.

**An error prints as text in text mode, not as the `--json` error object.** Verified today,
reproducible on `main` without `--json`: a bad `typdoc new` target (for example
`typdoc new "not a code and not a path"`) prints the raw JSON error object
(`{"error":...,"code":1,"details":[]}`) to stderr instead of `typdoc: <message>`. The cause is
systemic, not one call site: every command this story adds text output to calls its error
formatter with a hard-coded `true` for "print JSON" (`crates/typdoc/src/cli.rs`, nine call sites
across `new`, `get`, `set`, `refs`, `toc`, and `mv --renumber`'s error path) rather than the
command's real `json` flag — safe only as long as those paths are unreachable without `--json`,
which stops being true the moment each "not built yet" gate is lifted. Building a command's text
output includes fixing its error path to the real flag, not only its success path.

## The release itself

- The toolchain is pinned to the version the project already builds and its gates already pass
  on; nothing here changes behavior, it only stops a future toolchain update from doing so by
  surprise without it being a deliberate, tested choice.
- Continuous integration runs, on every push and pull request into `main`, the same three checks
  a contributor runs locally before committing: `scripts/test.sh`, `cargo fmt --check`, and
  `cargo clippy --workspace --all-targets -- -D warnings`. No workflow step passes because it
  silently did nothing: each of the three is shown failing on a throwaway branch before its
  passing is trusted.
- Every crate's version rises to `0.2.0`, a changelog exists, and the README installs from the
  tag rather than from a clone of an unreleased branch.

## The user docs state where exact numeric comparison ends

A `number` value past what an `f64` holds exactly compares wrong with no error — two values that
differ only past the eighteenth digit can compare equal in a `--where` expression or a `--sort`;
measured on `62e6335`, `list --where 'count>99999999999999999998'` returns nothing even when a
document holds `99999999999999999999`. This story adds no comparison logic and no `validate`
finding for it; it only states the limit in the user docs, with this demonstration, so a reader
learns about it from the documentation rather than from a query that quietly returns the wrong
answer. Whether `date` and `datetime` — which also compare as instants — are affected the same
way is not measured; the doc line does not claim it either way unless this story measures it.

## Out of Scope

See `out-of-scope.md`.

## Done

The story is done when all of the following hold:

- `crates/typdoc-testkit/src/design.rs` no longer exists, and nothing in the workspace reads
  `docs/design/design.md` (or any file under `docs/archived-design/` or
  `docs/migrating-design/`) as data. `rg` for the old module's path and for the removed functions'
  names returns nothing outside history.
- The three tests that used to depend on `design.rs` still pass, now reading
  `docs/design/catalog/`, and still catch a rule, command, exit code, or frontmatter-loss row
  added to the catalog without a matching change to the code, the same way they do today.
- A regression test built on this branch, run on a thread with an explicitly small stack and a
  chain of at least 200,000 one-way `acyclic` references (no cycle), is shown failing before the
  rewrite and passing after it, in `scripts/test.sh`'s own output, not merely in a harness that
  exited early.
- `typdoc new` (both forms), `get`, `set`, `refs`, `toc`, `validate` (plain and `--schemas`), both
  forms of `mv`, and `list` each produce the text-mode result this document (and the contract)
  fixes for it, without `--json`; none of the previously-gated commands refuses with "not built
  yet" any more. Both `mv` forms print the same labeled block, `rewritten:`, `unrewritten:`, and
  `findings:` lines; `mv --json` carries the full `rewritten` list additively.
- `docs/design/spec/` and the user docs (`docs/commands.md`, `docs/getting-started.md`,
  `README.md`) show every shape this story changed, current and correct; `docs/archived-design/`
  is never edited to match, since it is a frozen record, not a followed document.
- Every one of those commands prints a plain-text error (`typdoc: <message>`) on stderr when it
  fails without `--json` — none of them prints the `--json` error object instead, including for
  the error paths that are today unreachable without `--json` and become reachable once each
  command's text mode is built.
- CI is green on `main` from a push that starts empty, having been shown red first on each of the
  three gates (`scripts/test.sh`, `cargo fmt --check`, `cargo clippy`) it runs, on the toolchain
  version this repository pins.
- Every crate reports version `0.2.0`; a changelog exists; the README's install instructions name
  the tag, not a clone of `main`.
- The user docs name, with the demonstrated example, where exact `number` comparison ends.
