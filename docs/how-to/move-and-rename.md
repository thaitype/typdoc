# How to move and rename documents

Use `typdoc mv` whenever you move or rename a document. It moves the file and rewrites every ref
in the project that points at it, both frontmatter refs and links in Markdown bodies. A plain
`git mv` or a file manager moves the file and leaves every link to it broken.

## Rename a document that has no code

```console
$ typdoc mv notes/site-ideas.md notes/website.md
path: notes/website.md
collection: notes
schema: note
namespace: default
title: Ideas for the site
rewritten: 1 ref in 1 document
unrewritten: none
findings: none
```

`rewritten` counts the refs typdoc changed. Each ref keeps the form it was written in: a relative
link stays relative, a link written with `%20` or `<...>` keeps that spelling. `git diff` shows
exactly what changed.

The destination must not exist yet. If it does, nothing happens and the command exits 7.

You can move a document into another folder, or into another collection, the same way. If the new
location's schema doesn't accept the document, the move still happens and `findings` lists what's
wrong; run `typdoc validate` afterwards and fix the document.

## Change the slug of a numbered document

A document with a key, like `WF-5`, keeps its key. The slug after the key in its file name is
for people, and changing it is a move to the same key with another slug:

```console
$ typdoc mv story-2:WF-5 story-2/_tickets/WF-5-json-shapes.md
path: story-2/_tickets/WF-5-json-shapes.md
collection: tickets
schema: ticket
namespace: story-2
key: WF-5
title: JSON output shapes
rewritten: 3 refs in 2 documents
unrewritten: none
findings: none
```

Each ref keeps the form it was written in. `blocked_by: [WF-5]` stays as it is, since the key did
not change. `WF-5-json-output-shape` becomes `WF-5-json-shapes`, and
`story-2:WF-5-json-output-shape` becomes `story-2:WF-5-json-shapes`. A link in a body names the
new file. Adding a slug (`WF-5.md` to `WF-5-json-shapes.md`) and removing one work the same way;
when the slug is removed, a ref written with it becomes the key alone.

If the ticket schema has a field with `auto: moves`, it records the previous path. A link someone
writes later to the old file name is then reported by `validate` as `refs.moved`, naming the new
path.

The new name must be the same key in the same folder. Another key is refused, and so is a slug
typdoc would not write itself: an empty one, or one with whitespace, `/`, `#` or `:`.

```console
$ typdoc mv story-2:WF-5 'story-2/_tickets/WF-5-json shapes.md'
typdoc: `story-2/_tickets/WF-5-json shapes.md` gives the key `WF-5` the slug `json shapes`, and a slug is not empty and holds no whitespace, `/`, `#` or `:`: nothing was written
```

If the collection's `slug` is `none` or `required` and the new name is in the other form, the
move still happens and `findings` lists `filename.pattern`, exit 0.

## Move a numbered document to another namespace

A document with a key, like `WF-2`, can't move to another key or out of its folder: its key names
its file. Trying is refused:

```console
$ typdoc mv WF-2 tickets/moved.md
typdoc: `tickets/WF-2.md` is a coded document: its path is fixed by its key `WF-2` within its own namespace, so it cannot be moved to `tickets/moved.md`
```

What you can do is move it into another namespace, where it gets the next free key there. Use
`--renumber` with the namespace's name:

```console
$ typdoc mv story-2:WF-1 --renumber story-1
path: story-1/tickets/WF-2.md
collection: tickets
schema: ticket
namespace: story-1
key: WF-2
...
rewritten: 1 ref in 1 document
unrewritten: none
findings: none
```

Every ref to the old key is rewritten to the new one. A ticket in `story-2` that said
`blocked_by: [WF-1]` now says `blocked_by: [story-1:WF-2]`. The old number is never handed out
again in `story-2`, so nothing new will ever answer to the old name.

A document with a slug keeps it under the new key. Starting again from
`story-2/_tickets/WF-5-json-output-shape.md`:

```console
$ typdoc mv story-2:WF-5 --renumber story-3
path: story-3/_tickets/WF-8-json-output-shape.md
collection: tickets
schema: ticket
namespace: story-3
key: WF-8
title: JSON output shapes
rewritten: 4 refs in 2 documents
unrewritten: none
findings: none
```

A ref by the key alone gets the new key, `WF-5` becoming `story-3:WF-8`, and a ref written with
the slug gets the new key and the same slug, `WF-5-json-output-shape` becoming
`story-3:WF-8-json-output-shape`. A field with `auto: moves` records the old key with its
namespace, `story-2:WF-5`. typdoc never drops a slug or makes one up here: a name in the form the
collection's `slug` does not expect, or with a slug holding whitespace, `/`, `#` or `:`, is
renumbered as it is, and `findings` lists `filename.pattern`.

See [use namespaces](use-namespaces.md) for how namespaces are set up.

## Deal with refs typdoc couldn't rewrite

Some refs to the old name can't be rewritten. `unrewritten` lists each one with a reason:

- `imported-project`: the ref is in another project that imports this one. typdoc never writes to
  another project. Go there and update it.
- `mention`: a plain-text mention of the moved key ("we decided this in WF-1"), not a ref at all,
  so `mv` never rewrites it — but it still surfaces here, at move time, so you don't have to wait
  for a later `typdoc validate` to find it.
- `links-rule-off`: the ref is a body link in a document where the `body.links` rule is switched
  off, so typdoc doesn't track that document's links.

If your project turns on the `body.mentions` rule, `typdoc validate` independently reports the
same mentions once they no longer resolve, agreeing with what `mv` already told you. Fix either
kind of leftover — a mention or a rule-off ref — by hand.

For the full list, with the file and field of each one, use `--json`:

```console
$ typdoc mv notes/website.md notes/site.md --json
{"document":{...},"rewritten":[{"document":"notes/weekly.md","field":"$body","before":"website.md","after":"site.md"}],"unrewritten":[],"findings":[]}
```

## If a move stops halfway

typdoc prepares everything before it renames anything, and moves the document itself last. If the
command is interrupted, run exactly the same command again to finish it.

In a large project `mv` holds the namespace lock longer than other commands. If another write is
waiting on it and gives up with exit 4, give that command a longer `--lock-timeout`.
