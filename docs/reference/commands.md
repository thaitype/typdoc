# Commands

typdoc 0.2.0 has eight commands. `get`, `list`, `refs`, `toc` and `validate` read a project and
never change a file. `new`, `set` and `mv` write.

## Options every command takes

| Option | Meaning |
| --- | --- |
| `--json` | Print one JSON object instead of text |
| `--namespace <list>` | Which namespaces to read: names or `*` globs, separated by `,` |
| `--lock-timeout <seconds>` | Write commands only. How long to wait for the namespace lock (default 5) |
| `-h`, `--help` | Help for the command |

`typdoc --version` prints the version.

## Naming a document

A command that takes a document accepts a key (`WF-2`) or a path from the project folder
(`tickets/WF-2.md`). Anything ending in `.md` is a path. A path starting with `/`, `./` or `../`
is relative to the current directory instead.

| Form | Means |
| --- | --- |
| `WF-2` | key in the current namespace |
| `story-2:WF-1` | key in namespace `story-2` |
| `story-2:notes/x.md` | path in namespace `story-2` |
| `memory::notes/x.md` | path in the imported project `memory` |
| `chief::story-3:WF-5` | key in namespace `story-3` of the imported project `chief` |

## Output

Without `--json`, a single document prints as one `name: value` line per field, and a list prints
as a table with a header row. An empty list prints nothing.

With `--json`, the result is one object on stdout. A document inside it looks like this:

```json
{"path":"tickets/WF-2.md","namespace":"default","key":"WF-2","code":"WF",
 "collection":"tickets","schema":"ticket","fields":{"title":"...","status":"open"}}
```

`key` appears only for documents whose schema has a code, and `project` only for a document in an
imported project. `fields` holds all the frontmatter, including fields the schema doesn't
declare. A number is printed with the digits the file holds.

On failure, the message goes to stderr, and with `--json` it's an object:

```json
{"error":"no document at WF-9","code":5,"details":[]}
```

---

## get

```
typdoc get <doc>
```

Prints one document's frontmatter.

```console
$ typdoc get WF-2
path: tickets/WF-2.md
collection: tickets
schema: ticket
namespace: default
key: WF-2
blocked_by: WF-1
status: open
title: Write the schema
```

`--json`: `{"document": <document>}`. A value that doesn't fit its type is shown as written;
`validate` reports it.

## list

```
typdoc list [options]
```

Prints the documents that match every `--where`.

| Option | Meaning |
| --- | --- |
| `--collection <list>` | Only these collections |
| `--code <list>` | Only collections whose schema has one of these codes |
| `--where <expr>` | A condition; may repeat, and all must hold. See [queries](queries.md) |
| `--fields <list>` | Extra columns for the table |
| `--sort <key>` | `field`, `field:asc` or `field:desc`; may repeat, the first breaks ties first |
| `--limit <n>` | Print at most n documents |
| `--ids` | Print one name per line, with no header |

The table's columns are the document's name, `title`, each field used in `--where`, then
`--fields`. The first column is headed `key` when every row is a numbered document, `path` when
none is, and `document` when the result mixes both.

```console
$ typdoc list --collection tickets --sort estimate:desc --fields estimate
key   title             estimate
WF-2  Write the schema  5
WF-5  Cosmos, or SQL?   2
WF-1  Pick a database
```

`--json`: `{"documents": [...], "total": n, "truncated": bool}`. `total` counts every match before
`--limit`; `truncated` is true when some were left out.

## refs

```
typdoc refs <doc> [--reverse] [--field <field>]
```

Prints the refs a document holds, or with `--reverse`, the refs that point at it.

| Option | Meaning |
| --- | --- |
| `--reverse` | Refs pointing at this document, from every namespace of this project |
| `--field <field>` | Only refs in this field; `$body` means links in the Markdown body |

```console
$ typdoc refs TK-1 --reverse
document             field
notes/site-ideas.md  $body
TK-2                 blocked_by
```

`--json`: `{"document": <name>, "direction": "out" | "in", "refs": [...]}`. Each ref names the
document at the other end, its `field`, and `written`, the text as it appears in the file; a body
link also has `line` and `col`. A ref that doesn't resolve has `unresolved` instead of a path:
`not-found`, `import-absent` or `bad-prefix`.

`--reverse` doesn't look inside projects that import this one.

## toc

```
typdoc toc <doc> [--depth <n>]
```

Prints the headings in a document's body and the lines each one covers.

```console
$ typdoc toc notes/site-ideas.md
line  end  level  heading
5     8    2      Hosting
9     11   2      Look and feel
```

Line numbers count from the top of the file, frontmatter included. `end` is the last line of the
heading's section, including its subsections. `--depth` lists only headings down to that level; it
doesn't change `end`.

A document with no headings prints nothing. If it has headings but `--depth` filters them all
out, a note goes to stderr.

`--json`: `{"document": <name>, "headings": [{"level", "text", "slug", "line", "end"}, ...]}`.
`slug` is what a `#heading` link to that section must use.

