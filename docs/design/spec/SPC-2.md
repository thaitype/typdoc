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

## What `new` and `set` check before they write

Before writing, `new` and `set` check every ref the document's frontmatter will hold (body links are
left to `validate`): its target must exist, and its schema must be one the field's `target` allows.
A ref into an import that is absent on this machine is reported at its `imports.absent` level
instead, and a ref to a document recorded as moved at its `refs.moved` level; either refuses the
write only at `error`. A cycle on an `acyclic` field refuses the write only when the write forms it:
the cycle passes through the document being written, on a field whose value the write changes. A
document that already sits on a cycle can still be written, and `validate` goes on reporting the
cycle.

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

## Arguments that name a document

An argument that names a document is a path or a key, told apart by its form and never guessed.
After any `project::` prefix, an argument that ends in `.md` is a path, and one that has the form
of a key is a key. A key never ends in `.md` and a document is always a `.md` file, so the two
cannot be confused. Anything else is bad arguments (exit 1). A path that begins with `/`, `./` or
`../` is a path on disk, absolute or relative to the current directory. Any other path is
relative to the project folder, the folder that holds `.typdoc`, which is what `path` is in
`--json`. The path of a document of an imported project is written `project::path`, relative to
that project's folder. `mv` reads both its arguments in this way, except that neither may carry a
`project::` prefix: `mv` writes only in the project it is run in, so an argument naming a
document of another project is bad arguments (exit 1). Its second names a file that does not
exist yet: a `mv` whose destination is already there writes nothing and exits 7, and so does a
`--renumber` whose destination name is taken. When a path relative to the project names nothing
in it but a file of that name exists relative to the current directory, the error is exit 5 and
says that `./name` exists. That is a suggestion; nothing is done in its place.

The string that names a document in an argument follows from the name it is printed with
(`SPC-12`):

| The document | As a path | As a key (a coded document only) |
| --- | --- | --- |
| In this project | `path` | `key` when the project has one namespace, `namespace:key` when it has several |
| In an imported project | `project::path` | `project::key` when that project has one namespace, `project::namespace:key` when it has several |

The path form works for every document and needs to know nothing about how many namespaces a
project has, so it is the form for a program to pass on. A key into an imported project with
several namespaces must name one, as a ref must, even when the key is not ambiguous. A name that a
command prints is accepted by every command that takes a key or a path, and a test walks every
document of every project in the fixtures to check it.

## `new`

```bash
typdoc new <CODE> "<title>" [--set k=v ...]      # coded schema: allocates the next key
typdoc new <path> [--set k=v ...]                  # path-identified schema
typdoc new WF "Cosmos or SQL?" --set kind=grilling --set blocked_by=WF-1
```

For a code, `new` allocates the next number under the namespace's lock: the larger of the highest
existing number in the collection within this namespace and the collection's `last` in
`.typdoc/state/<namespace>.json`, plus one, then records it as the new `last`. It writes into
exactly one namespace: if the scope holds more than one, it exits 1 with the choices. A number is
never reused after its document is deleted, as long as the state file records the collection
(`SPC-8`); when it does not and the collection has coded documents in this namespace,
`state.missing` stops the command with exit 2 and nothing is written, and a record that is there
but is not a number that can be held stops it the same way as `state.malformed`. A file created
by hand with a higher number is respected: the highest existing number is then larger than
`last`, and the numbers in between stay unissued, which is harmless. The file is named from the
collection's `match` template.

For a path, the path must match a collection, so `new` cannot create a file outside every
collection. The path already names its namespace folder, so `--namespace` and
`TYPDOC_NAMESPACE` play no part: one given alongside a path is ignored, not checked against it.

