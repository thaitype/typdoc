---
title: Queries explained
status: active
migrated_from: docs/archived-design/design.md#query
---

Each `--where` is evaluated while standing on one candidate document: a plain condition reads the
candidate's own fields, and a `ref.*` or `refby.*` condition follows arrows to other documents
and tests them. The same expressions serve `--if` on `set`, and, with the differences below,
`--set`.

## Grammar

```
expr     = ref-expr | plain
ref-expr = dir "." quant "(" f ")" [ "." plain ]   ; "all" requires the "." plain part
dir      = "ref" | "refby"
quant    = "all" | "any" | "none"
f        = field | "$body"                         ; a ref or ref[] field, or $body
plain    = field op value
field    = [A-Za-z_][A-Za-z0-9_-]*                 ; or a pseudo-field
op       = "!=" | "<=" | ">=" | "=" | "<" | ">"    ; longest match at the first operator after the field
value    = item { "," item }                       ; one comparison value only for < <= > >=
item     = { char | "*" | "\" ( "," | "*" | "\" ) }  ; "\" before anything else is an error
```

## Omitting the condition

Omitting `.EXPR` tests only whether arrows exist: `ref.any(blocked_by)` is "I have a blocker",
`refby.none(sources)` is "nothing cites me". `ref.all(f)` and `refby.all(f)` need a `.EXPR`;
without one they are errors, with a hint to use `any` or `none` in the same direction, since
"every arrow passes a condition that is not there" is always true. For `ref.*` an arrow is
counted from the value written in the field, dangling refs included, so a ticket whose only
blocker points at nothing does not look unblocked. To find dangling refs, use
`ref.any(f).path!=*`: a dangling ref has no document and so no `path`, which satisfies `!=` under
Absence and negation. A `refby` arrow always comes from a document that exists, so `refby` has no
dangling case.

## Reached documents

A reached document is read under its own schema. A reached document lacking the field, or a
dangling ref, counts as absent (see Absence and negation); every dangling ref a `ref.*` condition
reaches also warns on standard error.

## Absence and negation

`k!=v` is exactly NOT `k=v`, in every form (single value, list, glob, array). Something absent,
whether a document without the field or a dangling ref, fails every positive condition (`=` in
any form, `k=*`) and satisfies every `!=`. The ordering comparisons (`<`, `<=`, `>`, `>=`) are
the exception: absent fails them. This keeps paired queries complementary:
`ref.all(blocked_by).status=resolved` and `ref.any(blocked_by).status!=resolved` split the open
tickets between them, and none falls through. Because `k!=v` includes documents whose schema has
no `k`, use it with `--collection` (or add `k=*`) when a query spans collections.

## Names and scope

A field name unknown to every schema in scope is an error, not an empty result, while a document
whose own schema lacks a field that another schema in scope defines counts as absent. For a
plain condition the scope is the collections chosen with `--collection` or `--code`, or every
collection of the namespaces in scope when none is chosen. In `ref.*(f)` and `refby.*(f)`, `f`
must be a field of type `ref` or `ref[]`, or `$body`, and the arrows it follows are not bound by
the collections a plain condition is limited to. The scope of the condition after `ref.*(f)` is
the schemas named by `f`'s `target`, with no restriction when the target is `"*"`; after
`refby.*(f)` it is the schemas that define `f`; for `$body` in either it places no restriction.
A `target` that `schema.valid` reports as invalid places no restriction either: the fault is
reported once, there.

## Values and escaping

An unescaped `*` is a glob; alone, in `k=*`, it means "present". In a value only three characters
are special: `,` (separates alternatives), `*` (glob) and `\`. Put `\` before one to mean it
literally: `title=Cosmos\, or SQL`, `k=\*`. `\` before any other character, or at the end of a
value, is an error, which catches typos and leaves room to add special characters later. `*` is
the only glob; there is no `?` and no `[...]`. `k=*` means "present" and `k=\*` a literal star.
`=`, `<`, `>` and `!` need no escape in a value, because the expression is split at the first
operator after the field name, taking the longest of `!=`, `<=`, `>=`, `=`, `<`, `>`. Wrap the
whole expression in single quotes so the shell leaves `\`, `*`, `<` and `>` alone.

The same rules apply to `--where`, `--if` and `--set`, with two differences in `--set`: an
unescaped `*` is an error, since a value that is written has no glob, and `,` splits a value only
for an array field, being an ordinary character in the value of any other field.

## Syntax

An expression is read whole, as one argument: spaces belong to names and values, so
`status = open` is an error, with the hint `did you mean status=open?`. Field names, values, globs
and enum values are case-sensitive. An empty value is an error in `--where` and `--if` (use `k!=*`
to test for absent or empty); in `--set`, `k=` removes the field. On an array field `=` means some
element matches and `!=` means no element does. The condition after `ref.*(f).` is a plain
condition; another `ref.*` inside it is an error, as is anything after `)` that is not `.EXPR`. The
ordering comparisons take one value, so `k<a,b` is an error. In a list, each value is coerced by
the field's type on its own, and one that cannot be coerced makes the whole expression an error.

## Coercion

Values are coerced by the field's type. A value outside an `enum` is an error, so a typo fails
loudly. A glob, and `*` alone, are never coerced: they are matched against the value as text.

## Comparisons

`<`, `<=`, `>` and `>=` apply to `number`, `date` and `datetime` only; on any other type they are an
error. `datetime` values compare as instants, offsets included. A date-only value compared with a
`datetime` field compares against the field's date part. A document without the field, or whose
value does not fit the field's type, satisfies no ordering comparison. Always quote the expression:
`<` and `>` are shell redirections.
