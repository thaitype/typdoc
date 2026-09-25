---
title: Namespaces and project discovery explained
status: active
migrated_from: docs/archived-design/design.md#model
---

A project is a folder containing `.typdoc/`. The folder itself is what marks a project — not
any one file inside it, so `.typdoc/` with no `config.json` is still a project, read as
`{"version": 1}`. `typdoc` finds its project by walking up to the nearest folder containing
`.typdoc/`, starting from the first of these that applies: a document path given as an argument
that is absolute or begins with `./` or `../`, read from disk as the file it names; the folder
`TYPDOC_DIR` names, which skips the walk; the current directory. A folder with its own `.typdoc/`
deeper in the tree is a separate project: a collection's `match` never crosses into it, and a
file there always belongs to the nearer project.

**Namespaces.** Without `namespaces`, the project is one namespace named `default`. With it,
each entry of the list is either a plain name or a glob (`*` only), or the same prefixed with
`!` to exclude what it matches. Entries apply in list order, gitignore-style: the last entry
that matches a folder decides whether it is a namespace, so a later `!` can exclude what an
earlier entry included, and a later plain entry can re-include what an earlier `!` excluded. A
`!` entry that matches no folder — a name or a glob alike — is always silent, unlike a plain
entry: a plain exact name matching nothing is still a config error (`config.namespaces-entry`),
but a `!` entry is never reported for matching nothing, since excluding zero folders is not a
mistake the way naming a missing one is.

**An excluded namespace is fully invisible.** It is not validated, not queried, and a ref into
it resolves as not found — the same answer as a namespace that was never configured at all.
Naming it explicitly, with `--namespace`/`TYPDOC_NAMESPACE` or a document argument's prefix,
fails the same way as naming a namespace that does not exist, and so does a write that targets
it (`new`, `mv --renumber`). An importing project sees nothing of an imported project's excluded
namespace, for the same reason: exclusion is resolved before anything about the namespace is
exposed to anything reading it.

**`!` is a `namespaces` entry only.** `--namespace` and `TYPDOC_NAMESPACE` keep their existing
syntax — names separated by `,`, or globs (`*` only) — and do not accept a leading `!`; giving
one there is bad arguments (exit 1), not a silent misread of the name as a literal folder.

**State survives exclusion.** A namespace's `.typdoc/state/<name>.json` is left untouched while
the namespace is excluded — not read, written, or migrated — so re-including it later continues
issuing numbers from where it left off, with no code reissued. This also means an excluded
namespace's state file is not reported as `config.state-orphan` (see
`docs/design/catalog/config-errors.md`): the file matching an entry that is currently excluding
its folder is a known, deliberate state, not an orphan. A state file whose folder does not exist
at all, and that no entry — plain or `!` — currently matches, is a genuine orphan and is still
reported.

`docs/design/catalog/config-errors.md` and `docs/design/catalog/rules.md` hold the machine-readable
ids this document's rules produce; this document explains the behavior behind them.
