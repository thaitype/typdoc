# Changelog

All notable, user-visible changes to typdoc are documented here. Internal reorganizations (for
example, how the project's own design documents are structured and read in its own test suite)
are left out unless they change something a user of the `typdoc` binary sees.

## [0.3.0] - 2026-09-24

### Added

- `namespaces` entries can now carry `!`-prefixed exclusions in the same list, gitignore-style
  (`["story-*", "!story-1", "!story-2"]`): patterns apply in list order, and the last pattern
  that matches a folder decides whether it's a namespace. An excluded namespace is fully
  invisible — not validated, not queried, a ref into it resolves as not found — and its
  `.typdoc/state/<namespace>.json` is left untouched while excluded, so re-including it later
  continues numbering with no reissued codes. `--namespace`/`TYPDOC_NAMESPACE` do not support
  `!`; passing one with a leading `!` still gets a clear syntax error.
- `typdoc validate` warns (`collections.empty`) when the project has no collections at all.

### Fixed

- `mv a.md a.md` (source and destination are the identical path) now gets its own message
  instead of reusing the case-only-rename wording. Exit code is unchanged (7).

### Internal

- `Cargo.toml` gains the metadata crates.io requires (`license`, `repository`, and each
  published crate's own `description`), and every published crate's version moves to `0.3.0`.
  Two new workflows: `publish-check.yml` runs `cargo publish --workspace --dry-run` on every
  push/PR that touches a manifest, alongside `ci.yml`'s existing jobs; `publish.yml`
  (`workflow_dispatch` only) gates a real `cargo publish --workspace` behind its own
  test/clippy run and a version-check step. Triggering the real publish is a separate,
  manual step after this release merges.

## [0.2.0] - 2026-09-24

### Changed

- Every command now produces text output without `--json`. `new` (both the path-identified and
  coded forms), `get`, `set`, `refs`, `toc`, `validate` (plain and `--schemas`), and both forms of
  `mv` (plain and `--renumber`) no longer refuse to run without `--json`. Text output always
  labels what it prints — no bare, unlabeled value — including `new`'s coded form and
  `mv --renumber`, which previously printed only a bare key on success; a caller that wants just
  the key now reads it out of `--json` instead.
- `list`'s table gains a header row above the columns it already prints (the identity column,
  then `title`, then each `--where` field), shown whenever the result is non-empty. The identity
  column reads `path` for an uncoded collection, `key` when every matched document has one, and
  `document` when the result mixes both (spanning collections with and without a code — neither
  `key` nor `path` alone would be accurate there). A coded document's own identity, in the table
  and in `--ids` alike, is its bare key when the project has exactly one namespace, `namespace:key`
  when it has several — the same rule `refs` follows below.
- `refs` also gains a header row above the columns it already prints: `document` (the document at
  the other end — a coded document as its bare key when the project has exactly one namespace,
  `namespace:key` when it has several, otherwise its bare path, or `(unresolved: <reason>)`) and
  `field` always; `written` (the target as actually written) as a third column only for the
  forward direction, since `--reverse`'s own `written` would only repeat how the holder wrote a
  reference back to the document already named on the command line. Plain/`--schemas` `validate`
  also gains a header row, matching its own `--json` field names: `path`, `level`, `rule`,
  `message`, whose columns are reordered so `rule` comes before `message`. Shown whenever there is
  a ref or a finding to print; still nothing at all, header included, when there is none.
- Both forms of `mv` now report what they rewrote: the destination's `get`-shaped block, followed
  by `rewritten: N refs in M documents`, `unrewritten:` (its own count, with one line per entry
  naming the project, document, field, and written form), and `findings:` (its entries, or
  `none`). `mv --json` gains the full `rewritten` list — one entry per rewritten ref, naming the
  document, field, and its value before and after — additive to every field `mv --json` already
  printed. `unrewritten`'s `reason` (`--json` only) is `imported-project`, `mention` (a
  plain-text mention of the moved key, found and reported at move time rather than left for a
  later `validate` run to discover alone), or `links-rule-off`.
- Every command that fails without `--json` now prints a plain-text error (`typdoc: <message>`)
  on stderr, instead of the `--json` error object leaking through on paths that used to be
  unreachable without `--json`.

### Fixed

- A long, one-way chain of `acyclic` references no longer overflows the stack. `cyclic_nodes`'s
  internal walk is now an explicit iterative traversal over a heap-allocated stack instead of one
  recursive call per document, with no ceiling on how long a chain can be. Output is unchanged for
  a two-node cycle, a self-loop, a chain with no cycle, and a cycle with a tail.
- A ref is resolved by its exact spelling on every platform. On a case-insensitive file system
  (macOS), `target.md` used to resolve to a file named `Target.md`; it is now reported as not
  found, as it always was on Linux.
- `set` and `new` refuse a write that forms a new cycle through an `acyclic` field (exit 2,
  nothing written), which the design required and `validate` alone used to catch. A write that
  forms no new cycle, including one to a document already on a cycle, still succeeds.
- `set` and `new --set` values follow the escape rules: `\,`, `\*` and `\\` are escapes, and a bare
  `*` or any other `\` is exit 1. Values used to be stored exactly as typed, backslashes included.

### Documentation

- The README and the user docs are rewritten for people rather than as a specification. The
  README now covers the motivation, the concepts and how typdoc works; `docs/` is organised as a
  tutorial (`getting-started.md`), how-to guides (`docs/how-to/`), reference (`docs/reference/`,
  replacing `docs/commands.md` and `docs/projects.md`) and explanation (`docs/explanation/`).
- An agent skill ships with the repository in `skills/typdoc/`, installable with
  `npx skills add thaitype/typdoc`. It is written for this version and says so on its first line.
- The README installs the latest version with `cargo install --git`, and says how to pin a
  release with `--tag`; the CI guide pins one.
- Examples and docs keep schemas in `.typdoc/schemas/`. A collection may still point at a schema
  anywhere in the project; this is only where the docs suggest putting one.
- The user docs now state where exact `number` comparison ends: a value past what an `f64` holds
  exactly (past the eighteenth significant digit) can compare equal to a different value in a
  `--where` expression or a `--sort` with no error. Whether `date` and `datetime`, which also
  compare as instants, are affected the same way is not claimed either way.

### Internal

- The toolchain is now pinned (`rust-toolchain.toml`) to the version this project's gates already
  pass on, and CI runs `scripts/test.sh`, `cargo fmt --check`, and
  `cargo clippy --workspace --all-targets -- -D warnings` on every push and pull request into
  `main`, on both Linux and macOS.

## [0.1.0] - 2026-09-23

Initial release. `get`, `list`, `refs`, `toc`, and `validate` read a project; `new`, `set`, and
`mv` (including `mv --renumber`) write one. Schemas with types, enums, refs, and inheritance;
rules over frontmatter, schemas, keys, file names, refs, and body links; a query language for
`list`, including conditions that follow refs. `pull` and remote schemas are not built yet. See
the [README](README.md) for what the tool does and does not do as of this release.
