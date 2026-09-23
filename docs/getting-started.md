# Getting started

This walks through building a typdoc project from an empty folder. Every command below was run as it is written; the outputs are the real ones.

You need a recent Rust toolchain to install the tool, and nothing else.

```console
$ git clone https://github.com/thaitype/typdoc
$ cd typdoc && cargo install --path crates/typdoc --bin typdoc
```

That puts `typdoc` in `~/.cargo/bin`, and this page calls it by that name throughout.

## A project is a folder with a `.typdoc` folder in it

Make a folder to work in and give it a config. The smallest one names only the version:

```console
$ mkdir -p my-notes/.typdoc/collections my-notes/schemas my-notes/notes
$ cd my-notes
$ echo '{ "version": 1 }' > .typdoc/config.json
```

Nothing else about the folder is special, and nothing about the documents you put in it has to change.

## Say what a document looks like

A *schema* says which fields a document has and what they hold:

```console
$ cat > schemas/note.json <<'JSON'
{
  "name": "note",
  "fields": {
    "title":  { "type": "string", "required": true },
    "status": { "type": "enum", "values": ["draft", "done"] }
  }
}
JSON
```

A *collection* says which files that schema applies to:

```console
$ echo '{ "match": "notes/*.md", "schema": "schemas/note.json" }' > .typdoc/collections/notes.json
```

## Write a document

An ordinary Markdown file with YAML frontmatter:

```console
$ cat > notes/first.md <<'MD'
---
title: My first note
status: draft
---

# My first note

The body is yours. typdoc reads its headings and links and changes nothing.
MD
```

## Ask typdoc about it

```console
$ typdoc validate --json
{"summary":{"scope":"all","strict":false,"checked":{"namespaces":["default"],"documents":1},"findings":{"error":0,"warn":0,"info":0}},"findings":[]}
```

One document checked and nothing to report. Now break it on purpose, by giving `status` a value the schema does not allow:

```console
$ printf -- '---\ntitle: Second\nstatus: nope\n---\n' > notes/second.md
$ typdoc validate --json
```

```json
{
  "rule": "frontmatter.types",
  "message": "the field `status` is not one of the schema's values: `nope`",
  "path": "notes/second.md",
  "level": "error"
}
```

The command ends with exit code 2 when a finding is an error, which is what makes it useful in a hook or in CI. Delete `notes/second.md` before going on.

## Give documents keys

A schema with a `code` gives its documents keys such as `WF-1`, and the collection's `match` says where the key sits in the file name:

```console
$ mkdir tickets
$ cat > schemas/ticket.json <<'JSON'
{
  "name": "ticket",
  "code": "WF",
  "fields": {
    "title":      { "type": "string", "required": true },
    "status":     { "type": "enum", "values": ["open", "done"] },
    "blocked_by": { "type": "ref[]", "target": ["ticket"] }
  }
}
JSON
$ echo '{ "match": "tickets/{key}.md", "schema": "schemas/ticket.json" }' > .typdoc/collections/tickets.json
$ printf -- '---\ntitle: First ticket\nstatus: done\n---\n' > tickets/WF-1.md
```

Validating now reports something you have not seen yet:

```console
$ typdoc validate --json
```

```json
{
  "rule": "state.missing",
  "message": "the collection `tickets` has documents in this namespace and no `last` recorded in its state file"
}
```

A collection whose schema has a `code` keeps the last number it handed out in a state file, so that two people numbering tickets at once do not collide. `WF-1` was written by hand rather than by typdoc, so nothing has recorded it yet; tell typdoc the highest number you have used, once, the same way you would when adopting typdoc on a folder of documents that already have keys:

```console
$ mkdir -p .typdoc/state
$ echo '{ "tickets": { "last": 1 } }' > .typdoc/state/default.json
$ typdoc validate --json
{"summary":{"scope":"all","strict":false,"checked":{"namespaces":["default"],"documents":2},"findings":{"error":0,"warn":0,"info":0}},"findings":[]}
```

## Create and change documents

