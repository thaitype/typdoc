---
title: Commands explained
status: active
migrated_from: docs/archived-design/design.md#commands
---

`typdoc` has nine commands: `new`, `get`, `list`, `set`, `toc`, `refs`, `mv`, `pull`, and
`validate`.

- `new` creates a document — either the path to create, or the code of a coded schema, which
  allocates the next key.
- `get` reads one document's frontmatter.
- `list` queries documents by schema, `--where` conditions and sort order.
- `set` updates one or more fields, optionally guarded by an `--if` compare-and-set condition.
- `toc` lists the headings of a document's body with their line ranges.
- `refs` shows a document's outgoing or, with `--reverse`, incoming refs.
- `mv` moves a document within the project and rewrites every ref this project holds to it.
- `pull` re-fetches a pinned remote schema and compares it with the recorded hash.
- `validate` checks the project, or named documents, against its schemas and rules.

`docs/design/catalog/commands.md` holds this same list as plain strings — the set a test
compares the CLI's own command table against, so a command added to one and not the other is
caught rather than drifting apart silently.

## `mv` explained

```bash
typdoc mv <from> <to>                     # move within this project
typdoc mv <from> --renumber <namespace>   # move to another namespace, under a new key
# stdout for --renumber: WF-4
```

`mv` moves a file and rewrites every ref to it that this project holds, in frontmatter and body
links, across every namespace of the project, keeping each ref's written form (key, prefixed
reference or relative path). It writes no file outside this project. A body link keeps its own
form too: a destination written `<…>` stays that way, one written with `%20` stays
percent-encoded, and if the new path contains a space, a `<` or unbalanced parentheses and the
link used neither, it is written `<…>`. It takes the lock of every namespace it writes, in a
fixed order (see Concurrency). Within its namespace a coded document keeps its key. A coded
document cannot move to another namespace, because its key belongs to the namespace that issued
it: the command fails, says so, and
suggests `--renumber`. A document without a code can move between the namespaces of one project,
with its refs rewritten. If the schema has a field with `auto: moves`, the previous key or path
is appended to it on every move.

**`mv`'s own-path check.** A destination that names the same file as the source is refused at
exit 7 with nothing written, but the message told apart by which of two distinct cases it is.
The same path given twice gets its own message, naming the document and saying nothing was
moved. A name that differs from the source only in case, on a file system that does not tell
the two names apart, keeps the other message: that the file system does not distinguish the two
names, so no change was made. `mv` asks the file system whether the two paths are one file
rather than comparing the text of them, since that is the question whose answer decides whether
anything can be written; which message is shown then follows from whether the two paths were
the same text to begin with.

**How `mv` writes, and what a failure leaves.** `mv` writes many files, and a file system gives
atomicity one rename at a time, so `mv` does not promise to be all or nothing. It prepares a
temp file for every file it will change first, and then does the renames in one run at the end.
The window that remains is the run of renames itself, and it is not closed: a failure or an
interrupt inside it leaves some files renamed and the rest not.

The document itself is always moved last, after every ref has been rewritten. The reason is
recovery rather than a smaller mess. If the document moved first and the command then stopped,
the same command could not be run again, because the path it names as its source is gone. Moving
it last means the document is still where it was, so running the same command again finishes the
work: the refs already rewritten name the new path and are left alone, and the rest are
rewritten. The command is its own way back, and no journal is needed.

Between the failure and the re-run the project is inconsistent, and `validate` says so: the refs
already rewritten name a path that is not there yet. A `mv` that stops says plainly that the same
command can be run again to finish, rather than only that it failed, so that recovering does not
depend on the user working it out.

**`--renumber`.** Moves a coded document to another namespace under a new key. The destination
is the value of the flag, not a second positional argument: `typdoc mv WF-2 --renumber story-3`.
An argument that names a document is read by the rule that something ending in `.md` is a path,
something of the form of a key is a key, and anything else is bad arguments; a bare namespace
name is none of those. So `mv` reads two positional arguments in its ordinary form and one with
`--renumber`, and `--renumber` is a flag that must be given a value: `--renumber` with nothing
after it is bad arguments, and the message says that a namespace is required.

