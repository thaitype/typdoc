# Commands

Eight commands: `get`, `list`, `refs`, `toc` and `validate` read a project; `new`, `set` and `mv` write one. [docs/design/design.md](design/design.md) is the source of truth; this page is the working reference.

Every command takes `--namespace <list>` to choose which namespaces it reaches, and `--json`. Today `list` and `validate --audit` also print plain text, as do `new`'s coded form and `mv --renumber` (the bare key); every other command needs `--json` and exit 1 without it. A write command also takes `--lock-timeout <seconds>` (default 5), how long to wait for the namespace's lock before giving up at exit 4.

A document is named by its key (`WF-2`) or by its path from the project folder (`tickets/WF-2.md`), told apart by form: a key never ends in `.md`. A prefix reaches further: `story-2:WF-5` a sibling namespace, `memory::LRN-1` an imported project. A path that really begins with a name and a colon is written `./name:file.md`.

## `typdoc get <key|path>`

One document's fields, read under its schema.

```console
$ typdoc get WF-2 --json
{"document":{"path":"tickets/WF-2.md","namespace":"default","key":"WF-2","code":"WF",
  "collection":"wayfinder","schema":"wayfinder","fields":{"title":"...","status":"open"}}}
```

A value that does not fit its declared type is returned as it is written, and `validate` is what reports it. `code` is always present and may be null; `key` appears only for a document whose schema has a code.

## `typdoc list`

The documents that match a query.

| Option | Meaning |
| --- | --- |
| `--collection <list>` | Only these collections |
| `--code <list>` | Only the collections whose schema has one of these codes |
| `--where <expr>` | A condition every listed document satisfies; may repeat, and all must hold |
| `--fields <list>` | The columns of the table |
| `--sort <key>` | `field`, `field:asc` or `field:desc`; may repeat, the first breaking ties first |
| `--limit <n>` | Print at most this many; `total` still counts every match |
| `--ids` | One key or path per line instead of the table |

A condition is `field=value`, `field!=value`, or an ordering comparison (`<`, `<=`, `>`, `>=`) on a number, date or datetime. `*` is a glob and a bare `field=*` asks whether the field is present at all. A list of alternatives is written `status=open,claimed`.

Absence has a rule worth knowing: a document without the field fails every positive condition and satisfies every negated one, so `status=open` and `status!=open` divide a set with nothing left over. The ordering comparisons are the exception, and a document without the field fails those too.

A condition can follow a ref:

```console
$ typdoc list --collection tickets --where 'ref.all(blocked_by).status=done'
```

`ref.all`, `ref.any` and `ref.none` follow the refs a document holds; `refby.all`, `refby.any` and `refby.none` follow the refs that point at it; `$body` stands for the document's body links. `all` is true for a document that holds no refs at all. A ref that resolves to nothing counts as absent, and is warned about on standard error.

A field name that no schema in scope declares is an error, not an empty result. A document whose own schema lacks a field that another schema in scope has is simply absent.

## `typdoc refs <key|path>`

What a document points at, or what points at it.

| Option | Meaning |
| --- | --- |
| `--reverse` | The refs that point at this document instead of the ones it holds |
| `--field <f>` | Only the refs in this field; `$body` for body links |

Each reference carries either the document it resolved to, named by its `path` and `namespace` (and its `key` and `project` where it has them), or an `unresolved` reason. Never both, and never neither. The reasons are `not-found`, `bad-prefix` and `import-absent`.

A reverse lookup scans this project's namespaces. It does not enter an imported project, which the design says it should; the difference is deliberate and is held in place by a test.

## `typdoc toc <key|path>`

The headings of a document's body, with the lines they cover.

`--depth <n>` lists only the headings down to that level. `end` is the last line of a heading's section and does not change with `--depth`; the ranges nest rather than tile, so a heading's range contains the ranges of the headings under it. Lines count from the top of the file, frontmatter included, so they match what an editor shows. A slug is what a `#heading` link has to use.

## `typdoc validate [<key|path>...]`

Whether the project keeps the promises its schemas and rules make. With no arguments it checks the whole project; with arguments, only the documents named.

| Option | Meaning |
| --- | --- |
| `--schemas` | Check the schemas only |
| `--strict` | Raise every remaining warning to an error |
| `--audit` | What would have to be fixed to adopt typdoc here |

The report is a summary and a list of findings. The summary says what was covered, so that a list of findings is never read as more than was checked: which namespaces, how many documents, and how many findings at each level, counted after the levels are merged and after `--strict` has raised the warnings.

`--audit` answers a different question from a plain run. It reports rules that are switched off as information rather than silence, lists the files that are in no collection and the files that have no frontmatter, and ends with 0 unless the config itself cannot be read. It is the mode to run when deciding whether to adopt typdoc on a folder that already exists.

Rule levels come from typdoc's defaults, then `validation.global`, then the collection's own `validation`, each merging over the last key by key.

