# Query syntax

`typdoc list --where` and `typdoc set --if` take the same expressions. `set` and `new --set` take
`field=value` pairs, described at the end.

An expression is one argument with no spaces around the operator. Put it in single quotes.

```console
$ typdoc list --where 'estimate>=3'
```

Several `--where` options must all hold. There's no OR between different fields.

## Conditions on a field

| Expression | Matches when | Example |
| --- | --- | --- |
| `f=v` | the field equals `v` | `status=open` |
| `f!=v` | the field doesn't equal `v` | `status!=done` |
| `f=a,b` | the field equals any of the values | `status=open,doing` |
| `f=text*` | the field matches the glob | `title=Cosmos*` |
| `f=*` | the field is present | `owner=*` |
| `f!=*` | the field is absent or empty | `estimate!=*` |
| `f=v` on a list | the list contains `v` | `blocked_by=WF-1` |
| `f<v`, `f<=v`, `f>v`, `f>=v` | ordering, for `number`, `date` and `datetime` fields | `updated_at<2026-09-01` |

Values are checked against the field's type. An enum value that isn't in the schema, or a field
no schema declares, is an error rather than an empty result, so a typo doesn't look like "no
matches".

Names, values and enum values are case-sensitive. `*` is the only wildcard.

### Special characters in a value

Only `,`, `*` and `\` are special. Put `\` in front of one to use it literally:

```console
$ typdoc list --where 'title=Cosmos\, or SQL?'
```

`=`, `<`, `>` and `!` need no escaping inside a value: the expression is split at the first
operator after the field name.

### Missing fields

A document without the field fails every positive condition and matches every `!=`. So
`status=open` and `status!=open` together cover every document exactly once.

The ordering operators are the exception: a missing field fails `<`, `<=`, `>` and `>=` too.

`f!=v` also matches documents whose schema has no field `f` at all. When a query spans several
collections, add `--collection` or `f=*` to keep those out.

### Large numbers

Numbers are compared as 64-bit floating point. Two values that differ only after about the
sixteenth significant digit can compare as equal: `--where 'count>99999999999999999998'` doesn't
match a document with `count: 99999999999999999999`. The value itself is always printed exactly
as the file has it.

## Conditions that follow refs

```
ref.all(blocked_by).status=done
│   │   │           └── condition on each document reached (optional for any/none)
│   │   └── the ref field to follow, or $body for body links
│   └── all, any or none
└── ref: refs this document holds.  refby: refs pointing at this document.
```

| Expression | Matches when | If there are no refs |
| --- | --- | --- |
| `ref.all(f).cond` | every document in `f` meets `cond` | matches |
| `ref.any(f)` | the document has at least one ref in `f` | doesn't match |
| `ref.any(f).cond` | at least one document in `f` meets `cond` | doesn't match |
| `ref.none(f)` | the document has no refs in `f` | matches |
| `ref.none(f).cond` | no document in `f` meets `cond` | matches |
| `refby.all(f).cond` | every document pointing here through `f` meets `cond` | matches |
| `refby.any(f)[.cond]` | at least one document points here through `f` (and meets `cond`) | doesn't match |
| `refby.none(f)[.cond]` | no document points here through `f` (that meets `cond`) | matches |

`ref.all(f)` without a condition is an error, since it would always match.

Examples:

```console
$ typdoc list --where status=open --where 'ref.all(blocked_by).status=done'   # ready to start
$ typdoc list --where 'ref.any(blocked_by).status!=done'                      # waiting on something
$ typdoc list --where 'refby.any(blocked_by)'                                 # blocking something
$ typdoc list --where 'refby.none($body)'                                     # nothing links here
$ typdoc list --where 'ref.any(blocked_by).path!=*'                           # a blocker that doesn't exist
```

Only one hop: the condition after `ref.*(f).` is a plain field condition.

A ref to a document that doesn't exist counts as a missing document. It fails positive conditions,
matches `!=`, and prints a warning.

## Built-in fields

Every document also has these, usable in any condition: `path`, `key` (numbered documents only),
`code`, `collection`, `schema`, `namespace`.

```console
$ typdoc list --where collection=notes
$ typdoc list --where 'path=drafts/*'
```

Schemas can't declare fields with these names, or names starting with `$`.

## Set values

`typdoc set <doc> field=value` and `typdoc new ... --set field=value` take the same pairs.

| Written | Result |
| --- | --- |
| `status=done` | sets `status` |
| `owner=` | removes `owner` |
| `'blocked_by=WF-1,WF-2'` | sets a list field to two items |
| `'title=One, two'` | a comma in a field that isn't a list is ordinary text |
| `'title=a\*b'` | `\*` is a literal star |
| `'title=a\\b'` | `\\` is a literal backslash |

An unescaped `*`, or a `\` before any other character, is an error: `*` means a wildcard in a
query, and requiring the escape keeps the two meanings from mixing.

The value is checked against the schema before anything is written.

## Quoting in the shell

Put expressions, `--set` pairs that contain special characters, and `--namespace` values in single
quotes. Unquoted, `>` redirects output to a file, `*` expands to file names, and `(` is a syntax
error. The shell changing an expression is worse than an error, because the changed expression
often still runs and gives a plausible wrong answer.

typdoc's documented examples are tested in `sh` and `bash`, where single quotes pass every
character through unchanged. Other shells aren't tested, and some treat `\` inside single quotes
differently (fish does). In those, avoid backslashes in values, or check the query against a case
whose answer you already know.
