# Changelog

All notable, user-visible changes to typdoc are documented here. Internal reorganizations (for
example, how the project's own design documents are structured and read in its own test suite)
are left out unless they change something a user of the `typdoc` binary sees.

## [0.2.0] - Unreleased

### Changed

- Every command now produces text output without `--json`. `new` (both the path-identified and
  coded forms), `get`, `set`, `refs`, `toc`, `validate` (plain and `--schemas`), and both forms of
  `mv` (plain and `--renumber`) no longer refuse to run without `--json`. Text output always
  labels what it prints — no bare, unlabeled value — including `new`'s coded form and
  `mv --renumber`, which previously printed only a bare key on success; a caller that wants just
  the key now reads it out of `--json` instead.
- `list`'s table gains a header row above the columns it already prints (`path`, or `key` for a
  coded collection, then `title`, then each `--where` field), shown whenever the result is
  non-empty. `--ids` output is unchanged.
- Both forms of `mv` now report what they rewrote: the destination's `get`-shaped block, followed
  by `rewritten: N refs in M documents`, `unrewritten:` (its own count, with one line per entry
  naming the project, document, field, and written form), and `findings:` (its entries, or
  `none`). `mv --json` gains the full `rewritten` list — one entry per rewritten ref, naming the
  document, field, and its value before and after — additive to every field `mv --json` already
  printed.
- Every command that fails without `--json` now prints a plain-text error (`typdoc: <message>`)
  on stderr, instead of the `--json` error object leaking through on paths that used to be
  unreachable without `--json`.

### Fixed

- A long, one-way chain of `acyclic` references no longer overflows the stack. `cyclic_nodes`'s
  internal walk is now an explicit iterative traversal over a heap-allocated stack instead of one
  recursive call per document, with no ceiling on how long a chain can be. Output is unchanged for
  a two-node cycle, a self-loop, a chain with no cycle, and a cycle with a tail.

### Documentation

- The user docs now state where exact `number` comparison ends: a value past what an `f64` holds
  exactly (past the eighteenth significant digit) can compare equal to a different value in a
  `--where` expression or a `--sort` with no error. Whether `date` and `datetime`, which also
  compare as instants, are affected the same way is not claimed either way.

### Internal

- The toolchain is now pinned (`rust-toolchain.toml`) to the version this project's gates already
  pass on, and CI runs `scripts/test.sh`, `cargo fmt --check`, and
  `cargo clippy --workspace --all-targets -- -D warnings` on every push and pull request into
  `main`.

## [0.1.0] - 2026-09-23

Initial release. `get`, `list`, `refs`, `toc`, and `validate` read a project; `new`, `set`, and
`mv` (including `mv --renumber`) write one. Schemas with types, enums, refs, and inheritance;
rules over frontmatter, schemas, keys, file names, refs, and body links; a query language for
`list`, including conditions that follow refs. `pull` and remote schemas are not built yet. See
the [README](README.md) for what the tool does and does not do as of this release.
