# How to split work into namespaces

Use namespaces when you want the same kinds of documents in several separate groups, each with
its own numbering: one set of tickets per story, per sprint, or per team. Each namespace is a
folder, and every namespace shares the project's collections and schemas.

## Turn a project into several namespaces

List the namespace folders in `.typdoc/config.json`, by name or with a `*` glob:

```json
{ "version": 1, "namespaces": ["story-*"] }
```

Then each folder that matches holds its own documents, laid out the way the collections say:

```
.typdoc/collections/tickets.json   { "match": "tickets/{key}.md", ... }
story-1/tickets/WF-1.md
story-1/tickets/WF-2.md
story-2/tickets/WF-1.md
```

`match` templates are relative to each namespace folder, so one collection file covers all of
them. Numbering is per namespace: `story-1` and `story-2` each have a `WF-1`, and each has its own
state file in `.typdoc/state/`.

A namespace folder's name may use letters, digits, `-` and `_`.

## Adopt namespaces one folder at a time

Prefix an entry with `!` to exclude a folder a wildcard would otherwise match:

```json
{ "version": 1, "namespaces": ["story-*", "!story-1", "!story-2"] }
```

Entries apply in list order, gitignore-style: the last entry that matches a folder decides
whether it's a namespace. A later `!` excludes what an earlier entry included, and a later plain
entry can re-include what an earlier `!` excluded:

```json
{ "version": 1, "namespaces": ["story-*", "!story-1", "story-1"] }
```

— here `story-1` ends up included again, since the plain entry comes last.

An excluded namespace is fully invisible: `validate`, `list`, `get` and `refs` all act as if its
folder does not exist, a `--namespace`/`TYPDOC_NAMESPACE` naming it explicitly fails the same way
naming a namespace that never existed does, and so does a write into it (`new`, `mv --renumber`).
Its `.typdoc/state/<name>.json`, if it already has one from before it was excluded, is left
untouched — nothing reads or writes it while the namespace stays excluded, so re-including it
later continues numbering from where it left off. This makes `!` a way to migrate a project to
namespaces one folder at a time, without moving every matching folder in the same commit.

An entry that starts with `!` and matches no folder — because the name is misspelled, or the
folder does not exist yet — is always silent, whether it's an exact name or a glob: it produces
no finding, unlike a plain entry naming an exact name that matches nothing (which still reports
`config.namespaces-entry`).

`!` is `namespaces`-only: `--namespace` and `TYPDOC_NAMESPACE` do not support a leading `!` — it
fails as a syntax error (`is not a namespace name or a glob`) rather than being read literally or
silently ignored.

## Create a document in a namespace

With more than one namespace, typdoc won't guess where a new document goes:

```console
$ typdoc new WF "Set up CI"
typdoc: the scope holds more than one namespace: story-1, story-2
```

Say which one, with `--namespace`:

```console
$ typdoc new WF "Set up CI" --namespace story-1
path: story-1/tickets/WF-3.md
...
```

or run the command from inside the namespace folder, where it's the default:

```console
$ cd story-1
$ typdoc new WF "Set up CI"
```

## Refer to a document in another namespace

Prefix the key with the namespace name and a colon: `story-2:WF-1`. The same works for paths:
`story-2:notes/kickoff.md`.

On the command line:

```console
$ typdoc get story-2:WF-1
```

In a frontmatter ref, a bare key means the document's own namespace, and a prefix reaches another:

```markdown
---
title: Ship the release
blocked_by: [WF-2, story-1:WF-3]
---
```

If you use a bare key on the command line and it exists in several namespaces, typdoc stops and
lists the choices rather than picking one:

```console
$ typdoc get WF-1
typdoc: `WF-1` is a key in more than one namespace: story-1:WF-1, story-2:WF-1
```

## Choose which namespaces a command reads

In order of precedence:

1. a prefix on the argument: `typdoc get story-2:WF-1`
2. `--namespace`: one name, several separated by commas, or a glob such as `'story-*'`;
   `--namespace '*'` means all of them
3. the `TYPDOC_NAMESPACE` environment variable, with the same syntax
4. the namespace folder you're standing in
5. otherwise, all namespaces for reading (and an error for writing)

```console
$ typdoc list --namespace '*'
key           title
story-1:WF-1  Set up the folder
story-2:WF-1  Choose the folder layout
```

Quote `'*'` so the shell doesn't expand it into file names.

## Move a document between namespaces

A numbered document gets a new key when it moves namespace. See
[move and rename documents](move-and-rename.md#move-a-numbered-document-to-another-namespace).