`new` fills defaults and `auto` fields (`SPC-15`), validates, and only then writes: a refused
`--set` spends no number. The number is written to the state file before the document is
created, so an interruption between the two leaves a skipped number, never one a later `new`
could issue again. A `new` whose file is already there writes nothing and exits 7, for a path
given on the command line and for a name a template produced alike; the second should not be
reachable, since the number is one nobody has used, and it is checked because an unchecked
impossible state is how a number comes to be issued twice. The check is made under the
namespace's lock, before `last` is raised, and the file is created with `O_EXCL`, so the file
system refuses the write rather than typdoc remembering to look first: a look taken before the
lock is advice that can go stale between the looking and the writing. Array values are
comma-separated.

## `set`

```bash
typdoc set <key|path> k=v [k=v ...] [--if EXPR ...]
typdoc set WF-3 status=claimed owner=session-7a2f --if status=open
```

`set` validates types, enums, transitions and refs, then writes all fields atomically. `--if`
uses `--where` expressions and is checked under the same lock as the write; if any is false,
nothing is written and the exit code is 3. `k=` removes a field. Writing an `auto` field directly
is a validation error; when at least one value changes, `auto: update` fields are set to the
current time in the same write.

A file that exists and that no collection matches can still be written by `set`, the same way a
ref can still reach it. It has no schema, so the write is an ordinary one: `--if` is still
decided under the lock, and nothing else is checked. Every value is kept as plain text, since
there is no field type to say that a comma splits it into a list, and its `--json` leaves
`namespace` out as `get` does.

## `list`

```bash
typdoc list [--collection c[,c]] [--code C[,C]] [--where EXPR ...] [--fields f,...]
            [--sort field[:asc|:desc] ...] [--limit n] [--ids] [--json]
typdoc list --collection wayfinder,decisions --where status=open --sort status --sort updated_at:desc
```

`--collection` selects by collection name; `--code` is a shorthand that selects the collections
whose schema has that code, and the two together select the union. A name or a code that matches
nothing is refused, as an unknown namespace is, since it is more likely a typo than an intended
empty scope. Expressions are described in `SPC-13`. `--ids` prints one key or path per line. An
empty result exits 0.

**Sorting.** `--sort field:dir` may repeat; the first flag sorts first and later flags break
ties. The direction is `asc` (default when omitted) or `desc`; anything else is an error. Ties
that every `--sort` leaves, and the whole result without `--sort`, are in key or path order.

| Sorted value | Order |
| --- | --- |
| `number`, `date`, `datetime` | By value; `datetime` as instants, offsets included |
| `enum` | By position in the schema's `values`, e.g. `open → claimed → resolved → closed` |
| `key` | By code, then numerically: `WF-2` before `WF-10` |
| `string`, `path` | Lexicographic |
| Missing value | Last, in both directions |

Only a document's own fields and pseudo-fields can be sort keys; fields reached through refs
cannot. A value that does not fit its declared type sorts as a missing one. A `--sort` field that
no schema in scope declares is not an error, unlike a `--where` field: it changes only the
order, never which documents are returned, so every document reads it as missing. Two fields of
one name whose schemas give them different types compare as equal, since no rule orders one type
against another, and a key compared with a path compares as text.

## `validate`

```bash
typdoc validate [<key|path> ...] [--schemas] [--strict] [--audit]
```

`--schemas` checks schemas only, including that each qualified `target` names a schema that
exists (schema drift). Suited to pre-commit, CI and agent post-edit hooks. The arguments are keys
or paths of documents, in any mix. A key that exists in more than one namespace in scope stops the
command with exit 1 and every choice listed, and an argument that names no document stops it with
exit 5; in both cases before any report is made, never as a finding. An argument that names a
file matched by more than one collection is neither: it is reported under `collections.overlap`
and the other arguments are still checked. `--schemas` and `--audit` describe the whole project,
each in its own way, so combining either with arguments, or the two with each other, is bad
arguments (exit 1).

## Cost of a run

Every run builds its index from the files with no cache, so the time of a run grows with the
number of documents, and no figure is promised for it. `list` reports `total`, so it filters every
document even under `--limit`: a choice made so that a result says how much it left out, and its
cost is part of the cost above. The passes that need the whole project, such as the one
`refs.moved` and `refs.acyclic` need before any one document's refs can be judged, read every
document again.
