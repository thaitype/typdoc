# Changelog

All notable, user-visible changes to typdoc are documented here. Internal reorganizations (for
example, how the project's own design documents are structured and read in its own test suite)
are left out unless they change something a user of the `typdoc` binary sees.

## [Unreleased]

### Added

- Every document typdoc prints carries `ref`, its portable name: the name that finds it from
  anywhere in the project, in a ref, in a body link when it is a path, and on the command line.
  It is `WF-5` (or `story-2:WF-5` in a project with several namespaces) for a numbered document,
  `default:notes/x.md` (or `story-2:notes/x.md`) for any other, and a document of an imported
  project has its `alias::` in front. In `--json` every document object (`get`, `set`, `new`,
  `mv`, `list`, `toc`, `refs` and its entries) gains `ref` beside `path`, which is unchanged. Text
  output gains a `ref:` line (`get`, `set`, `new`, `mv`) and a `ref` column (`list`, `refs`). A file
  outside every namespace folder has none.
- `refs --json` lists an absolute path in a ref or a body link as `unresolved: "absolute"`: typdoc
  never looks one up, so it is not reported as `not-found`.
- `typdoc-core` (library): `Names`, from `Project::names`, writes a document's portable name
  (`Names::portable`) and the name a command prints for it (`Names::printed`).

### Changed

- On the command line, a path after a namespace prefix is read from that namespace's folder, as in
  a ref: `typdoc get story-2:notes/x.md` reads `story-2/notes/x.md`. It used to be read from the
  project folder, with the prefix only choosing the namespace, so `story-2:story-2/notes/x.md` is
  now `story-2/story-2/notes/x.md`. This holds for every command, `new`'s path and `mv`'s
  destination included.
- A namespace prefix is one namespace's exact name, on the command line as in a ref. A glob or a
  list in a prefix (`story-*:WF-1`, `story-1,story-2:WF-1`) is exit 1; select several namespaces
  with `--namespace` or `TYPDOC_NAMESPACE`: `typdoc get WF-1 --namespace 'story-*'`. `default:`
  names the only namespace of a project, or of an imported project, that has one, whatever it is
  called.
- `./` and `../` in a frontmatter ref or a body link are read from the document's folder under
  both `refBase` values; `refBase` moves only a path with no prefix. A project whose collection sets
  `"refBase": "namespace"` and writes `./x.md` in frontmatter meaning the namespace folder's `x.md`
  now finds nothing there: `refs.resolve` and `body.links` say where the name used to lead and what
  to write instead, `story-1:x.md` in a ref (`default:x.md` in a project with one namespace) and a
  path from the document in a body link.
- A ref with the shape of a key (`XX-1`) is a key whatever its code, as on the command line: it
  used to be read as a relative path when no schema had the code. A key no document has is
  reported as `no document with key XX-1`, in `refs.resolve` and as a command's error (exit 5,
  which used to say `no document at XX-1`).
- An absolute path in a ref or a body link, a Windows drive letter included (`C:\notes\a.md`), is
  reported by `refs.resolve` or `body.links` as an absolute path and never followed.
- A body link `name:rest.md` whose `name` is no namespace is reported by `body.links`, as its prefix
  names no namespace; it used to be taken for a URL and skipped. A link that does not end in `.md`
  (`tel:123`) is still a URL.
- A document of an imported project is printed with its `alias::` in `list` (`typdoc list
  --namespace 'memory::*'` prints `memory::LRN-1`, not `LRN-1`), and a key in it is qualified by
  that project's namespaces in `refs` (`several::story-2:WF-9`, not `several::WF-9`), so every name
  typdoc prints reads back.
- A collection's `match` and `body.links`' `ignore` refuse `?`, `[`, `]`, `\`, `,` and a leading `!`
  as a config error that names the pattern; such a pattern used to match nothing, silently. Both
  take `*` and `**` only.
- `typdoc-core` (library): `Error` has a new variant, `NoKey`, for a key no document has, which
  used to be `NotFound`; a `match` on `Error` must add it. A path in a `DocumentArg` with a
  namespace prefix is read from that namespace's folder.

### Fixed

- `mv` rewrote a body link to the moved document without its `./` or its `#anchor`
  (`[one](./b.md#part)` became `[one](c.md)`), and left a second link to it on the same line
  unchanged when the first rewrite shortened the line. Every link on a line is rewritten, keeping
  its `./`, its `#anchor` and its `<...>` or `%20`.
- A move recorded in `auto: moves` is matched by the document a ref names, not by its text: a
  recorded key only in the namespace that issued it, so `WF-4` in one namespace is no longer
  reported as moved because another namespace's `WF-4` moved, and a recorded path from whatever
  folder the ref is written in. The message names the document with its namespace
  (`story-2:WF-5`). `mv` reports only the plain-text mentions of the moved document, not every
  mention of the same key, and a mention with an import prefix (`chief::story-3:WF-5`) is looked
  up in that import instead of always reported as not found.

