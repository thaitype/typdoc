# typdoc

> **Public release 0.2 is ready to test.** typdoc is still in active development, so commands,
> output and file formats may still change between releases. What's there is checked by more than
> a thousand tests, run on Linux and macOS on every change, so it should behave the way the docs
> say. If it doesn't, please [open an issue](https://github.com/thaitype/typdoc/issues).

typdoc checks and queries a folder of Markdown files as if it were a small database. You write a
schema for your tickets, notes or decisions, and typdoc validates the frontmatter, resolves the
links between documents, and answers questions about them.

```console
$ typdoc list --where status=open --where 'ref.all(blocked_by).status=resolved'
key   title                        status  blocked_by
WF-2  Decide the numbering scheme  open    WF-1
```

That's every open ticket whose blockers are all resolved, read from plain `.md` files.

## Motivation

A lot of project knowledge already lives in Markdown with a bit of YAML on top: tickets, design
decisions, meeting notes, lessons learned. It's easy to read and easy to diff, and more and more
of it is written by coding agents rather than by hand.

What that setup doesn't have is anything that notices when it goes wrong. A ticket gets a status
nobody defined. Two tickets end up with the same number. A file is renamed and every link to it
now points at nothing. You find out when someone reads the broken page, if you find out at all.

The usual fix is to move the data into a real database or a tracker, and then the files are no
longer the source of truth. typdoc takes the other route. It adds no syntax and no new file
format: frontmatter stays ordinary YAML, links stay ordinary Markdown links, and a document still
opens fine on GitHub for someone who has never heard of typdoc. What typdoc adds is the checking.
A schema says what fields a document has, a collection says which files the schema applies to,
and from then on typdoc can tell you when a value, a reference or a link is wrong, hand out
numbers without collisions, and move files without breaking the links to them.

Every command prints `--json` and uses distinct exit codes, because the main users are agents.
The same folder works for people and for tools, and if you stop using typdoc you delete the
`.typdoc/` folder and nothing else changes.

## Install

typdoc is written in Rust and isn't on crates.io yet. Install the latest version from GitHub:

```console
$ cargo install --git https://github.com/thaitype/typdoc typdoc
$ typdoc --version
```

To install a specific release instead, add its tag: `--tag v0.2.0`.

It runs on Linux and macOS. Windows isn't supported; WSL works.

If you work with a coding agent, add the typdoc skill too. It teaches Claude Code and other
skill-aware agents how to find, create, change and move documents in a typdoc project:

```console
$ npx skills add thaitype/typdoc
```

## Concepts

### Documents

A **document** is one Markdown file with YAML frontmatter at the top:

```markdown
---
title: Decide the numbering scheme
status: open
kind: research
blocked_by: [WF-1]
---

Ordinary Markdown. typdoc reads the headings and links in here and never rewrites it.
```

typdoc owns the frontmatter. The body belongs to you.

### Schemas and collections

A **schema** says what fields a kind of document has. It's a JSON file you write:

```json
{
  "name": "wayfinder",
  "code": "WF",
  "fields": {
    "title":      { "type": "string", "required": true },
    "status":     { "type": "enum", "values": ["open", "claimed", "resolved", "closed"] },
    "blocked_by": { "type": "ref[]", "target": "*" }
  }
}
```

A **collection** says which files a schema applies to:

```json
{ "match": "tickets/{key}.md", "schema": ".typdoc/schemas/wayfinder.json" }
```

The two are separate on purpose. The schema is the shape of the data. The collection is where
that data lives. One `note` schema can serve both `notes/*.md` and `drafts/*.md`.

### Keys and paths

Every document has a name you use on the command line.

If its schema has a `code`, the document gets a **key** such as `WF-2`, and the key is its file
name: `tickets/WF-2.md`. `typdoc new WF "..."` picks the next free number for you, so you never
name these files yourself.

If the schema has no code, the document is named by its **path** from the project folder, like
`notes/setup.md`, and you choose that path when you create it.

A key never ends in `.md`, so typdoc can always tell which one you meant.

### Refs

A **ref** is one document pointing at another. There are two kinds:

- a frontmatter field of type `ref` or `ref[]`, such as `blocked_by: [WF-1]`
- an ordinary Markdown link in the body, such as `[setup notes](../notes/setup.md)`

typdoc resolves both, so it can tell you what a document points at, what points at it, and which
refs are broken. When you move a file with `typdoc mv`, it rewrites every ref to that file.

### The project

A **project** is the folder that holds `.typdoc/`:

```
.typdoc/config.json                 {"version": 1}
.typdoc/collections/wayfinder.json  which files are tickets, and their schema
.typdoc/schemas/wayfinder.json      the schema
.typdoc/state/default.json          the last ticket number handed out
tickets/WF-1.md                     your documents, wherever you already keep them
```

typdoc finds it the way git finds `.git`: it walks up from the current directory. `.typdoc/`
itself is what marks the project — `config.json` is optional; a folder with nothing under
`.typdoc/` yet still validates, just with a warning that there's nothing to check.

### Namespaces

Some projects need the same kind of document in several separate groups, for example one set of
tickets per story. A **namespace** is one of those groups: a folder with its own documents and
its own numbering.

```
story-1/tickets/WF-1.md
story-2/tickets/WF-1.md
```

Both files are `WF-1`, and that's fine, because keys are unique per namespace. You tell them
apart with a prefix: `story-1:WF-1`, `story-2:WF-1`.

