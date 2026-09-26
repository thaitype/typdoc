---
title: Text output explained
status: active
---

One principle holds for every command's text output, without `--json`: a value is always
labeled with the name of the field it came from. No command prints a bare, unlabeled value on
its own line.

**`get`, `set`, and both forms of `new` print the same labeled block** — one `name: value` line
per field: `path`, `collection`, `schema` (the last two left out for a document `mv` has moved
out of every collection), `namespace` (left out for a file outside every namespace folder), `key`
(present only for a coded document), then the frontmatter fields in file order. A `number` field
prints the document's own digits, not the value they convert to — the same rule the write path
already holds itself to.

```console
$ typdoc new WF "Decide the numbering scheme"
path: tickets/WF-2.md
collection: wayfinder
schema: wayfinder
namespace: default
key: WF-2
title: Decide the numbering scheme
```

The coded form of `new` prints this block too, not a bare key. A caller that wants just the key
reads it out of `--json`, the same as any other field.

**`toc` is a table with a header row:** `line`, `end`, `level`, `heading`, one row per heading
down to `--depth`, columns separated by two spaces and padded to the widest value in that column
(header included).

**Two different empty results, told apart on stderr.** A document with no headings at all prints
nothing at all — stdout empty, stderr empty, exit 0. A document that *has* headings, all of them
filtered out by `--depth`, says so in one line on stderr instead — stdout still empty, exit still
0 — since silence alone can't tell "nothing here" from "wrong depth for this document." `--json`
is unaffected either way; it always returns the (possibly empty) `headings` array.

```console
$ typdoc toc notes/guide.md
line  end  level  heading
5     16   1      Getting started
9     12   2      Installation
13    16   2      Usage
17    19   1      Reference

$ typdoc toc notes/guide.md --depth 1
line  end  level  heading
5     16   1      Getting started
17    19   1      Reference

$ typdoc toc notes/subsections.md --depth 1
no headings at depth ≤ 1 (2 headings are deeper)

$ typdoc toc notes/no-headings.md
$
```

(`notes/subsections.md` has two headings, both at level 2 — `--depth 1` keeps neither. `--depth`
itself only accepts `1..=255`; `0` is bad arguments, exit 1, the same as any other out-of-range
value — not a way to ask for "nothing at any depth.")

**Both forms of `mv` print the destination's `get`-shaped block, then three lines always
present**, so a clean move reads as loud as a busy one:

- `rewritten: N ref(s) in M document(s)` — `N` is how many refs were rewritten, `M` is the number
  of distinct documents that held them (a document rewritten in two fields, or in one field and
  its body, counts once). Singular/plural on each count independently: `1 ref`/`N refs`,
  `1 document`/`M documents` — including `0 refs in 0 documents` for a clean move, since a count
  of zero still takes the plural form.
- `unrewritten:` — `none`, or its own count followed by one line per entry: the holder's identity,
  the field it lives in (`$body` for a body link), and the written form `mv` left untouched.
- `findings:` — `none`, or one line per finding in the destination's new schema, the same finding
  shape `--json`/`--audit` already expose. A move that lands on a schema the document does not
  satisfy still exits 0 — the move happened, and `findings` is what a caller reads instead.

```console
$ typdoc mv plain.md tasks/plain.md
path: tasks/plain.md
collection: tasks
schema: task
namespace: default
title: Plain note
moved_from: plain.md
rewritten: 0 refs in 0 documents
unrewritten: 2
holder-a.md  $body  plain.md
holder-b.md  $body  plain.md
findings:
tasks/plain.md#owner: frontmatter.types error: the field `owner` is required and is missing
tasks/plain.md#status: frontmatter.types error: the field `status` is required and is missing
```

A clean move, with nothing left unrewritten and nothing the destination's schema rejects, still
prints all three lines:

