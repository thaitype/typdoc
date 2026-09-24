# How to link to another project

Use an import when documents in one typdoc project need to point at documents in another: a
ticket tracker citing a separate knowledge base, for example. Both projects stay separate folders
with their own `.typdoc/`.

## Import the other project

In the project that does the pointing, add an alias under `imports` in `.typdoc/config.json`. The
path is relative to the project folder:

```json
{ "version": 1, "imports": { "memory": "../memory" } }
```

## Point refs into it

Use the alias and two colons:

```markdown
---
title: Uses a lesson
see: [memory::notes/lesson.md]
---
```

A key works too, `memory::LRN-1`. If the imported project has several namespaces, name one:
`chief::story-3:WF-5`.

typdoc follows these refs when it validates and queries, and you can read the other project's
documents directly:

```console
$ typdoc get memory::notes/lesson.md
```

The imported project is read-only from here. `set`, `new` and `mv` never write to it, and moving a
document here never rewrites refs over there.

An import is one level deep: a project you import doesn't bring its own imports with it. Two
projects can import each other.

## When the path differs between machines

If the other project isn't at the same relative path on every machine, don't commit the path.
Put it in `imports.json` in your own config folder instead, which typdoc reads from the first of
these that's set:

1. `$TYPDOC_CONFIG_DIR/imports.json`
2. `$XDG_CONFIG_HOME/typdoc/imports.json`
3. `~/.config/typdoc/imports.json`

It has the same shape as the `imports` key:

```json
{ "memory": "/home/me/projects/memory" }
```

Entries in this file are added to the project's own imports; they don't override them.

You can also use an environment variable in the path:

```json
{ "version": 1, "imports": { "memory": "${MEMORY_DIR}" } }
```

If the variable isn't set, typdoc treats the import as missing on this machine rather than
guessing a path.

## When the other project isn't there

A ref into an import that isn't on this machine is a warning, not an error:

```
notes/a.md  warn  imports.absent  the ref `memory::notes/lesson.md` does not resolve: MEMORY_DIR is not set
```

That keeps the project usable for someone who hasn't checked out the other one. If it has to be
there, in CI for example, make it an error:

```json
{ "version": 1, "validation": { "global": { "imports.absent": { "level": "error" } } } }
```

## What isn't followed yet

In 0.2.0, `typdoc refs --reverse` only looks in this project. A ref held in a project that
imports this one isn't in the result.
