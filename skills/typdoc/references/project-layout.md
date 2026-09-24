# Reading a typdoc project

typdoc 0.2.0. How to read what a project declares, so you know what its documents may hold. This
page is for understanding an existing project, not for designing one.

## The layout

```
.typdoc/config.json               the project: version, namespaces, imports, rule levels
.typdoc/collections/<name>.json   one file per collection: which files, which schema
.typdoc/state/<namespace>.json    the last key number issued per coded collection (committed)
.typdoc/locks/                    write locks (not committed)
.typdoc/schemas/*.json            schemas, usually here; a collection may name any path
```

Everything else is documents, arranged however the project already arranges them.

A project is found by walking up from the current directory to the nearest `.typdoc/` folder, or
from `TYPDOC_DIR` when it is set. A folder deeper in the tree with its own `.typdoc/` is a
separate project.

## Config

`config.json` is optional: a `.typdoc/` folder with none is read as `{"version": 1}`. A project
with no collections at all still loads and every command still runs; `validate` warns
(`collections.empty`) that there's nothing configured to check.

```json
{
  "version": 1,
  "namespaces": ["story-*", "archive"],
  "imports": { "memory": "../memory" },
  "validation": { "global": { "body.links": { "level": "error", "ignore": ["assets/**"] } } }
}
```

- `version` is the only required key, and only when `config.json` exists at all.
- `namespaces`: folders (names or `*` globs) that each hold their own documents. Absent → one
  namespace, `default`, which is the whole folder.
- `imports`: alias → another project on this machine, so refs can cross into it. One level only.
- `validation`: rule levels; see [validation.md](validation.md).

## Collections and schemas

A collection says which files a schema applies to:

```json
{ "match": "tickets/{key}.md", "schema": ".typdoc/schemas/ticket.json" }
```

- `match` is relative to the namespace folder. `*` matches within one name, `**` whole folders,
  `{key}` the document's key. Globs do not enter folders whose name begins with `.`.
- `schema` is a path from the project folder.
- A collection file may carry its own `validation`, merged over the project's.

A schema says which fields a document has:

```json
{
  "name": "ticket",
  "code": "WF",
  "fields": {
    "title":      { "type": "string", "required": true },
    "status":     { "type": "enum", "values": ["open", "claimed", "done"], "default": "open" },
    "blocked_by": { "type": "ref[]", "target": ["ticket"], "acyclic": true, "default": [] },
    "estimate":   { "type": "number" }
  }
}
```

| Field option | Applies to | Meaning |
| --- | --- | --- |
| `type` | all | `string`, `number`, `bool`, `date`, `datetime`, `enum`, `list` (of strings), `ref`, `ref[]` |
| `required` | all | must have a value |
| `default` | all | filled in by `typdoc new` |
| `values` | `enum` | allowed values; their order is also the sort order |
| `transitions` | `enum` | which value may follow which |
| `target` | `ref`, `ref[]` | `"*"` for any file, or a list of schema names |
| `acyclic` | `ref`, `ref[]` | no cycle may run through this field |
| `auto` | `date`, `datetime`, `list` | filled in on create, on update, or on move (`moves` records old names) |
| `override` | all | required to redefine a field inherited through `extends` |

`extends` points at a parent schema to inherit its fields.

## Keys and paths

| Schema | `match` | Example file | Named by |
| --- | --- | --- | --- |
| with `code` | `tickets/{key}.md` | `tickets/WF-3.md` | key `WF-3` (or its path) |
| without `code` | `notes/*.md` | `notes/setup.md` | path `notes/setup.md` |

- A coded document's file name **is** its key; the key is not stored in the frontmatter. `new`
  allocates it; a title never becomes a file name.
- A key is `CODE-number` (`[A-Z][A-Z0-9]*-\d+`). It never ends in `.md`, so a key and a path are
  never confused.
- A path argument that does not start with `/`, `./` or `../` is relative to the project folder —
  the same `path` `--json` prints.
- In frontmatter, refer to a coded document by key (`WF-2`); by path works but `refs.codedByPath`
  warns. Body links always use paths, relative to the document.

## A document file

```markdown
---
title: Write the schema
status: open
blocked_by:
- WF-1
---

## Notes

See [the database ticket](WF-1.md).
```

YAML frontmatter, then a free Markdown body. typdoc owns only the frontmatter. A file with no
`---` block is an ordinary Markdown file; an empty block is a document with no fields yet. A
field written with no value (`reviewer:`) prints as `null` in `--json`; one written `''` prints as
`""`. Fields the schema does not declare are kept, and reported by `frontmatter.unknown`.

## Namespaces

A namespace is a folder holding its own documents under the project's shared collections and
schemas. Keys are unique per namespace: two namespaces may each have a `WF-1`.

Which namespaces a command reads, first match wins:

1. a prefix on the argument: `story-2:WF-1`, `story-2:notes/x.md`;
2. `--namespace <list>` — names or `*` globs, comma-separated; `'*'` means every namespace;
3. `TYPDOC_NAMESPACE`, same syntax;
4. the current directory, when it is inside a namespace folder;
5. otherwise every namespace for reads — and a **write is refused** (exit 1, `the scope holds more
   than one namespace`).

A bare key found in more than one namespace in scope is exit 1 with `candidates`.

Every name typdoc prints can be passed to another command as is: with several namespaces in scope,
`list`, `list --ids` and `refs` print `story-1:WF-1`; in a one-namespace project they print `WF-1`.

## Imports

```json
{ "imports": { "memory": "../memory" } }
```

A ref crosses into an import with two colons: `memory::notes/lesson.md`, `memory::LRN-1`; into a
project with several namespaces, name one: `chief::story-3:WF-5`. Imported projects are read-only
— no command writes there.

A machine-specific location goes in `imports.json` (never committed), looked for under
`$TYPDOC_CONFIG_DIR/`, then `$XDG_CONFIG_HOME/typdoc/`, then `~/.config/typdoc/`; it adds to the
project's imports without overriding them. A path may use `${VAR}`; if the variable is unset or
empty the import is treated as absent, not as a path with the variable blanked:

```
notes/a.md  warn  imports.absent  the ref `memory::notes/lesson.md` does not resolve: NOPE_DIR is not set
```

An absent import is a warning by default (`imports.absent`); a project may set it to `error` in CI.

## State files

`.typdoc/state/<namespace>.json` records, per coded collection, the highest number ever issued:

```json
{
  "tickets": {
    "last": 4
  }
}
```

`typdoc new` and `mv --renumber` update it under the lock. It is the record that keeps a deleted
document's number from being issued again, so:

- never lower `last` to match the files, and never revert the file to an older version;
- on a merge conflict, take the higher `last`;
- write it by hand only when adopting typdoc on existing coded documents, set to the highest
  number ever used.

## Remote schemas

A schema path may be an `http(s)://` URL in the design, but 0.2.0 cannot fetch one: an unpinned
remote schema is the config error `config.schema-unpinned`. Use local schema files.
