# typdoc

Treat a folder of Markdown files with YAML frontmatter as typed, linked documents, and ask questions about them from the command line.

typdoc is a lens over files, not a format for them. Adopting it means adding a config and some schemas beside documents you already have; it never rewrites them. Remove it again and every file is exactly as usable as it was before.

```console
$ typdoc list --where status=open --where 'ref.all(blocked_by).status=resolved'
key   title                        status  blocked_by
WF-2  Decide the numbering scheme  open    WF-1
```

That asks a folder of tickets which of them are open and waiting on nothing unfinished. Every part of it is explained in the [getting started guide](docs/getting-started.md); nothing here is a language you have to learn before the tool is useful.

## Status

Version 0.2.0. Not released, and not finished: `pull` and remote schemas are not built.

**What works today**

- `get`, `list`, `refs`, `toc` and `validate` read a project; `new`, `set` and `mv` (including `mv --renumber`, which moves a coded document to another namespace under a new key) write one. A ref may point into another project on this machine that yours imports, and is followed there, though a few checks stop at that edge; see below.
- Schemas with types, enums, refs and inheritance; rules over frontmatter, schemas, keys, file names, refs and body links; a query language for `list`, including conditions that follow refs.
- Every command prints readable text without `--json`: a labeled block for `get`, `set`, `new` and both forms of `mv`; a table with a header row for `list` and `toc`; one line per ref for `refs`; one line per finding for `validate`. `--json` prints the same information as a single machine-readable document instead.
- **A write changes only the values of the fields it is given, and never the body.** Every write is atomic — a temp file, then a rename — takes the namespace's lock, and is safe to interrupt: `SIGINT` and `SIGTERM` are caught, the lock is released, and two writers racing the same lock never issue the same key. Nothing outside the project is written, and no command deletes a document.

**What is not built yet**

- `pull`, remote schemas, and the project lock they need. The design describes them; the binary does not have them, and says so if you ask.
- Some checks stop at the edge of an imported project: a reverse lookup does not enter one, and a `#heading` anchor across an import is not checked. Both are deliberate, and each is held in place by a test.

Linux is the platform this is run and tested on. Nothing else is claimed.

## Install

typdoc needs a recent Rust toolchain and nothing else. It is not published to crates.io yet, so
install it from the `v0.2.0` tag rather than from `main`:

```console
$ git clone --branch v0.2.0 --depth 1 https://github.com/thaitype/typdoc
$ cd typdoc
$ cargo install --path crates/typdoc
```

That puts `typdoc` in `~/.cargo/bin`, which is where the rest of this page and the
[getting started guide](docs/getting-started.md) expect to find it.

To work on typdoc rather than with it, `cargo build` leaves the binary at `target/debug/typdoc`
instead, and the [development guide](docs/development.md) has the rest: the workspace, the tests and
the lints.

## Quick start

The clone comes with a [small example project](examples/) you can copy. Run the tool inside it:

```console
$ cd examples

$ typdoc validate --json
{"summary":{"scope":"all","strict":false,"checked":{"namespaces":["default"],"documents":3},"findings":{"error":0,"warn":0,"info":0}},"findings":[]}

$ typdoc get WF-2 --json
{"document":{"path":"tickets/WF-2.md","namespace":"default","key":"WF-2","code":"WF","collection":"wayfinder","schema":"wayfinder","fields":{"title":"Decide the numbering scheme","status":"open","kind":"research",...}}}

$ typdoc list --where status=open --ids
WF-2

$ typdoc set WF-2 status=claimed --json
{"document":{"path":"tickets/WF-2.md","namespace":"default","key":"WF-2","code":"WF","collection":"wayfinder","schema":"wayfinder","fields":{"title":"Decide the numbering scheme","status":"claimed","kind":"research","blocked_by":["WF-1"],"created_at":"2026-09-16T09:00:00+07:00","updated_at":"...the moment of the write..."}}}
```

`set` writes the file; `git checkout examples/` in the clone undoes it. A write rewrites the whole
frontmatter block rather than the line it changed: every value is carried across exactly as it was
written, but comments, blank lines, quoting and inline lists such as `[WF-1]` are not kept.
The [design](docs/archived-design/design.md) has the full list of what a write does not promise to preserve.
[Getting started](docs/getting-started.md) walks through building a project of your own from an empty
folder.

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

Collections, schemas, namespaces and imports are explained under [projects](docs/projects.md).

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
.typdoc/state/story-1.json           { "_tickets": { "last": 1 } }
.typdoc/state/story-2.json           { "_tickets": { "last": 1 } }
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
key                        title                     namespace  collection  key   path
WF-1                       Set up the folder         story-1    _tickets    WF-1  story-1/_tickets/WF-1.md
WF-1                       Choose the folder layout  story-2    _tickets    WF-1  story-2/_tickets/WF-1.md
story-1/_notes/kickoff.md  Kickoff notes             story-1    _notes            story-1/_notes/kickoff.md
story-2/_notes/kickoff.md  Kickoff notes             story-2    _notes            story-2/_notes/kickoff.md
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

Every command takes `--json`. The [command reference](docs/commands.md) has the options, the exit codes and the shape of what each one prints.

## Documentation

- [Getting started](docs/getting-started.md) — build a project from an empty folder.
- [Projects](docs/projects.md) — config, collections, schemas, namespaces, imports.
- [Commands](docs/commands.md) — the eight commands, their options and their output.

## Contributing

Issues and pull requests are welcome. The [development guide](docs/development.md) is what to read
first: the four crates and why the split between them is load-bearing, how to build, how to run the
tests, and what the per-crate lints hold in place.

Two things about the tests are worth knowing before the first change. They run through
`scripts/test.sh`, which puts them under a memory ceiling, and a run started any other way has no
ceiling at all. And there is no CI here, so that run is the only thing between a change and the
trunk.

## License

MIT. See the [license](LICENSE).
