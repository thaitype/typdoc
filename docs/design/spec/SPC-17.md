---
title: Collections explained
status: active
migrated_from: docs/archived-design/design.md#config-typdocconfigjson
follows: [PRN-1, PRN-2, PRN-3, PRN-5, PRN-6, PRN-7, PRN-12]
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

## Collection files

Each collection is one file, `.typdoc/collections/<name>.json`. The file name without `.json` is
the collection's name: ASCII letters, digits, `-` and `_`, so it is unique by construction. The
file maps files to a schema. It is configuration only: numbering state lives in `.typdoc/state/`.

```json
// .typdoc/collections/wayfinder.json
{
  "match": "tickets/{key}.md",
  "schema": "schemas/wayfinder.json",
  "slug": "optional",
  "validation": { "body.mentions": { "level": "error" } }
}
```

| Key | Required | Meaning |
| --- | --- | --- |
| `match` | yes | Which files belong to the collection. See Match templates. |
| `schema` | yes | Relative path or `http://` or `https://` URL of the schema. See `SPC-16`. |
| `slug` | no | Coded schemas only: whether a file name carries a slug after its key, `optional` (default), `required` or `none`. See Slugs in file names. |
| `refBase` | no | How frontmatter paths resolve: `file` (default, relative to the document) or `namespace` (relative to the namespace folder) |
| `validation` | no | Rule levels and options for this collection only, merged over `validation.global`. See `SPC-1`. |

## Match templates

For a coded schema, `match` is a template with a placeholder; for a schema without a code, it is a
glob (`SPC-19`): `*` and `**`, and no other character of the syntax.

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

## Slugs in file names

A coded document's file name may carry a slug after its key: `tickets/WF-8-lock-order.md` is the
document `WF-8`, and `lock-order` is its slug. The key is what identifies the document; the slug
is there for a person reading a folder listing or a link. typdoc never makes a slug up: the
caller gives it (`typdoc new WF "Decide lock order" --slug lock-order`), as the caller gives the
path of a document without a code. The title and the slug are independent, so changing the
title never renames the file, and changing the slug is a `mv` (`SPC-2`).

A file whose name fits the template up to the end of its key is a document of the collection,
with a slug or without one. The collection's `slug` says which of the two forms it expects:

| `slug` | Expected file names | `new` |
| --- | --- | --- |
| `optional` (the default) | `WF-1.md` and `WF-1-<slug>.md` | With or without `--slug` |
| `required` | `WF-1-<slug>.md` | Without `--slug`: exit 1, nothing written |
| `none` | `WF-1.md` | With `--slug`: exit 1, nothing written |

A file in the form the collection does not expect, `WF-1.md` under `required` or `WF-1-x.md`
under `none`, stays a document of the collection: it is read, listed and checked against the
schema, and refs to it keep resolving. `filename.pattern` reports it at its level. So turning
`required` on in an existing project gives one finding per file to rename, and breaks no ref.

A `slug` value other than these three, or a `slug` in a collection whose schema has no code, is
the config error `config.collection-slug`.

**Where the key ends.** The number is every digit after the code's `-`, so `WF-10.md` is `WF-10`
and never `WF-1` followed by something. A slug begins with the `-` right after the last digit:
`WF-12-x.md` is `WF-12` with the slug `x`, and `WF-1-2x.md` is `WF-1` with the slug `2x`. The
slug sits between the key and whatever the template has after `{key}`: `tickets/{key}.md` names
`tickets/WF-1-<slug>.md`, and `{key}/README.md` names the folder `WF-1-<slug>/`. What follows
`{key}` in a template is plain text, never a glob, so the slug is what is left once that text is
taken off the end of the name: in `WF-1-v1.2.md` under `{key}.md` the slug is `v1.2`.

A template whose text right after `{key}` starts with a digit or a `-` (`{key}1.md`,
`{key}-notes.md`) cannot tell a slug from its own text. With `slug` `optional` or `required` such
a template is `config.match-template`, and the message says to set `slug` to `none`. It is the
one case where a project that loaded before slugs existed stops loading under the default, and
it stops loudly, with exit 2, rather than reading the same files differently. Under `none`, such
a template reads names exactly as it did before slugs existed and never looks for a slug; every
other template under `none` reads a slug as the form the collection does not expect, above.

**What a slug may contain.** Any characters a file name can hold, in any language, except
whitespace, `/` (a slug is part of one path segment), `#` (a Markdown link reads what follows it
as an anchor) and `:` (a ref reads what comes before it as a namespace, `SPC-14`). It is not
empty, and its case is kept as written. `new --slug` given anything else exits 1 with nothing
written, and the message states the rule; typdoc does not rewrite the value into a valid one,
since that would be making a slug up. A file whose text after the key holds an excluded
character (`WF-1-lock order.md`) is still the document `WF-1`: the key decides, and
`filename.pattern` reports the name, as it does for the form the collection does not expect.

**One key, one file.** `WF-5.md` and `WF-5-x.md` in one namespace are two files with the key
`WF-5`, which `keys.unique` reports, as it reports any two files that share a key. So are
`WF-5-a.md` and `WF-5-A.md`, which are two files on Linux and one on the default file system of
macOS: `keys.unique` reports them before the difference in case matters. The key is what
counts; a namespace issues numbers as before, and the slug plays no part in them (`SPC-8`).

**A project from before slugs.** An existing project reads more files, never fewer: a
`WF-1-x.md` that `filename.pattern` reports today becomes the document `WF-1`. Such a file then
has to satisfy the schema, is counted by `keys.unique` against any `WF-1.md` beside it, and a
path ref to it gets `refs.codedByPath`. A project where `filename.pattern` is at its default
level (`error`) was already failing on that file. A project that loaded and passed before can
fail after the upgrade in three cases, and the changelog lists them:

1. `filename.pattern` is `off` or `warn`, and a `WF-1-x.md` does not satisfy the schema or shares
   its key with a `WF-1.md`.
2. A collection with a glob in the same folder also matches a `WF-1-x.md` (`tickets/*-notes.md`
   and `WF-1-notes.md`), which is now `collections.overlap`.
3. A template with a digit or `-` right after `{key}` (Where the key ends).

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
