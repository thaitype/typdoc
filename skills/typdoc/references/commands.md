# Commands

typdoc 0.2.0. Eight commands. `get`, `list`, `refs`, `toc` and `validate` read; `new`, `set` and
`mv` write. `validate` is in [validation.md](validation.md).

Every command takes `--json` and `--namespace <list>`. A write command also takes
`--lock-timeout <seconds>` (default 5): how long to wait for the namespace's lock before exit 4.

Without `--json`, a single document prints as a labeled block (one `name: value` line per field);
a list prints as a table with a header row, and an empty result prints nothing.

## The shared shapes in `--json`

- **Name of a document**: `path` (from the project folder), `namespace`, `key` when coded, and
  `project` when it belongs to an imported project.
- **Document**: the name plus `code` (may be `null`), `collection`, `schema` and `fields` — all
  of the frontmatter, including fields the schema does not declare.
- **Finding**: `rule`, `level`, `message`, `path`, plus `namespace`/`collection`/`key` for a
  document, `field`, and `line`/`col` when known.
- A `number` is printed with the digits the document holds (`1e3` stays `1e3`).
- Output may gain fields in later versions: ignore any field you do not know.

---

## get

```
typdoc get <key|path> [--json]
```

One document's frontmatter.

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

$ typdoc get WF-2 --json
{"document":{"path":"tickets/WF-2.md","namespace":"default","key":"WF-2","code":"WF","collection":"tickets","schema":"ticket","fields":{"blocked_by":["WF-1"],"status":"open","title":"Write the schema"}}}
```

A value that does not fit its type is returned as written; `validate` reports it. Missing → exit 5.

## list

```
typdoc list [--collection <list>] [--code <list>] [--where <expr>]... [--fields <list>]
            [--sort <key>]... [--limit <n>] [--ids] [--json]
```

| Option | Meaning |
| --- | --- |
| `--collection <list>` | only these collections (comma-separated) |
| `--code <list>` | only collections whose schema has one of these codes |
| `--where <expr>` | a condition; may repeat, all must hold — see [query.md](query.md) |
| `--fields <list>` | extra columns for the text table |
| `--sort <key>` | `field`, `field:asc` or `field:desc`; may repeat, first breaks ties first |
| `--limit <n>` | print at most n; `total` still counts every match |
| `--ids` | one key or path per line, no header — for pipes |

The text table's columns are the name (key, or path), `title`, each field used in `--where`, then
`--fields`:

```console
$ typdoc list --collection tickets --sort estimate:desc --fields estimate
key   title             estimate
WF-2  Write the schema  5
WF-5  Cosmos, or SQL?   2
WF-1  Pick a database

$ typdoc list --collection tickets --limit 1 --json
{"documents":[{"path":"tickets/WF-1.md",…,"fields":{…}}],"total":3,"truncated":true}
```

No match is exit 0 with an empty result. Across several namespaces the text and `--ids` output
print `namespace:key` (`story-1:WF-1`), a name any other command accepts; a one-namespace project
prints the bare key. The first column is headed `key` when every row is a numbered document,
`path` when none is, and `document` when the result mixes both.

## refs

```
typdoc refs <key|path> [--reverse] [--field <f>] [--json]
```

The refs a document holds, or with `--reverse` the refs that point at it (this project's
namespaces only; imported projects are not scanned in 0.2.0). `--field` keeps one field; `$body`
means body links.

```console
$ typdoc refs WF-1 --reverse --json
{"document":{"path":"tickets/WF-1.md","namespace":"default","key":"WF-1"},"direction":"in",
 "refs":[{"path":"tickets/WF-2.md","namespace":"default","key":"WF-2","field":"blocked_by","written":"WF-1"}]}
```

Each reference is the name of the document at the other end, plus `field` and `written` (the
text as written in the file), and `line`/`col` for a body link. A reference that does not resolve
has no `path` and has `unresolved` instead: `not-found`, `import-absent` (the import is not on
this machine) or `bad-prefix` (the prefix names no namespace or import).

## toc

```
typdoc toc <key|path> [--depth <n>] [--json]
```

The body's headings with the lines they cover.

```console
$ typdoc toc notes/getting-started.md
line  end  level  heading
5     7    2      Links
```

`line` counts from the top of the file, frontmatter included. `end` is the last line of the
heading's section; ranges nest (a heading's range contains its sub-headings'), and `--depth` never
changes `end`. `--json` adds each heading's `slug` — what a `#heading` link must use.

A document with no headings prints nothing. With `--depth` filtering every heading out, text mode
says so on stderr (`no headings at depth ≤ 1 (1 heading is deeper)`) and exits 0.

To read one section: take `line` and `end` from `toc`, then read those lines of the file.

## new

```
typdoc new <CODE> "<title>" [--set k=v]...     # coded: the key is allocated
typdoc new <path.md> [--set k=v]...            # uncoded: you name the file
```

```console
$ typdoc new WF "Pick a database"
path: tickets/WF-1.md
collection: tickets
schema: ticket
namespace: default
key: WF-1
blocked_by: 
status: open
title: Pick a database

$ typdoc new notes/setup.md --set title="Setup notes" --json
{"document":{"path":"notes/setup.md","namespace":"default","code":null,"collection":"notes","schema":"note","fields":{"title":"Setup notes"}}}
```