## [0.6.0] - 2026-09-28

### Added

- Windows support, on amd64 and arm64. Every command reads and writes there as it does on macOS and
  Linux, with the same guarantees: a lock is released only while it is still this process's own,
  on NTFS and ReFS alike; a document typdoc replaces keeps its access control list and its
  read-only flag, and one whose access control list cannot be carried is left as it was (exit 6);
  a document another program holds open without letting it be replaced is left as it was, and the
  message says so (exit 6). Ctrl+C removes the locks held and ends the process with
  `STATUS_CONTROL_C_EXIT` (`0xC000013A`).
- A path on the command line may be written with Windows separators: `C:\notes\a.md`, `\`, `.\`
  and `..\` name a file on disk, and a path relative to the project may use `\` (`typdoc get
  notes\a.md`, also after a `namespace:` or `project::` prefix). On Unix `\` stays part of a
  name. A drive-relative `C:note.md` is still read as the namespace `C`.
- A PowerShell installer: `irm https://typdoc.thaitype.dev/install.ps1 | iex`, with
  `TYPDOC_VERSION` and `INSTALL_DIR` as in the shell script. It verifies the checksum before
  installing, never edits `PATH`, and installs `typdoc.exe` to `%USERPROFILE%\.local\bin`.
- Releases carry `typdoc-x86_64-pc-windows-msvc.zip` and `typdoc-aarch64-pc-windows-msvc.zip`,
  each with its `.sha256` and a build attestation.
- `typdoc-core` (library): `project_path_from_argument`, which reads a project path typed on the
  command line the way `Argument::parse` does.

### Changed

- `typdoc-core` (library): `FileId::inode` is a `u128`, to hold a Windows file id, which is 128
  bits on ReFS. Code that builds or reads a `FileId` must change.

## [0.5.0] - 2026-09-27

### Fixed

- On macOS, a lock held by a running typdoc was reported as stale when a write timed out (exit 4),
  with the file to delete: the hostname and whether a process runs were read from `/proc`, which
  macOS does not have. Both are now asked of the system on every Unix. A lock held by a process
  of another user reads as running.
- The exit-4 message says the lock is stale only when the system answered that no process has
  the recorded id. Where that cannot be known it says so, with one of two new endings: `its host
  is not known`, when this machine's hostname cannot be read or the lock records that its owner's
  could not, and `whether it is running cannot be checked here`, when the process cannot be asked
  about.

### Changed

- `typdoc-core` (library): `acquire` takes `env: &dyn Env` in place of `host: &str`, and `Env`
  has a new required method, `process_status`, returning the new `ProcessStatus`. A caller of
  `acquire` or an implementation of `Env` must change.

## [0.4.0] - 2026-09-27

### Added

- A coded document's file name can carry a slug after its key: `tickets/WF-8-lock-order.md` is
  the document `WF-8`, with the slug `lock-order`. The key identifies the document everywhere (in
  refs, arguments, output and numbering); the slug is only there for a person reading a folder
  listing or a link, and `--json` shows it only in `path`, never in `key`. The number is every
  digit after the code, so `WF-12-x.md` is `WF-12` with the slug `x`. A slug holds any characters
  a file name can, in any language, except whitespace, `/`, `#` and `:`, and it is not empty. In
  `{key}/README.md` the slug goes on the folder (`WF-1-x/README.md`). File names of documents
  without a code are unchanged.
- A collection file takes a new key, `slug`, for coded schemas: `optional` (the default: a file
  name with or without a slug), `required` (a slug after every key) or `none` (no slug). Any other
  value, a value that is not a string, or `slug` on a collection whose schema has no code is the
  new config error `config.collection-slug` (exit 2).
- `typdoc new WF "Decide lock order" --slug lock-order` creates `tickets/WF-8-lock-order.md`.
  typdoc never makes a slug up or rewrites the one given. A slug that breaks the character rule,
  `--slug` under `slug: none`, no `--slug` under `slug: required`, and `--slug` with a path target
  are each exit 1, checked before the namespace lock, so no number is used.
- A key can be written with its slug wherever a key works, as an argument and as a frontmatter
  ref: `WF-8-lock-order`, `story-2:WF-8-lock-order` and `chief::story-3:WF-8-lock-order` all name
  `WF-8` and resolve by the key alone, and `refs.codedByPath` does not fire for them. Text after
  the digits that is not a valid slug makes no key, and text ending in `.md` is never a key, so
  `WF-8-a#b` is exit 1 as an argument and `WF-1-x.md` in a ref stays a path. A key written with
  its slug in plain body text is not a mention and stays unchecked.