## `typdoc new <CODE|path> [title]`

Create a document. A coded schema's target is its code, and the key is allocated for you; an uncoded schema's target is the path to create.

```console
$ typdoc new WF "Decide the numbering scheme"
WF-2

$ typdoc new notes/second-note.md --set title="A second note" --json
{"document":{"path":"notes/second-note.md","namespace":"default","code":null,"collection":"notes","schema":"note","fields":{"title":"A second note"}}}
```

| Option | Meaning |
| --- | --- |
| `--set <k=v>` | A field to set on the new document; may repeat |

`--json` prints the whole document, defaults and `auto` fields included, because those are exactly what the caller could not work out for itself. A coded form's `title` is required; an uncoded form takes none, and any field goes through `--set`. Allocation happens under the namespace's lock: the next number is the larger of the highest key that exists and the collection's own recorded `last`, so a deleted document's number is never reissued. The file is created with no temp file and no replace — a destination that already exists, coded or not, is refused at exit 7 with nothing written and no number burned.

## `typdoc set <key|path> <field=value>...`

Change fields on one document. `field=value` sets a field; `field=` removes it.

```console
$ typdoc set WF-2 status=claimed --json
{"document":{"path":"tickets/WF-2.md","namespace":"default","key":"WF-2","code":"WF","collection":"wayfinder","schema":"wayfinder","fields":{"title":"Decide the numbering scheme","status":"claimed", ...}}}
```

| Option | Meaning |
| --- | --- |
| `--if <expr>` | A condition, in `list`'s own `--where` syntax, checked under the same lock as the write; may repeat, ANDed |

```console
$ typdoc set WF-2 status=done --if status=done --json
{"error":"`status=done` is false","code":3,"details":[{"path":"tickets/WF-2.md","namespace":"default","collection":"tickets","key":"WF-2","rule":"set.if","level":"error","message":"`status=done` is false"}]}
```

A false `--if` writes nothing and exits 3; the condition and the write happen under one lock, so nothing can change the field between the check and the write. Writing a field the schema marks `auto` directly is refused; `auto: update` is stamped on its own when a value actually changes. A field the schema does not name is written anyway, as plain text, and reported afterward by `frontmatter.unknown` — `set` never refuses an unknown field. Only the fields named change: every other value, and everything about the file that is not frontmatter, is carried across exactly ([docs/design/design.md](design/design.md)'s table of what a write's formatting may lose is the complete list of what is not promised).

## `typdoc mv <from> [to]` / `typdoc mv <from> --renumber <namespace>`

Move a document, or renumber a coded one into another namespace. Rewrites every ref this project holds to it — in frontmatter and in body links, in every namespace — keeping each ref's own written form.

```console
$ typdoc mv notes/first.md notes/renamed.md --json
{"document":{"path":"notes/renamed.md","namespace":"default","code":null,"collection":"notes","schema":"note","fields":{"title":"A note"}},"unrewritten":[],"findings":[]}

$ typdoc mv WF-1 --renumber archive --json
{"document":{"path":"archive/tickets/WF-1.md","namespace":"archive","key":"WF-1","code":"WF","collection":"tickets","schema":"ticket","fields":{"title":"Filed by team A"}},"unrewritten":[],"findings":[]}
```

| Option | Meaning |
| --- | --- |
| `--renumber <namespace>` | Move a coded document to this namespace under a new key, instead of giving a destination path |

A coded document keeps its key within its own namespace and cannot be moved to another path there — `--renumber` is the way to change its namespace, and it takes one positional argument instead of two. `unrewritten` lists, in `refs --reverse`'s own shape, the refs this run could not rewrite (a ref in an imported project, a mention of a coded key, or a body link with `body.links` switched off) with a `reason` for each; `findings`, in `validate`'s own shape, reports what the destination's schema rejects — a move onto a schema the document fails still exits 0, because the move happened and `findings` is what a caller reads instead. A destination that already exists, or that names the same file as the source (by file identity, not by spelling), is refused at exit 7 with nothing written. Renumbering into the document's own namespace, or into another project, is refused at exit 1: the first would retire the key while the document never moved, and the second is never allowed at all. Every temp file `mv` needs is prepared first, then every rename happens in one run with the document moved last, so a run that stops partway can be finished by running the exact same command again.

## Exit codes

| Code | Meaning |
| --- | --- |
| 0 | Success, including a query that matches nothing |
| 1 | Bad arguments: a malformed option or expression, a key or write that is ambiguous across namespaces, or a write the design refuses outright (renumbering into the same namespace, moving a coded document by path) |
| 2 | Validation failed, or the config could not be read |
| 3 | A `set --if` condition was false; nothing written |
| 4 | The namespace's lock was not acquired within `--lock-timeout` |
| 5 | The key, path or file asked for does not exist |
| 6 | A file or directory cannot be read or written |
| 7 | The destination of a write already exists; nothing was written |