- Coded: takes the namespace lock, issues the larger of (highest existing number, recorded
  `last`) + 1, writes the file and records the number. A deleted document's number is never
  reissued. The title is required; `WF` alone is exit 1.
- Uncoded: the path must fit an uncoded collection's `match` (else exit 1); no title argument —
  set `title` with `--set`.
- Defaults and `auto` fields are filled in; the output (text or `--json`) is the whole new
  document, so read the key from it.
- The value checks of `set` apply to `--set` (see [query.md](query.md#set-fields-and-new---set)).
- The destination already existing is exit 7, with nothing written and no number used.
- In a multi-namespace project, choose the namespace (`--namespace story-1` or run inside its
  folder) or it is exit 1.

## set

```
typdoc set <key|path> <field=value>... [--if <expr>]... [--json]
```

`field=value` sets, `field=` removes. At least one is required.

```console
$ typdoc set WF-1 status=done --if status=open --json
{"document":{"path":"tickets/WF-1.md",…,"fields":{…,"status":"done",…}}}

$ typdoc set WF-1 status=claimed --if status=open
typdoc: `status=open` is false                       # exit 3, nothing written
```

- The whole write happens under the namespace lock: read, `--if`, schema check, write. A false
  `--if` exits 3 with nothing written — use it as compare-and-set.
- Refused (exit 2, nothing written): an enum value outside `values`, a ref that does not resolve,
  a wrong type, a status change `transitions` does not allow, writing an `auto` field directly,
  and a write that itself changes an `acyclic` field to a value that closes a cycle (`a cycle
  passes through blocked_by`). A write that changes an `acyclic` field to break a cycle is
  allowed, and a write that never touches an `acyclic` field succeeds even if the document
  already sits on an unrelated, pre-existing cycle — `validate` still reports that cycle on its
  own.
- A bad escape in a value (`\q`) or an unescaped `*` is exit 1: see
  [query.md](query.md#set-fields-and-new---set).
- A field the schema does not declare is written anyway and reported later by
  `frontmatter.unknown`.
- `auto: update` fields are stamped when a value actually changes.

What a write keeps and loses. Values are carried across exactly — `1e3`, `0755`, `no`, `~`, a
date, `''`, long integers, text with newlines all come back as written; a field written with no
value stays that way. But the whole frontmatter block is rewritten, so these are lost on any
write, even one that changes a single field:

| Written in the file | After a write |
| --- | --- |
| comments anywhere in the block | gone |
| blank lines between fields | gone |
| `tags: [a, b]` | a block list, one item per line (`[]` when empty) |
| `title: 'Ship it'`, `status: "no"` | unquoted, unless quotes are needed to keep the value |
| `id:   WF-3` | one space after the colon |
| `&anchor` / `*alias` | the value written out in full at every place |
| `!!str`, `!Ref`, any tag | gone; the value stays |

The body is never touched. A document whose frontmatter depends on comments, anchors or tags is
one to edit by hand (then run `validate`).

## mv

```
typdoc mv <from> <to>                        # move / rename
typdoc mv <coded-doc> --renumber <namespace> # coded document into another namespace, new key
```

Moves the file and rewrites every ref this project holds to it — frontmatter and body links, in
every namespace — keeping each ref's written form (a key stays a key, a relative link stays
relative, `%20` and `<…>` are kept).

```console
$ typdoc mv notes/setup.md notes/install.md
path: notes/install.md
collection: notes
schema: note
namespace: default
title: Setup guide
rewritten: 1 ref in 1 document
unrewritten: none
findings: none

$ typdoc mv notes/install.md notes/setup.md --json
{"document":{…},"rewritten":[{"document":"notes/linker.md","field":"$body","before":"install.md","after":"setup.md"}],"unrewritten":[],"findings":[]}
```

- `rewritten`: every ref rewritten (document, field, before, after); text shows the count.
- `unrewritten`: refs still pointing at the old name, each with a `reason` —
  `imported-project` (imported projects are read-only), `mention` (plain text is never
  rewritten), `links-rule-off` (a body link where `body.links` is off). Fix these yourself.
- `findings`: what the destination's schema rejects. The move still happened and exits 0 —
  branch on `findings`, not on the code.
- A coded document cannot change path inside its namespace (its file name is its key): exit 1.
  `--renumber <namespace>` moves it to another namespace under the next key there, and refs to it
  are rewritten to the new name — `blocked_by: [WF-1]` in `story-2` becomes `[story-1:WF-2]` after
  `typdoc mv story-2:WF-1 --renumber story-1`.
- The destination existing, or naming the same file as the source, is exit 7 with nothing written.
- A `mv` in a large repository holds the lock longer; raise `--lock-timeout` if it hits exit 4.
- A `mv` that stops partway can be finished by running the exact same command again.
- Moving with `git mv` or a file tool instead leaves every ref pointing at the old name;
  `validate` then reports them as `refs.resolve` and `body.links` findings.
