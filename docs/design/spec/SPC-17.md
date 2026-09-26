---
title: Collections explained
status: active
migrated_from: docs/archived-design/design.md#config-typdocconfigjson
---

A collection is one file in `.typdoc/collections/` that maps files to a schema: which files belong
to it is its `match`, and the schema they are checked against is its `schema`.

## Loading

Every `*.json` file in `.typdoc/collections/` is a collection; other files are ignored. A file that
cannot be parsed, has an unknown key or names a schema that does not exist is a config error that
names the file, and typdoc stops rather than skip it, because a skipped collection would silently
shrink every result. Collections have no order; anything that lists them sorts by name.

A document matched by two collections is an error (`collections.overlap`), never settled by
precedence; the overlap is checked in each namespace. This is intended and not a limit waiting to be
lifted. A rule that named a winner, the more specific match or the first one, would leave someone
who reads two collection files unable to say which one wins without knowing a rule that neither file
states, and it would decide for them without saying so. `--audit` shows every collection, one that
ends with no document included, and names the collections of each overlap, so the overlap is seen
from both sides.

Numbering state is not kept in a collection file: a `last` key there is an unknown key and a config
error.

## Match templates

For a coded schema, `match` is a template with a placeholder; for a schema without a code, it is a
glob.

| Placeholder | Stands for | Allowed in |
| --- | --- | --- |
| `{key}` | `{code}-{number}`, with the code taken from the schema, e.g. `WF-3` | Coded schemas; exactly once, no globs |
| `*`, `**` | Glob | Schemas without a code; no placeholders |

One template serves both directions: it decides which files belong to the collection, and
`typdoc new` uses it to name new files. Because `{key}` includes the schema's code, several coded
collections can share one template in one folder: `tickets/{key}.md` matches `WF-3.md` for one
schema and `RFC-4.md` for another. Each code counts on one number sequence of its own in each
namespace, so a coded schema serves exactly one collection; two collections naming the same coded
schema is a config error, as is a state entry for a collection whose schema has no code.

## Which files a run reads

A glob does not enter a folder whose name begins with `.`, which is the rule namespaces follow too
(`SPC-7`), so a collection and a namespace answer the question the same way. A literal segment does
enter one: a project that keeps its documents under `.agents/` names that folder in `match` and gets
them, because naming a folder is saying it is wanted, while a glob is saying "whatever is here". A
`*` does match a leading dot in a file name, since a file is named by the template that reaches it
rather than found by walking into it. A symbolic link to a folder is not followed, so a run cannot
leave the project or read one file twice under two names. `.gitignore` is not read: what a version
control system hides is a different question from what a project declares, and a file that no
`match` reaches is already outside every collection. A directory entry a template reaches that is a
symbolic link, or whose name is not valid UTF-8, is skipped and reported under `files.unreadable`
rather than stopping the run: one name that cannot be read should not deny an answer about every
other file beside it.
