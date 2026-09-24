# Getting started

In this tutorial we'll build a small ticket tracker for a website project, starting from an empty
folder. By the end we'll have tickets that block each other, a note that links to them, and a
project that typdoc checks for us. It takes about ten minutes.

You need typdoc installed (`typdoc --version` should print `typdoc 0.2.0`). The
[README](../README.md#install) shows how.

## 1. Create the project

Make a folder and tell typdoc it's a project:

```console
$ mkdir website && cd website
$ git init
$ mkdir -p .typdoc/collections .typdoc/schemas tickets notes
$ echo '{ "version": 1 }' > .typdoc/config.json
$ echo '.typdoc/locks/' > .gitignore
```

`.typdoc/config.json` is what marks the folder as a typdoc project. We'll keep the schemas in
`.typdoc/schemas/` so everything typdoc reads sits in one place; a schema can live anywhere in the
project, though, since collections point at it by path. typdoc creates
`.typdoc/locks/` while it writes, and it doesn't belong in git, hence the `.gitignore`.

Check that typdoc sees it:

```console
$ typdoc validate
```

No output and no error: an empty project is a valid one.

## 2. Describe a ticket

A schema says what a ticket looks like. Save this as `.typdoc/schemas/ticket.json`:

```json
{
  "name": "ticket",
  "code": "TK",
  "fields": {
    "title":      { "type": "string", "required": true },
    "status":     { "type": "enum", "values": ["open", "doing", "done"], "default": "open" },
    "blocked_by": { "type": "ref[]", "target": ["ticket"], "default": [] }
  }
}
```

Every ticket needs a title. Its status is one of three values and starts as `open`. It can list
other tickets that block it.

The `code` makes tickets numbered: they'll be called `TK-1`, `TK-2` and so on.

Now tell typdoc where tickets live. Save this as `.typdoc/collections/tickets.json`:

```json
{ "match": "tickets/{key}.md", "schema": ".typdoc/schemas/ticket.json" }
```

## 3. Create some tickets

```console
$ typdoc new TK "Choose a static site generator"
path: tickets/TK-1.md
collection: tickets
schema: ticket
namespace: default
key: TK-1
blocked_by: 
status: open
title: Choose a static site generator
```

typdoc picked the number, created `tickets/TK-1.md`, and filled in the defaults. Two more, one of
them blocked by the first:

```console
$ typdoc new TK "Write the landing page" --set blocked_by=TK-1
$ typdoc new TK "Buy a domain"
```

Open `tickets/TK-2.md`. It's an ordinary Markdown file:

```markdown
---
blocked_by:
- TK-1
status: open
title: Write the landing page
---
```

Anything you write below the frontmatter is yours; typdoc won't change it.

## 4. Ask questions

List everything:

```console
$ typdoc list
key   title
TK-1  Choose a static site generator
TK-2  Write the landing page
TK-3  Buy a domain
```

Now something more useful: which open tickets can we start right now, because nothing blocking
them is unfinished?

```console
$ typdoc list --where status=open --where 'ref.all(blocked_by).status=done'
key   title                           status  blocked_by
TK-1  Choose a static site generator  open
TK-3  Buy a domain                    open
```

`TK-2` isn't there, because `TK-1` isn't done yet. The single quotes matter: they stop the shell
from reading `(` and `)` itself.

## 5. Work a ticket

Start on `TK-1`, but only if it's still open:

```console
$ typdoc set TK-1 status=doing --if status=open
path: tickets/TK-1.md
...
status: doing
title: Choose a static site generator
```

Run the same command again:

```console
$ typdoc set TK-1 status=doing --if status=open
typdoc: `status=open` is false
```

Nothing was written this time. If two people try to claim the same ticket, only one of them gets
it.

Finish it and ask the same question as before:

```console
$ typdoc set TK-1 status=done
$ typdoc list --where status=open --where 'ref.all(blocked_by).status=done'
key   title                   status  blocked_by
TK-2  Write the landing page  open    TK-1
TK-3  Buy a domain            open
```

`TK-2` is ready now.

## 6. Catch a mistake

Edit `tickets/TK-3.md` by hand and change its status to something the schema doesn't allow:

```markdown
status: in-progress
```

Then check the project:

```console
$ typdoc validate
path             level  rule               message
tickets/TK-3.md  error  frontmatter.types  the field `status` is not one of the schema's values: `in-progress`
```

Put it right with `set`, which checks the value before writing:

```console
$ typdoc set TK-3 status=doing
$ typdoc validate
```

Clean again.

## 7. Add notes that link to tickets

Notes don't need numbers, so their schema has no `code`. Save `.typdoc/schemas/note.json`:

```json
{ "name": "note", "fields": { "title": { "type": "string", "required": true } } }
```

and `.typdoc/collections/notes.json`:

```json
{ "match": "notes/*.md", "schema": ".typdoc/schemas/note.json" }
```

A note without a code is named by its path, which you choose:

```console
$ typdoc new notes/site-ideas.md --set title="Ideas for the site"
```

Open `notes/site-ideas.md` and add a body with ordinary Markdown links:

```markdown
## Hosting

Whatever we pick in [the generator ticket](../tickets/TK-1.md) decides this.

## Look and feel

Keep it plain. See [the landing page ticket](../tickets/TK-2.md).
```

And one more note, `notes/weekly.md`, that links to the first:

```markdown
---
title: Weekly notes
---

Started collecting [site ideas](site-ideas.md).
```

typdoc reads those links. Ask what points at `TK-1`:

```console
$ typdoc refs TK-1 --reverse
document             field
notes/site-ideas.md  $body
TK-2                 blocked_by
```

`$body` means a link in the Markdown body; `blocked_by` is the frontmatter field.

## 8. Rename a file without breaking links

`site-ideas.md` isn't a great name. Rename it with typdoc:

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

Look at `notes/weekly.md`:

```markdown
Started collecting [site ideas](website.md).
```

typdoc found the link and rewrote it. Had we used `git mv` instead, that link would now point at
a file that doesn't exist, and `typdoc validate` would say so.

```console
$ typdoc validate
```

Still clean.

## What we built

We have a project with two kinds of document: numbered tickets that block each other, and notes
named by path that link to tickets and to each other. typdoc hands out ticket numbers, answers
questions about the tickets, refuses values the schema doesn't allow, and keeps links working when
files move. The files themselves are still plain Markdown.

Where to go next:

- Already have a folder of Markdown files? [Add typdoc to it](how-to/adopt-an-existing-folder.md).
- Want to know what else `--where` can do? See [query syntax](reference/queries.md).
- Curious why ticket numbers work the way they do? Read
  [keys, numbers, and why they're never reused](explanation/keys-and-numbers.md).
