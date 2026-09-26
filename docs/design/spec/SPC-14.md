---
title: Refs explained
status: active
migrated_from: docs/archived-design/design.md#refs
---

Refs come from two places, frontmatter fields and body links, and both resolve through the same
index.

## Frontmatter values

A `ref` or `ref[]` value is a plain string, read in this order:

| Value | Read as | Condition |
| --- | --- | --- |
| `WF-3` | Key in the document's own namespace | Matches `^[A-Z][A-Z0-9]*-\d+$` and the code exists in this project |
| `story-2:WF-5` | Key in the sibling namespace `story-2` | `story-2` is a namespace of this project |
| `story-2:notes/x.md` | Path inside the sibling namespace `story-2`, from its folder | Same |
| `memory::precedents/x.md` | Path inside the imported project `memory`, which has one namespace | `memory` is an alias in `imports` |
| `chief::story-3:WF-5` | Key in namespace `story-3` of the imported project `chief` | `chief` is an alias in `imports` |
| anything else | Relative path | Resolved from the document (`refBase: file`) or the namespace folder (`refBase: namespace`) |

`name:` reaches a sibling namespace and `name::` an import, and the two never fall back to each
other: a name that does not exist on the side the syntax names is an error, never a relative
path. A path that really contains a colon is written with a leading `./`. A ref into a project
with several namespaces must name one (`chief::story-3:WF-5`); `chief::WF-5` is an error there,
since only a one-namespace project has a `default`. A name may be both a sibling and an import;
the `names.shadowed` rule warns. Sibling names and import aliases may not collide with URL
schemes (`http`, `https`, `mailto`, `file`); `validate` enforces this, as `config.namespace-name`
for a namespace and as `schema.valid` for an import alias.

## Ownership rules

So that two namespaces never conflict:

1. A file is validated only by the namespace that owns it. A ref from another namespace or
   project checks only that the target exists, matches `target`, and has the linked heading.
2. `match` stops at a nested project's boundary; namespaces of one project never nest.
3. A bare schema name in `target` always means this project.
4. Imports are followed one level; imports of imports are ignored.
5. Writes stay inside the namespace the scope names, except `mv`, which also rewrites refs in the
   other namespaces of the same project. No write ever leaves the project.

## Schema drift

A schema names a schema of an imported project only through a qualified name in `target`
(`"memory::learning"`). If the imported project renames or removes that schema, the `target` no
longer names anything, and `validate` reports it under `schema.valid`, at the schema file of this
project and naming the ref field. A qualified name whose alias this project does not configure
is reported the same way, since either way the target names nothing. An import that is absent on
this machine is reported by `imports.absent` instead and is not an error here. A field that an
imported project renames is not checked ahead of time, because nothing in a project's files
records which fields of another schema it relies on; a query that names a field no schema in
scope defines is an error and not an empty result, so such a change is loud when the query runs.
Run `validate --schemas` in CI when projects live in different repos.

## Imports absent on this machine

`${NAME}` in an import path is replaced by the variable's value; one rule serves the `imports` in
`config.json` and the ones in `imports.json`. A variable that is unset, or set to an empty value,
is never replaced by an empty string: that would turn `${HOME}/projects` into `/projects`, a path
that may exist and be the wrong project. The import is instead treated as absent on this machine
and reported by `imports.absent`, with a message that names the variable (`TYPMEM_DIR is not
set`), which is different from a path that does not exist. A location with no project of its own
is absent in the same way. Other imports and namespaces load normally. Imports that differ per
machine exist so that no machine-specific path is committed; if one missing import stopped the
whole project from loading, the easy way out would be to commit the path, which is what this is
meant to prevent. So a project that needs its imports to be there should set `imports.absent` to
`error` in CI: a mistyped variable name is otherwise only a warning.

A location that does hold a project whose own config cannot be loaded is an ordinary error, not
an absent import: it is a mistake at a real location, and folding it into `imports.absent` would
hide it. A `project::` argument that names an import absent on this machine is bad arguments
rather than a finding: an argument is a direct request for that document.