- A new configurable rule, `refs.slug` (default `warn`), reports a frontmatter ref written with a
  slug that is not the one its target's file carries now, or whose target now has none; the
  message names the file's current name. A ref by the key alone is never reported. `new` and `set`
  check it before they write, as they check `refs.moved`, and refuse the write only when it is set
  to `error`.
- `mv` can change a coded document's slug: a move to the name its collection gives the same key
  with another slug, or with none (`typdoc mv story-2:WF-5 story-2/_tickets/WF-5-json-shapes.md`).
  A ref by the key alone is left as it is, a ref written with the old slug gets the new one (or
  the key alone when the slug is removed), body links name the new file, and a field with
  `auto: moves` records the previous path. A destination with another key or in another folder is
  still exit 1, and so is a destination whose slug breaks the character rule, with nothing
  written. A destination in the form the collection's `slug` does not expect is moved, with
  `filename.pattern` in `findings`, exit 0.
- `mv --renumber` keeps the file's slug under the new key:
  `story-2/_tickets/WF-5-json-output-shape.md` renumbered into `story-3` becomes
  `story-3/_tickets/WF-8-json-output-shape.md`, and a ref written with the slug gets the new key
  and the same slug. A slug already on disk is never dropped: a name in a form the collection does
  not expect, or with a slug that breaks the character rule, is renumbered as it is, with
  `filename.pattern` in `findings`, exit 0.

### Changed

- `filename.pattern` also reports a coded document whose name is not in the form its collection's
  `slug` expects (`WF-1.md` under `required`, `WF-1-x.md` under `none`), or whose text after the
  key is empty or holds whitespace, `/`, `#` or `:` (`WF-1-a b.md`). Such a file stays the
  document its key names: it is listed, checked against the schema, and refs to it resolve. The
  finding carries its `collection` and `key`; a file that fits no template is reported as before,
  with neither.
- `config.match-template` also covers a coded `match` whose text right after `{key}` starts with
  a digit or `-` (`{key}1.md`, `{key}-notes.md`) while the collection's `slug` is not `none`: such
  a template cannot tell a slug from its own text. The message says to set `slug` to `none`, which
  reads names exactly as before.

### Fixed

- A body link written from another folder to a document's old path, after a move recorded in an
  `auto: moves` field, is now reported as `refs.moved`, naming the new path, instead of
  `body.links`.

### Upgrade note

A project reads more files after this release, never fewer: a `WF-1-x.md` that
`filename.pattern` reported before is now the document `WF-1`. With `filename.pattern` at its
default level (`error`) such a project was already failing on that file. A project that loaded
and passed `validate` before can fail after the upgrade in three cases:

1. `filename.pattern` is set to `off` or `warn`, and a file like `WF-1-x.md` does not satisfy the
   schema, or shares its key with a `WF-1.md` beside it (`keys.unique`).
2. A collection with a glob in the same folder also matches a file like `WF-1-x.md` (for example
   `tickets/*-notes.md` and `WF-1-notes.md`): that file now belongs to both collections, which is
   `collections.overlap`.
3. A coded `match` has a digit or `-` right after `{key}` (`{key}-notes.md`): the project stops
   loading with `config.match-template` (exit 2) until the collection sets `"slug": "none"`.

## [0.3.1] - 2026-09-25

### Added

- Prebuilt binaries for four platforms, attached to this release alongside the existing
  `cargo install typdoc` path: macOS (Apple Silicon and Intel) and Linux x86_64 and aarch64
  (musl, carrying no glibc-version requirement). Install with no Rust toolchain via
  `curl -fsSL https://typdoc.thaitype.dev/install | sh`. Windows is not supported yet, by this
  installer or by `cargo install`.
- Every released binary carries a SHA-256 checksum and a GitHub artifact attestation, checkable
  independently of the installer with `gh attestation verify`.
- `TYPDOC_VERSION` pins an installer run to a specific release instead of latest; `INSTALL_DIR`
  overrides the default install location (`~/.local/bin`). The installer never edits `PATH` or a
  shell profile; it prints the one line to run if the install directory isn't already on `PATH`.
- The `x86_64-apple-darwin` binary is cross-compiled for this release but not executed in CI —
  no matching-arch GitHub-hosted runner is available to this repo — and is named as
  built-but-not-run rather than presented as verified. Every other target is both built and run
  (`typdoc --version`) on a runner of its own architecture before being attached to the release.

### Internal

- `dist-workspace.toml` (`cargo-dist` pinned to an exact version) configures the four build
  targets. A reusable GitHub Actions workflow builds them and produces each target's archive and
  `.sha256` checksum; a PR-triggered caller runs it on every relevant pull request, and
  `publish.yml` calls the same workflow for the real release, then attests build provenance
  (`actions/attest-build-provenance`) over the built archives — a PR build never generates one.
- Every published crate's version moves to `0.3.1`; no behavior change in `typdoc` itself.

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