It's easy to mix up namespaces and collections, so here's the difference. A collection is a
*kind* of document (tickets, notes). A namespace is a *group* of documents (story 1, story 2).
Every document sits in exactly one of each.

| | tickets (code `WF`) | notes |
| --- | --- | --- |
| `story-1` | `story-1:WF-1` | `story-1/notes/kickoff.md` |
| `story-2` | `story-2:WF-1` | `story-2/notes/kickoff.md` |

A project without namespaces has one, called `default`, and you never have to write it.

### Imports

A project can **import** another project on the same machine and point refs into it with two
colons, such as `memory::notes/lesson.md`. That's how a ticket tracker can cite a separate
knowledge base without copying it. Imported projects are read-only: no command writes there.

## How it works

Every run starts from the files. There's no cache and no database. typdoc reads the config,
matches every file against the collections, and builds an index of every key and path before it
answers anything. Change a file by hand and the next command sees it.
[How typdoc sees your files](docs/explanation/how-typdoc-sees-files.md) covers the index and how
refs resolve through it.

Writes are checked first and applied whole. `new`, `set` and `mv` check the change against
the schema before touching anything; if it's invalid, nothing is written. Each file is written to
a temp file and renamed into place, so a reader sees the old file or the new one, never half of
each. An interrupted command releases its lock on the way out.

Numbers are handed out under a lock and never reused. Each namespace has a lock that `new`
holds while it picks the next number and records it in `.typdoc/state/`. Two people or agents
creating tickets at the same moment get different keys, and a deleted ticket's number is never
given to a new one, so an old ref can't quietly land on the wrong document.
[Keys, numbers, and why they're never reused](docs/explanation/keys-and-numbers.md) explains why
the state file only ever goes up.

A write touches the frontmatter and nothing else. Values are kept exactly as written, but the
block is rewritten as a whole, so comments and hand formatting inside the frontmatter don't
survive a `set`. The body is never touched, except that `mv` updates link paths.

## Try it

The repository ships a small project in `examples/`: three tickets and a schema. Clone it and try
a few commands there:

```console
$ git clone https://github.com/thaitype/typdoc
$ cd typdoc/examples
$ typdoc list
key   title
WF-1  Set up the example project
WF-2  Decide the numbering scheme
WF-3  Should a ticket track its owner

$ typdoc new WF "Try typdoc on my own notes" --set kind=task
path: tickets/WF-4.md
collection: wayfinder
schema: wayfinder
namespace: default
key: WF-4
...

$ typdoc set WF-4 status=claimed --if status=open
path: tickets/WF-4.md
...
status: claimed
...

$ typdoc set WF-4 status=claimed --if status=open
typdoc: `status=open` is false

$ typdoc set WF-2 status=resolved
typdoc: transition not allowed: open -> resolved
```

The second `set` does nothing because the ticket isn't open anymore; `--if` is how two people
avoid claiming the same ticket. The last one fails because the schema says a ticket has to be
claimed before it's resolved. `git checkout examples/` puts everything back.

To build a project of your own from an empty folder, follow
[getting started](docs/getting-started.md).

## Commands

| Command | What it does |
| --- | --- |
| `typdoc get <doc>` | Show one document's fields |
| `typdoc list` | Find documents with `--where` conditions |
| `typdoc refs <doc>` | Show what a document points at, or with `--reverse`, what points at it |
| `typdoc toc <doc>` | List a document's headings and the lines they cover |
| `typdoc validate` | Check the whole project, or the documents you name |
| `typdoc new <CODE\|path>` | Create a document; a coded one gets the next key |
| `typdoc set <doc> field=value` | Change fields, optionally only `--if` a condition holds |
| `typdoc mv <from> <to>` | Move or rename a document and rewrite every ref to it |

`<doc>` is a key like `WF-2` or a path like `notes/setup.md`. Every command takes `--json`. The
[command reference](docs/reference/commands.md) has the options, output and exit codes.

## Documentation

Learn by doing:

- [Getting started](docs/getting-started.md): build a small ticket tracker from an empty folder

Get a specific job done:

- [Add typdoc to a folder you already have](docs/how-to/adopt-an-existing-folder.md)
- [Move and rename documents](docs/how-to/move-and-rename.md)
- [Split work into namespaces](docs/how-to/use-namespaces.md)
- [Link to another project](docs/how-to/link-projects.md)
- [Check a project in CI](docs/how-to/check-in-ci.md)

Look something up:

- [Commands](docs/reference/commands.md)
- [Query syntax](docs/reference/queries.md)
- [Project files](docs/reference/project-files.md): config, collections, schemas, state
- [Validation rules](docs/reference/validation.md)

Understand the design:

- [How typdoc sees your files](docs/explanation/how-typdoc-sees-files.md)
- [Keys, numbers, and why they're never reused](docs/explanation/keys-and-numbers.md)

## Status

This is 0.2.0. Everything in this README works. Two things from the design aren't built yet:
schemas fetched from a URL (and the `pull` command that updates them), and `refs --reverse`
following refs from inside a project you import. The [changelog](CHANGELOG.md) lists what changed
in each release.

## Contributing

Issues and pull requests are welcome. Start with the [development guide](docs/development.md): how
the workspace is laid out, how to run the tests, and what CI checks.

## License

MIT. See [LICENSE](LICENSE).
