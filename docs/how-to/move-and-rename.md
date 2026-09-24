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

## Move a numbered document to another namespace

A document with a key, like `WF-2`, can't be renamed: its file name is its key. Trying is refused:

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