```console
$ typdoc mv a.md renamed.md
path: renamed.md
collection: notes
schema: note
namespace: default
title: A
rewritten: 2 refs in 1 document
unrewritten: none
findings: none
```

**`mv --json` carries `rewritten`**, the full list behind the text summary's count, one entry
per rewritten ref: `{document, field, before, after}`. It sits between `document` and
`unrewritten`.

```console
$ typdoc mv notes/first.md notes/renamed.md --json
{"document":{"path":"notes/renamed.md","namespace":"default","code":null,"collection":"notes",
  "schema":"note","fields":{"title":"A note"}},"rewritten":[],"unrewritten":[],"findings":[]}
```

There is no `--verbose` flag: the detail lives in `--json`, and a human at a terminal already has
it in `git diff`.

**`list` prints a header row** above its table: the identity column, `title`, then each
`--where`/`--fields` column, in that order. The identity column is labeled `key` when every matched
document has one (a coded collection), `path` when none does, and `document` when the matched set is
a genuine mix of both (spanning collections with and without a code) — a header must not claim a
column holds something a row in it plainly doesn't, and once even one row of each shape is present,
neither `key` nor `path` alone is accurate; the cell itself is already "key when coded, else path"
per row (`table_row`'s own `doc.key.unwrap_or(doc.path)`), so `document` is the label that fits
every row rather than misdescribing some of them. Widths are computed over every matched document,
not only the rows `--limit` prints, so a column's width never moves when `--limit` does. No header,
and nothing printed, when the result is empty. `--ids` prints one key or path per line, no header.

```console
$ typdoc list --collection tickets --where status=open --where 'ref.all(blocked_by).status=done'
key   title          status  blocked_by
WF-2  Second ticket  open    WF-1
```

A result spanning a coded collection and an uncoded one gets `document` instead:

```console
$ typdoc list
document    title
WF-1        Ticket one
notes/a.md  A note
```

**`refs` and plain/`--schemas` `validate` print a header row**, as `list` and `toc` do: `refs`
prints one ref per line with its field, and `validate` one line per finding, in the finding shape
`--json`/`--audit` expose, and the labeling principle asks for a header naming those columns.
Both use the same column-aligned, two-space-separated shape as `list`'s table: no header, and
nothing printed, when there is nothing to show, the same empty-result rule `list` and `toc`
follow.

`refs`' first two columns are always `document` — the document at the other end, the same identity
`list`'s own `document`/`key`/`path` column names (a coded document as its bare key when the project
has exactly one namespace and `namespace:key` when it has several, so a key is qualified only when
the project needs it; otherwise its bare path, with a `project::` prefix for an imported project's
document), or `(unresolved: <reason>)` when a forward ref did not resolve — and `field`. A third
column, `written`, appears only for the forward direction (no `--reverse`): `written` can genuinely
differ from the resolved `document` there (an alias, a relative form), so it is real information.
`--reverse` answers "who points at this document", and there `written` is only how the holder
happened to write the reference back to the very document already named on the command line — it
tells the reader nothing `document` doesn't already say, so it is dropped, header included.

```console
$ typdoc refs team/doc.md
document             field    written
chief:WF-7           context  chief:WF-7
team/learnings/x.md  $body    learnings/x.md
```

```console
$ typdoc refs WF-1 --reverse
document  field
WF-2      blocked_by
```

`validate`'s columns are reordered to `path`, `level`, `rule`, `message` — `rule` moves before
`message` so the column order matches `--json`'s own finding shape (`finding_json`'s field
order), and the header says `path`, matching `--json`'s `path` field, even though the printed
value is `path:line:col` when a position is known.

```console
$ typdoc validate
path          level  rule                 message
error.md:1:5  error  body.links           link target missing: ./nope.md
warn.md       warn   frontmatter.unknown  the field `extra` is not a field of the schema
```

**Every command's error path prints plain text, not the `--json` error object**, when it fails
without `--json`: `typdoc: <message>` on stderr. This applies to every command this document
covers.