## validate

```
typdoc validate [<doc>...] [--schemas] [--strict] [--audit]
```

Checks the project, or only the documents named.

| Option | Meaning |
| --- | --- |
| `--schemas` | Check schemas and config only |
| `--strict` | Treat every warning as an error |
| `--audit` | Report for adopting typdoc on an existing folder; exits 0 unless the config is broken |

```console
$ typdoc validate
path                  level  rule               message
notes/broken.md:5:5   error  body.links         link target missing: nowhere.md
tickets/TK-3.md       error  frontmatter.types  the field `status` is not one of the schema's values: `in-progress`
```

Exits 2 if any finding is an error. The report goes to stdout whatever the result.

`--json`: `{"summary": {...}, "findings": [...]}`. The summary says what was checked (`scope`,
`checked.namespaces`, `checked.documents`) and counts findings by level. Each finding has `path`,
`rule`, `level`, `message`, and where known `line`, `col`, `field`, `key`.

Every rule is listed in [validation rules](validation.md).

## new

```
typdoc new <CODE> "<title>" [--set field=value]...
typdoc new <path.md> [--set field=value]...
```

Creates a document.

With a code, typdoc takes the namespace lock, picks the next number, creates the file, and records
the number in the state file. The title is required. With a path, the path has to fit one of the
collections' `match` templates, and there's no title argument; set `title` with `--set`.

Defaults and `auto` fields are filled in. The output is the new document, the same as `get`.

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

Refused with nothing written if a value breaks the schema (exit 2), or if the file already exists
(exit 7). In a project with several namespaces, choose one with `--namespace` or by running from
inside its folder.

## set

```
typdoc set <doc> <field=value>... [--if <expr>]...
```

Changes fields. `field=value` sets a field; `field=` removes it.

| Option | Meaning |
| --- | --- |
| `--if <expr>` | Only write if this holds; may repeat. Same syntax as `--where` |

```console
$ typdoc set TK-1 status=doing --if status=open
$ typdoc set TK-1 status=doing --if status=open
typdoc: `status=open` is false
```

The check, the `--if` conditions and the write happen under one lock. A false `--if` exits 3 and
writes nothing.

Refused with exit 2 and nothing written: a value of the wrong type, an enum value that isn't
allowed, a status change `transitions` doesn't allow, a ref that doesn't resolve, a ref that would
close a cycle on an `acyclic` field, and writing an `auto` field directly. A field the schema
doesn't declare is written, and `validate` reports it as `frontmatter.unknown`.

Value syntax, including commas in lists and escaping, is in [queries](queries.md#set-values).

### What a write keeps

Every value comes back exactly as written. The frontmatter block is rewritten as a whole, though,
so these don't survive:

| In the file | After a write |
| --- | --- |
| comments | removed |
| blank lines between fields | removed |
| `tags: [a, b]` | written as a block list, one item per line |
| `'quoted'` or `"quoted"` values | unquoted, unless quotes are needed |
| extra spaces after a colon | one space |
| `&anchor` and `*alias` | the value written out in full at each place |
| YAML tags such as `!!str` | removed; the value stays |

The body is never changed. If a document's frontmatter depends on comments, anchors or tags, edit
it by hand and run `typdoc validate`.

## mv

```
typdoc mv <from> <to>
typdoc mv <doc> --renumber <namespace>
```

Moves a document and rewrites every ref to it in this project, in frontmatter and in body links.
Each ref keeps the form it was written in.

| Option | Meaning |
| --- | --- |
| `--renumber <namespace>` | Move a numbered document into another namespace under the next key there |

A numbered document can't change its path inside its namespace; use `--renumber` to move it to
another one.

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

`--json` adds the details: `rewritten` lists each rewritten ref (`document`, `field`, `before`,
`after`), `unrewritten` lists refs that still point at the old name with a `reason`
(`imported-project`, `mention`, `links-rule-off`), and `findings` lists what the new location's
schema rejects. A move with findings still happens and exits 0.

The destination existing is exit 7, with nothing written. An interrupted `mv` can be finished by
running the same command again.

---

## Exit codes

| Code | Meaning | What to do |
| --- | --- | --- |
| 0 | Success, including a query that matched nothing | |
| 1 | The command was wrong: bad syntax, or a name that's ambiguous across namespaces | Fix the command. `--json` lists `candidates` for an ambiguous name |
| 2 | Validation failed, or the config can't be read | Read the findings |
| 3 | A `set --if` condition was false; nothing written | Read the document again and decide |
| 4 | The namespace lock wasn't free within `--lock-timeout` | Retry, or wait longer. The message says whether the lock's owner is still running |
| 5 | The document, key, path or project doesn't exist | Check the name |
| 6 | A file couldn't be read or written | Check permissions and disk |
| 7 | The destination already exists; nothing written | Pick another name |

A command interrupted with Ctrl-C releases its lock and ends by the signal, without one of these
codes.