A `--renumber` whose destination is the namespace the document is already in is refused, and
nothing is written. Carrying it out would be worse than doing nothing: `WF-2` would become
`WF-3` while the document stayed exactly where it was, the key `WF-2` would be dead for good
because `last` had risen past it, everything outside this project that cites `WF-2` would be
pointing at nothing, and `validate` would report none of it and exit 0. A coded document cannot
be renumbered into another project either — no command writes into another project.

`--renumber` prints the new key, and nothing else, on standard output, as `typdoc new` prints the
key it allocated, so that a shell can put it in a variable. It holds the locks of both
namespaces (in the same fixed order), issues the next number from the destination's `last`,
moves the file and rewrites every visible ref in frontmatter and body links, bare and prefixed,
in the form that is correct
from each referencing document's own namespace, and reports what it cannot rewrite. The old key
is never issued again, because the source namespace's `last` never goes down. It writes under the
same promise as `mv` above, with one ordering rule of its own: the destination namespace's `last`
is written before the document appears under its new key. A number that is recorded and then not
used is skipped, and a skipped number is ordinary: a collection that runs `WF-3` and then `WF-5`
is not missing a document. The alternative — letting a document exist under a number the state
file has not recorded — is what issues that number a second time.

**The collection a document lands in.** `mv` changes a path, and a path decides which collection
a document belongs to, so a move can change a document's schema or take it out of every
collection.

- **Refused.** A coded document cannot move out of its own collection's folder, and a document
  without a code cannot move into a coded collection. A coded collection's `match` takes `{key}`
  exactly once and allows no globs, and moving a coded document out would leave the key every ref
  uses pointing at nothing. `mv --renumber` is the way a coded document moves.
- **Moved, and reported.** A document that moves into another collection and then does not
  satisfy that collection's schema is moved anyway, and what the schema rejects is reported.
  Refusing would leave no order of steps that works, since `set` validates before it writes too.
- **Allowed, and said out loud.** A document may move out of every collection. The command says
  so in its own words: the document will not appear in `list`, and its refs are no longer
  checked.

**A move that lands on a schema the document does not satisfy exits 0, not 2.** Exit 2 means
validation failed, and everywhere else it comes with nothing having been written. Deciding
whether a document satisfies its schema is `validate`'s work, not `mv`'s: the result of the check
travels in `mv`'s `--json` output, and CI catches it by running `validate`.

**What `mv` cannot rewrite, and how each case still surfaces:**

- **Every other project.** A project that imports this one holds refs that `mv` does not touch,
  because an imported project is read-only and `mv` never crosses a project boundary. `mv`
  reports them, naming the project each unrewritten ref is in and the refs themselves. A project
  this one cannot see at all cannot be reported.
- **Mentions.** Plain-text mentions are never rewritten. `body.mentions` checks them when it is
  on, and `refs.moved` names the new key.
- **Body links, when `body.links` is `off`.** A body link that `mv` cannot follow to the old path
  is caught by `body.links` at validation; with that rule off, it is not caught.
- **Refs from projects that do not import this one.** They are invisible from here, caught when
  that project runs `validate` itself.

None of these four makes the run a failure: a `mv` that meets them finishes, exits 0 and reports
each one in detail. What `mv` promises is the refs typdoc tracks — plain text was never one of
them, and an import absent on this machine is ordinary by design. A project that needs its
imports present sets `imports.absent` to `error`.

For a coded document, none of these can point at a different document silently: a key is never
issued again, and a path made from a key is never reused either. A document without a code is
identified by its path, and a path can be created again later; a ref to it then resolves to the
new document, which is the ref meaning what it says.
