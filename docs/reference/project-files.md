# Project files

Everything typdoc reads as configuration, and the state it keeps.

```
.typdoc/config.json               the project
.typdoc/collections/<name>.json   one per collection
.typdoc/state/<namespace>.json    last number issued, per numbered collection
.typdoc/locks/                    write locks; don't commit
.typdoc/schemas/<name>.json       schemas (the usual place; any path works)
```

typdoc finds the project by walking up from the current directory to the nearest folder holding
`.typdoc/config.json`. Set `TYPDOC_DIR` to start from another folder instead. A folder further
down with its own `.typdoc/` is a separate project.

## config.json

```json
{
  "version": 1,
  "namespaces": ["story-*", "archive"],
  "imports": { "memory": "../memory" },
  "validation": {
    "global": {
      "body.links": { "level": "error", "ignore": ["assets/**"] },
      "frontmatter.unknown": { "level": "off" }
    }
  }
}
```

| Key | Required | Meaning |
| --- | --- | --- |
| `version` | yes | Always `1` |
| `namespaces` | no | Folders that each hold their own documents, by name or `*` glob. Without it the project has one namespace, `default` |
| `imports` | no | Alias to the path of another project on this machine |
| `validation.global` | no | Rule levels and options for the whole project. See [validation rules](validation.md) |

Namespace folder names may use ASCII letters, digits, `-` and `_`, and can't be `default`,
`http`, `https`, `mailto` or `file`.

An entry of `namespaces` prefixed with `!` excludes a folder an earlier entry matched, applied in
list order (the last entry that matches a folder decides). This is `namespaces`-only: a leading
`!` on `--namespace`/`TYPDOC_NAMESPACE` is a syntax error, not a way to exclude one there. See
[how to split work into namespaces](../how-to/use-namespaces.md#adopt-namespaces-one-folder-at-a-time).

An import path is relative to the project folder and may use `${VAR}`. If the variable is unset
or empty, the import counts as missing on this machine.

## Collections

One file per collection in `.typdoc/collections/`. The file name, without `.json`, is the
collection's name.

```json
{ "match": "tickets/{key}.md", "schema": ".typdoc/schemas/ticket.json" }
```

| Key | Required | Meaning |
| --- | --- | --- |
| `match` | yes | Which files belong to the collection, relative to each namespace folder |
| `schema` | yes | Path to the schema, from the project folder |
| `validation` | no | Rule levels for this collection's documents, merged over `validation.global` |

In `match`:

- for a schema with a `code`, use `{key}` exactly once and no wildcards: `tickets/{key}.md`
- for a schema without a code, use `*` (within one name) and `**` (any number of folders):
  `notes/*.md`, `docs/**/*.md`

Wildcards don't enter folders whose names start with `.`, but a folder named explicitly
(`.agents/notes/*.md`) is read. Symbolic links to folders aren't followed. A file matched by two
collections is an error.

Two collections can't share a numbered schema.

## Schemas

A schema is a JSON file anywhere in the project; collections point at it by path. Without a reason
to put it elsewhere, keep it in `.typdoc/schemas/`, next to the rest of the configuration.

```json
{
  "name": "ticket",
  "code": "TK",
  "extends": "./base.json",
  "fields": {
    "title":      { "type": "string", "required": true },
    "status":     { "type": "enum", "values": ["open", "doing", "done"], "default": "open",
                    "transitions": { "open": ["doing"], "doing": ["done", "open"] } },
    "blocked_by": { "type": "ref[]", "target": ["ticket"], "acyclic": true, "default": [] },
    "updated_at": { "type": "datetime", "auto": "update" }
  }
}
```

| Key | Required | Meaning |
| --- | --- | --- |
| `name` | yes | How other schemas refer to this one in `target` |
| `code` | no | Capital letters and digits. Makes documents numbered: `TK-1`, `TK-2` |
| `extends` | no | A parent schema to inherit fields from |
| `fields` | yes | Field name to definition |

Field names are letters, digits, `_` and `-`, starting with a letter or `_`.

### Field types

| Type | Holds |
| --- | --- |
| `string` | text |
| `number` | a number |
| `bool` | `true` or `false` |
| `date` | `2026-09-24` |
| `datetime` | `2026-09-24T14:30:00+07:00` |
| `enum` | one of `values` |
| `list` | a list of strings |
| `ref` | one reference to another document |
| `ref[]` | a list of references |

### Field options

| Option | For | Meaning |
| --- | --- | --- |
| `required` | all | The document must have a value |
| `default` | all | Filled in by `typdoc new` |
| `values` | `enum` | The allowed values, in order; also the sort order |
| `transitions` | `enum` | Which value may follow which, checked when `set` changes it |
| `target` | `ref`, `ref[]` | `"*"` for any document, or a list of schema names |
| `acyclic` | `ref`, `ref[]` | No chain of refs through this field may loop back |
| `auto` | `date`, `datetime`, `list` | `create`, `update` or `moves`: filled in by typdoc, never set by hand |
| `override` | all | Required when redefining a field inherited through `extends` |

A `list` field with `auto: moves` records a document's old names when it's moved, and the
`refs.moved` rule uses it to say where a stale ref should point now.

## Documents

A document is a Markdown file that starts with a frontmatter block:

```markdown
---
title: Write the landing page
status: open
blocked_by:
- TK-1
---

Anything here is the body.
```

- A file without a `---` block isn't a document. An empty block is a document with no fields yet.
- A numbered document's key is its file name. The key isn't stored in the frontmatter.
- In frontmatter, refer to a numbered document by key (`TK-1`). By path works, with a warning.
- Body links are ordinary relative Markdown links.
- `reviewer:` (no value) and `reviewer: ''` are kept as written; typdoc treats both as present and
  empty.

## State files

`.typdoc/state/<namespace>.json` records the highest number issued per numbered collection:

```json
{
  "tickets": {
    "last": 3
  }
}
```

`typdoc new` and `typdoc mv --renumber` update it. Commit it.

- Never lower `last`, and never revert the file to an older version. A deleted document's number
  would be issued again, and old refs to it would quietly point at the new document.
- On a merge conflict, keep the higher number.
- Write it by hand only when adopting typdoc on numbered files that already exist: set `last` to
  the highest number ever used. See
  [add typdoc to a folder you already have](../how-to/adopt-an-existing-folder.md).

## imports.json

Machine-specific import paths, never committed. typdoc reads the first of:

1. `$TYPDOC_CONFIG_DIR/imports.json`
2. `$XDG_CONFIG_HOME/typdoc/imports.json`
3. `~/.config/typdoc/imports.json`

```json
{ "memory": "/home/me/projects/memory" }
```

Its entries are added to the project's `imports` without overriding them.

## Environment variables

| Variable | Meaning |
| --- | --- |
| `TYPDOC_DIR` | Find the project from this folder instead of the current directory |
| `TYPDOC_NAMESPACE` | Default for `--namespace` |
| `TYPDOC_CONFIG_DIR` | Where to look for `imports.json` first |

## Remote schemas

The design allows a schema path to be an `http://` or `https://` URL. typdoc 0.2.0 can't fetch
one yet, so a remote schema is reported as `config.schema-unpinned`. Use local schema files.