From here on, `typdoc new` allocates the key and writes the state file for you — there is no state file to maintain by hand once one exists:

```console
$ typdoc new WF "Second ticket" --set status=open --set blocked_by=WF-1 --json
{"document":{"path":"tickets/WF-2.md","namespace":"default","key":"WF-2","code":"WF","collection":"tickets","schema":"ticket","fields":{"title":"Second ticket","status":"open","blocked_by":["WF-1"]}}}
```

The coded form of `new` takes a title and prints the bare key without `--json`; every other field goes through `--set`, the same `field=value` syntax `set` takes below. `new` validates the candidate before it allocates anything, so a `--set` that fails never burns a number, and it creates the file with no replace: a destination that already exists is refused, nothing written.

`typdoc set` changes fields on a document that already exists, under the same lock as the write:

```console
$ typdoc set WF-1 title="The first ticket" --json
{"document":{"path":"tickets/WF-1.md","namespace":"default","key":"WF-1","code":"WF","collection":"tickets","schema":"ticket","fields":{"title":"The first ticket","status":"done"}}}
```

Only `title` changed; every other field, and everything about the file that is not frontmatter, reads back exactly as it did. `typdoc mv` moves or renames a document and rewrites every ref this project holds to it; a coded document such as these two keeps its key within its own namespace and needs `mv --renumber <namespace>` to leave it, which `projects.md` and `commands.md` cover. None of the three writes anything without the namespace's lock, and all three refuse rather than guess wherever the destination is ambiguous.

## Follow the links

`blocked_by` is a ref field, so typdoc knows `WF-2` points at `WF-1`, and can answer in both directions:

```console
$ typdoc refs WF-1 --reverse --json
```

```json
{ "path": "tickets/WF-2.md", "field": "blocked_by", ... }
```

A ref that points at nothing is reported by `validate`, which is the thing that makes a folder of Markdown hold together as documents rather than as files.

## Ask questions

```console
$ typdoc list --collection tickets --where status=open --where 'ref.all(blocked_by).status=done'
WF-2  Second ticket  open  WF-1
```

That is the question worth asking a folder of tickets: what is open and not waiting on anything unfinished. `--where` may repeat, and every condition must hold. A condition can follow a ref, as `ref.all(blocked_by).status=done` does, and `all` is true for a document with no blockers at all.

`--collection tickets` is doing real work there. Without it the query spans every collection, and `status=open` is an error rather than an empty result, because the `note` schema allows `draft` and `done` and not `open`. A field name or an enum value that no schema in scope knows is a mistake worth hearing about, not a query that quietly matches nothing.

## Starting from notes you already have

The walkthrough above builds a project from nothing. Adopting typdoc on a folder that already exists is the other way round, and `--audit` is the mode for it: it answers "what would I have to fix to use this here", and it ends with 0 whatever it finds, so you can run it on a folder you have not decided about yet.

Take a folder holding five Markdown files, three under `notes/`, a `README.md` and a scratch file under `drafts/`. Give it the smallest config, one collection and one schema, and ask:

```console
$ typdoc validate --audit
typdoc audit: 1 collections, 5 files (2 in no collection)

notes  3 files   frontmatter.unknown 1 warn

in no collection: README.md, drafts/scratch.md (2)

no frontmatter: notes/plain.md (1)
```

Every file is accounted for: three in the `notes` collection, of which one has a field the schema does not name; two in no collection at all; and one of the three that has no frontmatter and so was listed rather than checked. Two and two and one make five, which is the point of the summary — it never reports less work than there is.

From there, adopting is a loop: widen `match` until the files you meant to cover are covered, add fields to the schema until the warnings are ones you care about, and leave the rest uncollected on purpose. When the audit is as clean as you want it, `validate` without `--audit` is the gate to put in a hook or in CI.

## Where to go next

- `projects.md` — namespaces, imports, match templates, and every field option a schema has.
- `commands.md` — the eight commands, their options, and their exit codes.
- `../examples/` — the project this page builds, ready to copy.
