# typdoc

Treat a folder of Markdown files with YAML frontmatter as typed, linked documents, and ask questions about them from the command line.

typdoc is a lens over files, not a format for them. Adopting it means adding a config and some schemas beside documents you already have; it never rewrites them. Remove it again and every file is exactly as usable as it was before.

```console
$ typdoc list --where status=open --where 'ref.all(blocked_by).status=resolved'
WF-2  Decide the numbering scheme  open  WF-1
```

That asks a folder of tickets which of them are open and waiting on nothing unfinished. Every part of it is explained in `docs/getting-started.md`; nothing here is a language you have to learn before the tool is useful.

## Status

Version 0.1.0. Not released, and not finished: `pull` and remote schemas are not built.

**What works today**

- `get`, `list`, `refs`, `toc` and `validate` read a project; `new`, `set` and `mv` (including `mv --renumber`, which moves a coded document to another namespace under a new key) write one. A ref may point into another project on this machine that yours imports, and is followed there, though a few checks stop at that edge; see below.
- Schemas with types, enums, refs and inheritance; rules over frontmatter, schemas, keys, file names, refs and body links; a query language for `list`, including conditions that follow refs.
- **A write changes only the fields it is given.** Every write is atomic — a temp file, then a rename — takes the namespace's lock, and is safe to interrupt: `SIGINT` and `SIGTERM` are caught, the lock is released, and two writers racing the same lock never issue the same key. Nothing outside the project is written, and no command deletes a document.

**What is not built yet**

- `pull`, remote schemas, and the project lock they need. The design describes them; the binary does not have them, and says so if you ask.
- Plain text output for most commands. `list` prints a table, `validate --audit` prints a summary, and `new`'s coded form and `mv --renumber` print the bare key; every other command needs `--json` and exit 1 without it.
- Some checks stop at the edge of an imported project. The ones known and deliberate are listed in `crates/typdoc/src/registry.rs`, each held in place by a test.

Linux is the platform this is run and tested on. Nothing else is claimed.

## Quick start

typdoc builds with a recent Rust toolchain and has no other requirements.

```console
$ git clone https://github.com/thaitype/typdoc
$ cd typdoc
$ cargo build
```

`examples/` is a small project you can copy. Run the tool inside it:

```console
$ cd examples

$ ./../target/debug/typdoc validate --json
{"summary":{"scope":"all","strict":false,"checked":{"namespaces":["default"],"documents":3},"findings":{"error":0,"warn":0,"info":0}},"findings":[]}

$ ./../target/debug/typdoc get WF-2 --json
{"document":{"path":"tickets/WF-2.md","namespace":"default","key":"WF-2","code":"WF","collection":"wayfinder","schema":"wayfinder","fields":{"title":"Decide the numbering scheme","status":"open","kind":"research",...}}}

$ ./../target/debug/typdoc list --where status=open --ids
WF-2

$ ./../target/debug/typdoc set WF-2 status=claimed --json
{"document":{"path":"tickets/WF-2.md","namespace":"default","key":"WF-2","code":"WF","collection":"wayfinder","schema":"wayfinder","fields":{"title":"Decide the numbering scheme","status":"claimed","kind":"research","blocked_by":["WF-1"],"created_at":"2026-09-16T09:00:00+07:00","updated_at":"...the moment of the write..."}}}
```

`set` writes the file; `git checkout examples/` in the clone undoes it. `docs/getting-started.md`
walks through building a project of your own from an empty folder.

## What a project looks like

A project is a folder with a `.typdoc` folder in it. Nothing about your documents changes.

```
.typdoc/config.json                  { "version": 1 }
.typdoc/collections/wayfinder.json   { "match": "tickets/{key}.md", "schema": "schemas/wayfinder.json" }
schemas/wayfinder.json               the fields a ticket has, and their types
tickets/WF-1.md                      an ordinary Markdown file
```

A document is frontmatter plus a body, and the body is yours:

```markdown
---
title: Decide the numbering scheme
status: open
kind: research
blocked_by: [WF-1]
---

# Decide the numbering scheme

Ordinary Markdown, which typdoc reads for its headings and links and otherwise leaves alone.
```

`docs/projects.md` explains collections, schemas, namespaces and imports.

## Concept and mental model

Two words carry most of the vocabulary, and they are easy to hold backward: **namespace** and
**collection**. `--namespace` and `--collection` are flags; error ids such as
`config.namespace-name`, `config.collection-name` and `collections.overlap` name one or the
other. Getting the two words swapped reads that whole vocabulary backward.

Take a project with two namespaces, `story-1` and `story-2`, each holding the same two
collections — a coded `_tickets` collection and an uncoded `_notes` collection:

```
.typdoc/config.json                  { "version": 1, "namespaces": ["story-1", "story-2"] }
.typdoc/collections/_tickets.json    { "match": "_tickets/{key}.md", "schema": "schemas/ticket.json" }
.typdoc/collections/_notes.json      { "match": "_notes/*.md", "schema": "schemas/note.json" }
schemas/ticket.json                  the fields a ticket has, with the code WF
schemas/note.json                    the fields a note has, with no code
story-1/_tickets/WF-1.md             a ticket in story-1
story-1/_notes/kickoff.md            a note in story-1
story-2/_tickets/WF-1.md             a ticket in story-2
story-2/_notes/kickoff.md            a note in story-2
```

`typdoc list` reads namespace and collection as separate columns:

```console
$ typdoc list --fields namespace,collection,key,path
WF-1                       Set up the folder         story-1  _tickets  WF-1  story-1/_tickets/WF-1.md
WF-1                       Choose the folder layout  story-2  _tickets  WF-1  story-2/_tickets/WF-1.md
story-1/_notes/kickoff.md  Kickoff notes             story-1  _notes          story-1/_notes/kickoff.md
story-2/_notes/kickoff.md  Kickoff notes             story-2  _notes          story-2/_notes/kickoff.md
```

`WF-1` appears twice, once per namespace: the same key, two different documents, no conflict,
because a key is unique within its namespace, never across the project.

Laid out on the two axes, a document always sits at one intersection:

| | `_tickets` (code `WF`) | `_notes` |
| --- | --- | --- |
| `story-1` | `WF-1` | `kickoff.md` |
| `story-2` | `WF-1` | `kickoff.md` |

Never in a namespace alone, and never in a collection alone — always one of each, at once.

A collection says what kind of thing a document is, and which set of rules checks it. A
namespace says what a document shares its numbering with, and which lock is held while it is
written.

## Commands

| Command | What it answers |
| --- | --- |
| `typdoc get <key\|path>` | What are this document's fields, read under its schema? |
| `typdoc list` | Which documents match this query? |
| `typdoc refs <key\|path>` | What does this document point at, and what points at it? |
| `typdoc toc <key\|path>` | What are this document's headings, and which lines do they cover? |
| `typdoc validate [<key\|path>...]` | Does the project keep the promises its schemas and rules make? |
| `typdoc new <CODE\|path> [title]` | Create a document; a coded schema allocates its key. |
| `typdoc set <key\|path> <field=value>...` | Change fields, optionally only if a condition holds. |
| `typdoc mv <from> [to\|--renumber <namespace>]` | Move or renumber a document, rewriting every ref this project holds to it. |

Every command takes `--json`. `docs/commands.md` has the options, the exit codes and the shape of what each one prints.

## Documentation

- `docs/getting-started.md` — build a project from an empty folder.
- `docs/projects.md` — config, collections, schemas, namespaces, imports.
- `docs/commands.md` — the eight commands, their options and their output.

The design is the source of truth for behaviour, and the two documents below are written for whoever works on typdoc rather than for whoever uses it:

- `docs/design/design.md` — what typdoc does and why, in full. Where this README and the docs above disagree with it, it wins.
- `docs/design/design-decision-phase-1/` and `docs/design/design-decision-phase-2/` — the decisions behind the design, with the research they rest on.

## License

MIT. See `LICENSE`.
