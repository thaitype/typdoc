# Commands

Five commands, all of which read and none of which write. `design.md` is the source of truth; this page is the working reference.

Every command takes `--namespace <list>` to choose which namespaces it reads, and `--json`. Today `list` and `validate --audit` also print plain text; `get`, `toc`, `refs` and plain `validate` need `--json` and exit 1 without it.

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

A reverse lookup scans this project's namespaces. It does not enter an imported project, which the design says it should; that difference is listed in `crates/typdoc/src/registry.rs`.

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

## Exit codes

| Code | Meaning |
| --- | --- |
| 0 | Success, including a query that matches nothing |
| 1 | Bad arguments: a malformed option or expression, or a key that is ambiguous across namespaces |
| 2 | Validation failed, or the config could not be read |
| 5 | The key, path or file asked for does not exist |
| 6 | A file or directory cannot be read |

Codes 3 and 4 belong to commands that write, which this version does not have.
