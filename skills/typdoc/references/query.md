# Queries: `--where`, `--if`, `--set`

typdoc 0.3.0. `list --where` and `set --if` share one expression language. `set`'s
`field=value` arguments and `new --set` look similar but follow their own, simpler rules (last
section).

## Shape

An expression is **one shell argument with no spaces around the operator**:

```console
$ typdoc list --where 'status = open'
typdoc: expected an operator (!=, <=, >=, =, <, >), found ` = open`: did you mean status=open?
```

It is read while standing on one candidate document. A plain condition reads that document's
own fields; a ref condition follows its arrows to other documents. Every `--where` must hold (they
are ANDed); there is no OR across fields.

## Plain conditions

| Expression | Meaning | Example |
| --- | --- | --- |
| `k=v` | equals | `status=open` |
| `k!=v` | not equals | `status!=done` |
| `k=a,b` | equals any listed value | `status=open,claimed` |
| `k=pre*` | glob (`*` is the only wildcard) | `title=Cosmos*` |
| `k=*` | the field is present | `owner=*` |
| `k!=*` | the field is absent or empty | `estimate!=*` |
| `k=v` on a list or `ref[]` | the list contains `v` | `blocked_by=WF-1` |
| `k<v` `k<=v` `k>v` `k>=v` | ordering, on `number`, `date`, `datetime` only; one value | `estimate>=3`, `updated_at<2026-09-01` |

- **Operator**: the expression splits at the first operator after the field name, taking the
  longest of `!=`, `<=`, `>=`, `=`, `<`, `>`. So `=`, `<`, `>` and `!` need no escape inside a value.
- **Escaping in a value**: only `,`, `*` and `\` are special. Write `\,`, `\*`, `\\` for the
  literal character: `--where 'title=Cosmos\, or SQL?'`. `\` before anything else is an error.
- **Types**: values are coerced by the field's schema type. A value outside an `enum` is an error,
  not an empty result (`` `nope` is not one of the values `status` allows ``, exit 1).
- **Unknown field**: a field no schema in scope declares is an error (exit 1), not an empty
  result. Narrow the scope with `--collection` or `--code` when fields differ between collections.
- **Case**: field names, values, globs and enum values are case-sensitive.
- **Empty value**: an error in `--where`/`--if`; use `k!=*` to test absent-or-empty.

### Absence

A document without the field — or a ref that resolves to nothing — **fails every positive
condition and satisfies every `!=`**. So `status=open` and `status!=open` split a set with nothing
left over. The ordering comparisons are the exception: absent fails `<`, `<=`, `>`, `>=` too.

Because `k!=v` also matches documents whose schema has no `k`, pair it with `--collection` (or
add `k=*`) when a query spans collections.

### Numbers past `f64`

A `number` compares by the `f64` it converts to, not by its digits. Past about 16–18 significant
digits two different values can compare equal: `--where 'count>99999999999999999998'` does not
return a document holding `99999999999999999999`. (`--json` still prints the digits as written.)

## Ref conditions

```
ref.all(blocked_by).status=done
 │   │      │          └─ plain condition tested on each document reached (optional)
 │   │      └─ ref or ref[] field holding the arrows, or $body for body links
 │   └─ all | any | none
 └─ ref (arrows leaving me) | refby (arrows pointing at me)
```

| Expression | True when | Empty case |
| --- | --- | --- |
| `ref.all(f).EXPR` | every document in my `f` matches | true when `f` is empty |
| `ref.any(f)[.EXPR]` | at least one document in my `f` matches | false when empty |
| `ref.none(f)[.EXPR]` | no document in my `f` matches | true when empty |
| `refby.all(f).EXPR` | every document whose `f` points at me matches | true when none |
| `refby.any(f)[.EXPR]` | at least one document whose `f` points at me matches | false when none |
| `refby.none(f)[.EXPR]` | no document whose `f` points at me matches | true when none |

- Without `.EXPR`, `any`/`none` test only whether arrows exist: `ref.any(blocked_by)` = "has a
  blocker", `refby.any(blocked_by)` = "blocks something", `refby.none($body)` = "nothing links to
  me". `ref.all(f)` without `.EXPR` is an error (`use ref.any(f) or ref.none(f)`).
- `$body` means the document's body links: `refby.any($body)` = "some document links to me".
- One hop only: the part after `ref.*(f).` is a plain condition; no nested `ref.*`.
- A dangling ref (target missing) counts as an arrow to something absent: it fails positive
  conditions and satisfies `!=`, and is warned about on stderr. To find dangling refs:
  `ref.any(blocked_by).path!=*`.
- Rule of thumb: "is there such a document" → `any`; "is nothing in the way" → `all`; "is there
  none" → `none`. For a single `ref` field prefer `any`, since `all` is true when it is empty.

Worked examples (verified on 0.2.0):

```console
$ typdoc list --where 'ref.any(blocked_by).status!=done' --ids     # blocked by something unfinished
$ typdoc list --collection tickets --where status=open --where 'ref.all(blocked_by).status=done' --ids   # ready to start
$ typdoc list --collection tickets --where 'ref.none(blocked_by)' --ids                                  # no blockers at all
$ typdoc list --where 'refby.any(blocked_by)' --ids                  # blocks something
```

## Pseudo-fields

Usable like fields in any condition: `path`, `key` (coded documents only), `code`,
`collection`, `schema`, `namespace`. `namespace` is `default` in a one-namespace project. A schema
cannot define a field with one of these names or one starting with `$`.

## `set --if`

Same language as `--where`, checked **under the same lock as the write**. May repeat (ANDed). If
any is false, nothing is written and the exit code is 3; the error names the condition that
failed.

```console
$ typdoc set WF-1 status=claimed --if status=open
typdoc: `status=open` is false
```

## `set` fields and `new --set`

`field=value` sets a field; `field=` removes it from the frontmatter.

The value uses the same escapes as `--where`, but has no wildcard:

| Written | Stored |
| --- | --- |
| `'title=One, two'` | `One, two`: a comma is ordinary text in a scalar field |
| `'blocked_by=WF-1,WF-2'` | `[WF-1, WF-2]`: a comma splits only a list or `ref[]` field |
| `'title=a\,b'` | `a,b` |
| `'title=a\*b'` | `a*b`: `\*` is a literal star |
| `'title=a\\b'` | `a\b` |
| `'title=x*y'` | exit 1: a bare `*` is not allowed in a `set` value |
| `'title=a\qb'` | exit 1: only `\,`, `\*` and `\\` are escapes |
| `blocked_by=` | the field is removed (not set to an empty list) |

The value is checked against the schema before anything is written: an enum value outside
`values` is exit 2, a ref that does not resolve is exit 2 (`` the ref `WF-99` does not resolve:
not found ``), and a write that would put the document on a cycle through an `acyclic` field is
exit 2 (`` a cycle passes through `blocked_by` ``). Removing a ref to break an existing cycle is
allowed.

## Quoting in the shell

Wrap every expression and `--namespace` value in **single quotes**: `--where 'estimate>=3'`,
`--where 'title=Cosmos\, or SQL?'`, `--namespace '*'`. In sh and bash single quotes pass `\`, `*`,
`<`, `>` and `!` through untouched. Unquoted, `>` redirects to a file and `*` is expanded by the
shell — the danger is not an error but a changed expression that still parses and returns a
plausible wrong answer.

sh and bash are the shells typdoc's own tests run its documented examples through. zsh, fish,
PowerShell and cmd are not covered; on those avoid values with a backslash, or first run a query
whose answer you already know.
