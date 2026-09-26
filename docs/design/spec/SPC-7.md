---
title: Namespaces and project discovery explained
status: active
migrated_from: docs/archived-design/design.md#model
---

A project is a folder containing `.typdoc/config.json`. It is what `imports` points at;
collections and schemas belong to the project, and a file belongs to the nearest project above
it. A folder with its own `.typdoc/` deeper in the tree is a separate project: a collection's
`match` never crosses into it, and a file there always belongs to the nearer project. Namespaces
of one project never nest.

## The `version` number

Decided: one number covers the format of every file typdoc owns — `config.json`, collection
files, the schema format, `lock.json`, and state files. A local schema is read in its project's
own version. A remote schema carries no format marker of its own yet; that is decided together
with remote schemas, not here. `config.json` is required, not optional, because this number must
always be present somewhere a project can be asked for it — making the file optional would mean
promising forever that "no file" means version 1 of everything it could ever cover, with no
version actually visible anywhere in the project. (This was reverted after landing once, before
any release shipped it — free to undo at that point; doing so after a version ships it would be a
breaking change.)

Decided: bump the number when an older typdoc would read a new file wrong without saying so, or
would write it back and lose data — an existing key changes meaning or type, a key is renamed or
removed, or a file's structure changes.

Decided: no bump when an older typdoc doesn't know about something new but doesn't break on it
either. A new field in a state entry survives an older typdoc's own write untouched — verified: a
write patches only the `last` value in place, leaving every other byte of the entry as it was. A
new key in `config.json` is refused loudly instead, as `config.unknown-key`, rather than being
silently ignored or silently accepted.

Decided: an unknown number is `config.version` and stops the command; typdoc never guesses at a
version it doesn't recognize.

**Open, not yet decided (leaning this way):** whether the number only ever goes up by one, and
whether a newer typdoc reading an older project's files either reads them as they are or says
exactly how to convert them.

**Discovery.** `typdoc` finds its project by walking up to the nearest folder containing
`.typdoc/config.json`, starting from the first of these that applies: a document path given as an
argument that is absolute or begins with `./` or `../`, read from disk as the file it names; the
folder `TYPDOC_DIR` names, which skips the walk, for an agent that runs from a repository or
worktree root above the project; the current directory. Any other path argument is relative to
the project folder — it cannot say which project it is in, so it takes no part in this choice and
is read after the project is found. There is no `--dir` flag. Config, collection files and local
schemas are read on every run with no cache; remote schemas are read from their pinned copies.

**Namespaces.** Without `namespaces`, the project is one namespace named `default`, and `match`
counts from the folder that holds `.typdoc`. With it, each entry of the list is one of three
kinds: a plain name, a glob (`*` only), or either prefixed with `!` to exclude what it matches.
Entries apply in list order, gitignore-style: the last entry that matches a folder decides
whether it is a namespace, so a later `!` can exclude what an earlier entry included, and a
later plain entry can re-include what an earlier `!` excluded. Every folder still standing after
all entries are applied is one namespace, named by its folder, with `match` counting from that
folder.

- An entry is one path segment. `/` and `**` are config errors, because a namespace name has to
  show where it ends in a path reference such as `chief::story-3/_tickets/WF-5.md`. Anything
  deeper is grouped by placing `.typdoc` deeper.
- A name uses only ASCII letters, digits, `-` and `_`. `*` never matches a folder starting with
  `.`. A matched folder with any other name is a config error that names the folder; it is never
  skipped.
- `default` is reserved: a folder may not use it.
- A plain entry without a glob must exist. A `!` entry matching no folder — exact name or glob
  alike — is always silent: excluding zero folders is not a mistake the way naming a missing one
  is, so it is never a config error, unlike a plain entry.
- A matched folder that holds its own `.typdoc` is a config error: namespaces do not nest.
- A glob that reaches a folder that is a symbolic link skips it, and the run reports it under
  `files.unreadable`, the same as a `match` that reaches a link. An entry that names a symbolic
  link in plain text is a config error (`config.namespaces-entry`) that says to name the folder
  it points to. A link is a second name for a folder that is already there, so following it would
  count one set of documents twice under two paths, which breaks the accounting and the rule that
  a path names one document; a folder whose name begins with `.` is a folder of its own that no
  other name reaches, which is why it is entered when it is named and a link is not.
- Collections, schemas and rule levels are shared by every namespace of a project. A code may be
  used in every namespace; each namespace numbers its own keys, and a key is unique within its
  namespace, not across them.
- A ref or mention with no prefix means the namespace of the document that holds it, whatever the
  working directory. Prefixes are described under Refs.
- Files outside every namespace folder belong to no namespace; a relative path can still point at
  them. A ref or a body link that reaches such a file resolves like any other: the file exists,
  so the answer is never `not-found`, and the command goes on. The file is named by its path
  alone: `refs` and a `ref.*` or `refby.*` condition treat it as a document with no namespace,
  and `--json` leaves the `namespace` field out, the way it leaves out `key` for a document that
  has none.
- Any other key in `config.json`, `name` included, is an unknown key and a config error.

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

## Choosing a namespace

In a project with more than one namespace, a command takes its scope from the first of these that
applies:

1. A prefix on a key or path argument (`story-2:WF-5`).
2. `--namespace <list>`: names separated by `,`, or globs (`*` only). `'*'` means every namespace
   of this project; imported projects are not included and are named explicitly, as in
   `'chief::*'`.
3. `TYPDOC_NAMESPACE`, with the same syntax as `--namespace`.
4. The current directory, when it is inside a namespace folder or below one.
5. Otherwise there is no scope: reads span every namespace of the project and writes are an
   error.

A key that exists in more than one namespace in scope, and a write that could land in more than
one, exit 1 with every choice listed and, with `--json`, a `candidates` array; typdoc never picks.
From inside a namespace, reading another needs a prefix or `--namespace`. A ref written in a file
always means the namespace of that file, whatever the working directory. A namespace of an
imported project can be named only for reading. Examples always quote `'*'`, because an unquoted
`*` is expanded by the shell.

`docs/design/catalog/config-errors.md` and `docs/design/catalog/rules.md` hold the machine-readable
ids this document's rules produce; this document explains the behavior behind them.
