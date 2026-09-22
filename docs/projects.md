# Projects

A typdoc project is a folder with a `.typdoc` folder in it. This page describes what goes in that folder. `getting-started.md` builds one step by step; this one is for looking things up.

`docs/design/design.md` is the source of truth for all of it. Where this page is shorter, it is a summary, not a different rule.

## The layout

```
.typdoc/config.json                  the project: version, namespaces, imports, rule levels
.typdoc/collections/<name>.json      one file per collection
.typdoc/state/<namespace>.json       the last key number handed out per collection
schemas/*.json                       schemas, wherever you like; collections name them by path
```

Everything else in the folder is your documents, arranged however you already arrange them.

## Config

```json
{
  "version": 1,
  "namespaces": ["story-*", "archive"],
  "imports": { "memory": "../memory" },
  "validation": { "global": { "body.links": { "level": "error", "ignore": ["assets/**"] } } }
}
```

- `version` is the only required key.
- `namespaces` names folders, by name or by glob, that each hold their own documents. Without it a project has one namespace, `default`, and it is the whole folder.
- `imports` maps an alias to another project on this machine, so refs can cross into it. One level only: a project you import does not bring its own imports with it.
- `validation.global` sets the level and the options of a rule for the whole project. A collection can narrow it; see below.

## Collections

Write the schema first and the collection second: a collection names its schema by path, so the schema has to exist for it to point at. Namespaces come before either, and only when one folder of documents is not enough.

A collection says which files a schema applies to:

```json
{ "match": "tickets/{key}.md", "schema": "schemas/ticket.json" }
```

- `match` is a template, relative to the namespace folder. `*` matches within one name, `**` matches whole folders, and `{key}` captures the document's key. Neither `*` nor `**` matches a name that begins with a dot.
- `schema` is a path from the project folder.
- `validation` may appear here too, and merges over `validation.global` key by key: a collection that sets only a level keeps the options the project set.

A file matched by two collections is an error, never settled by whichever was read first. Fix the templates so that each file belongs to one.

## Schemas

```json
{
  "name": "ticket",
  "code": "WF",
  "extends": "./base.json",
  "fields": {
    "title":      { "type": "string", "required": true },
    "status":     { "type": "enum", "values": ["open", "done"] },
    "blocked_by": { "type": "ref[]", "target": ["ticket"], "acyclic": true }
  }
}
```

- `name` is how other schemas refer to it in `target`.
- `code` makes the documents keyed: a collection whose schema has a code gives its documents keys such as `WF-1`, and the collection's `match` says where the key sits in the name.
- `extends` inherits fields from another schema. Redefining an inherited field needs `"override": true`, so nothing silently changes meaning underneath you.

The field types are `string`, `number`, `bool`, `date`, `datetime`, `enum`, `list`, `ref` and `ref[]`. There is no `string[]` or `number[]`: a list of plain values is `list`, which holds strings, and `ref[]` is the only typed list. A type that is not one of these is reported by `schema.valid` rather than guessed at.

| Option | Applies to | Meaning |
| --- | --- | --- |
| `required` | all | Must have a value |
| `default` | all | Used when a document is created |
| `values` | `enum` | The allowed values, in order; that order is also the sort order |
| `transitions` | `enum` | Which value may follow which |
| `target` | `ref`, `ref[]` | `"*"` for any file, or a list of schema names |
| `acyclic` | `ref`, `ref[]` | Refuse a cycle formed through this field |
| `auto` | `date`, `datetime`, `list` | Filled in on create, on update, or on move |
| `override` | all | Required to redefine an inherited field |

A field with no value in a document is absent, which is not the same as empty: an absent field fails every positive condition in a query and satisfies every negated one.

## Namespaces

A namespace is a folder that holds its own documents and shares the project's collections and schemas. Two namespaces may each have a `WF-1`; keys are unique per namespace, not per project.

A command reads the namespace you are standing in, or the ones `--namespace` names, or all of them. A prefix on an argument reaches a sibling: `story-2:WF-5`, or `story-2:notes/x.md`.

## Imports

An import is another project on this machine, named by an alias:

```json
{ "imports": { "memory": "../memory" } }
```

A ref may then cross into it, written with two colons: `memory::LRN-1`, or `memory::notes/x.md`. A project with several namespaces must be told which one: `chief::story-3:WF-5`.

Two projects may import each other. Each names the other in its own config, and a ref crossing either way resolves; neither inherits the other's imports, because the one-level rule still holds.

A path that differs per machine belongs in `imports.json`, which holds the same shape as the `imports` key and is never committed:

```json
{ "memory": "/home/me/projects/memory" }
```

It is looked for under `TYPDOC_CONFIG_DIR`, then `XDG_CONFIG_HOME`, then the platform's own config folder, and what it holds is added to the project's own imports without overriding them. A path may use `${VAR}`; a variable that is unset or empty leaves the import absent rather than turning `${HOME}/x` into `/x`.

An import that is not on this machine is a warning by default, not an error: the point is that no machine-specific path has to be committed. Set `imports.absent` to `error` in CI if a missing import should fail there.

## State

A collection whose schema has a `code` records the last number it handed out, per namespace:

```json
{ "tickets": { "last": 12 } }
```

`typdoc new` and `typdoc mv --renumber` write this file, under the namespace's lock, and never take a number back down. Write it yourself only when adopting typdoc on documents a keyed collection already has, so that the number typdoc allocates next does not collide with one already in use — `validate` reports `state.missing` for a keyed collection that has documents and no entry.

Three rules read the file after that: `state.malformed` (error) for a `last` that is present and unusable — text, absent, negative, a fraction, or too large; `state.behind` (warn) for a `last` lower than the highest key that actually exists, which still allocates correctly but is a record nobody has checked; `state.retired` (warn) for an entry whose collection the project no longer has, kept rather than treated as a config error, since it is the only record that its numbers were ever issued. No command removes an entry or derives `last` from what exists on disk: deriving it would let a deleted document's number be handed out again.

On a merge conflict in this file, take the higher `last`; never take a side. The number the lower side loses may already have been handed to a document that was later deleted, and `last` is the only record left that the number was used — keeping the lower side lets `typdoc new` hand it out again, to the wrong document, with nothing afterward for `validate` to find. For the same reason, never revert this file to an older version.
