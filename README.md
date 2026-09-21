# typdoc

Treat a folder of Markdown files with YAML frontmatter as typed, linked documents, and ask questions about them from the command line.

typdoc is a lens over files, not a format for them. Adopting it means adding a config and some schemas beside documents you already have; it never rewrites them. Remove it again and every file is exactly as usable as it was before.

```console
$ typdoc list --where status=open --where 'ref.all(blocked_by).status=resolved'
WF-2  Decide the numbering scheme  open  WF-1
```

## Status

Version 0.1.0. Not released, and not finished: this is the read-only half of the tool.

**What works today**

- `get`, `list`, `refs`, `toc` and `validate`, against a project on this machine and the projects it imports.
- Schemas with types, enums, refs and inheritance; rules over frontmatter, schemas, keys, file names, refs and body links; a query language for `list`, including conditions that follow refs.
- **Nothing is written.** Every command reads. The tool does not create, edit, move or fetch anything, and the state file it reads is never written back.

**What is not built yet**

- `new`, `set`, `mv` and `pull`. The design describes them; the binary does not have them, and says so if you ask.
- Plain text output for most commands. `list` prints a table and `validate --audit` prints a summary; `get`, `toc`, `refs` and plain `validate` need `--json` and exit 1 without it.
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
```

`docs/getting-started.md` walks through building a project of your own from an empty folder.

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

## Commands

| Command | What it answers |
| --- | --- |
| `typdoc get <key\|path>` | What are this document's fields, read under its schema? |
| `typdoc list` | Which documents match this query? |
| `typdoc refs <key\|path>` | What does this document point at, and what points at it? |
| `typdoc toc <key\|path>` | What are this document's headings, and which lines do they cover? |
| `typdoc validate [<key\|path>...]` | Does the project keep the promises its schemas and rules make? |

Every command takes `--json`. `docs/commands.md` has the options, the exit codes and the shape of what each one prints.

## Documentation

- `docs/getting-started.md` — build a project from an empty folder.
- `docs/projects.md` — config, collections, schemas, namespaces, imports.
- `docs/commands.md` — the five commands, their options and their output.

The design is the source of truth for behaviour, and the two documents below are written for whoever works on typdoc rather than for whoever uses it:

- `docs/design.md` — what typdoc does and why, in full. Where this README and the docs above disagree with it, it wins.
- `docs/design-decision-phase-1/` — the decisions behind the design, with the research they rest on.

## License

MIT. See `LICENSE`.
