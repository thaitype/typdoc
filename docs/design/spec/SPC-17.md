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

## Slugs in file names

A coded document's file name may carry a slug after its key: `tickets/WF-8-lock-order.md` is the
document `WF-8`, and `lock-order` is its slug. The key is what identifies the document; the slug
is there for a person reading a folder listing or a link. typdoc never makes a slug up: the
caller gives it (`typdoc new WF "Decide lock order" --slug lock-order`), as the caller gives the
path of a document without a code. The title and the slug are independent, so changing the
title never renames the file, and changing the slug is a `mv` (`SPC-2`).

The collection's `slug` says which file names it accepts:

| `slug` | Accepts | `new` |
| --- | --- | --- |
| `optional` (the default) | `WF-1.md` and `WF-1-<slug>.md` | With or without `--slug` |
| `required` | `WF-1-<slug>.md` only | Without `--slug`: exit 1, nothing written |
| `none` | `WF-1.md` only, as before slugs existed | With `--slug`: exit 1, nothing written |

A `slug` value other than these three, or a `slug` in a collection whose schema has no code, is
the config error `config.collection-slug`. A name the collection does not accept is outside it,
like any other name its `match` does not fit, and `filename.pattern` reports it.

**Where the key ends.** The number is every digit after the code's `-`, so `WF-10.md` is `WF-10`
and never `WF-1` followed by something. A slug begins with the `-` right after the last digit:
`WF-12-x.md` is `WF-12` with the slug `x`, and `WF-1-2x.md` is `WF-1` with the slug `2x`. The
slug sits between the key and whatever the template has after `{key}`: `tickets/{key}.md` names
`tickets/WF-1-<slug>.md`, and `{key}/README.md` names the folder `WF-1-<slug>/`.

A template whose text right after `{key}` starts with a digit or a `-` (`{key}1.md`,
`{key}-notes.md`) cannot tell a slug from its own text. With `slug` `optional` or `required` such
a template is `config.match-template`, and the message says to set `slug` to `none`. It is the
one case where a project that loaded before slugs existed stops loading under the default, and
it stops loudly, with exit 2, rather than reading the same files differently.

**What a slug may contain.** Lowercase ASCII letters and digits, in words joined by single `-`:
it begins and ends with a letter or digit and holds no `--`. `new --slug` given anything else
exits 1 with nothing written, and the message states the rule; typdoc does not rewrite the value
into a valid one, since that would be making a slug up. A file whose text after the key is not a
valid slug (`WF-1-Lock Order.md`) is a name the collection does not accept.

> **Open question (seen by users).** The character rule above. The reasons for it: a body link
> names the file exactly and GitHub opens it literally, so a space or a non-ASCII letter has to
> be percent-encoded in every link to the file; paths are compared with their case (`SPC-14`)
> while the default file system on macOS ignores it, so two slugs that differ only in case would
> be two documents on Linux and one file on macOS; and with one spelling per slug nobody has to
> guess which one a file uses. The cost: a Thai slug is not possible. No length limit of its
> own; the file system's limit on a name applies.

**One key, one file.** `WF-5.md` and `WF-5-x.md` in one namespace are two files with the key
`WF-5`, which `keys.unique` reports, as it reports any two files that share a key. The key is
what counts; a namespace issues numbers as before, and the slug plays no part in them (`SPC-8`).

**A collection that requires a slug and has files without one.** Under `required`, `WF-1.md` is
a name the collection does not accept: it is outside the collection, `filename.pattern` reports
it, and every ref to `WF-1` no longer resolves until the file is renamed with `mv`.

> **Open question (seen by users).** The paragraph above follows the table literally: `required`
> accepts only names with a slug, and a name a collection does not accept is outside it. The
> other reading keeps `WF-1.md` a document of the collection, so its refs still resolve, and
> reports it under `filename.pattern` as missing its slug. Proposed: the literal reading, so that
> `slug` has one meaning (which names the collection accepts) and turning `required` on in an
> existing project is loud. Its cost is noise: one broken ref per ref, not one finding per file.

**A project from before slugs.** With `optional` as the default, an existing project reads more
files, never fewer: a `WF-1-x.md` that `filename.pattern` reports today becomes the document
`WF-1`. Such a file then has to satisfy the schema, is counted by `keys.unique` against any
`WF-1.md` beside it, and a path ref to it gets `refs.codedByPath`. A project where
`filename.pattern` is at its default level (`error`) was already failing on that file.

> **Open question (contradiction).** "Nothing valid today becomes invalid" does not hold in three
> cases, each about a `WF-1-x.md` that exists today: `filename.pattern` is `off` or `warn` and the
> file does not satisfy the schema, or shares its key with a `WF-1.md` (both newly errors); a glob
> collection in the same folder also matches it (`tickets/*-notes.md` and `WF-1-notes.md`), which
> is now `collections.overlap`; and a template with a digit or `-` right after `{key}` (Where the
> key ends). Proposed: state these three in the changelog as the upgrade note, and keep `optional`
> as the default.

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
