# 5: What is the exact grammar of a `--where` expression?

Type: wayfinder:grilling
Status: resolved
Blocked by: None (can start immediately)

## Question

The design gives a table of expressions but no grammar. Decide, precisely enough to write a parser and its error messages: quoting and escaping of values that contain `,`, `*`, `=`, spaces or parentheses; what `k=a,b` means when the field is itself an array; how `k=*` glob and `k=v` on an array interact; the disambiguation of `k!=*`; operator precedence when `ref.any(f).status!=x` contains `!=`; whether `ref.all(f)` with no `.EXPR` is legal for every quantifier; and what is an error versus "no match" (the design says an out-of-enum value is an error except with globs).

Amend `docs/design.md` with the grammar.

## Answer

Decided 2026-09-19. The grammar is written into `docs/design.md`: a Grammar block in the Query section, plus the bullets Absence and negation, Values and escaping, Names and scope and Syntax that it refers to.

- **`!=` is exactly NOT `=`, in every form** (single value, list, glob, array). Something absent (a document lacking the field, a dangling ref) fails every positive condition and satisfies every `!=`. The ordering comparisons `<`, `<=`, `>`, `>=` are unchanged: absent fails them. Reason: the design's own paired queries (`ref.all(blocked_by).status=resolved` for the frontier and `ref.any(blocked_by).status!=resolved` for blocked tickets) must split the open tickets between them; under "absent never matches" a ticket whose blocker lacks `status` falls out of both lists silently. Cost accepted: `k!=v` includes documents whose schema has no `k`, so it belongs with `--collection`. Amended in `docs/design.md` (Reached documents, new Absence and negation bullet, Comparisons).
- **Escaping.** In a value only `,` (alternatives), `*` (glob) and `\` are special, and `\` before one makes it literal. `\` before any other character, or at the end, is an error (catches typos; leaves room for more special characters later). `*` is the only glob (no `?`, no `[...]`); `k=*` is "present", `k=\*` a literal star. `=`, `<`, `>` and `!` need no escape in a value: the expression is split at the first operator after the field name, longest match of `!=`, `<=`, `>=`, `=`, `<`, `>`. Same rules for `--where`, `--if` and `--set`; `--set` splits on `,` only for array fields. Docs recommend single quotes around the expression. Amended in `docs/design.md` (new Values and escaping bullet). Note for ticket 7: `\` also escapes in double-quoted shell strings and PowerShell/cmd differ.
- **Names and scope.** Field names are `[A-Za-z_][A-Za-z0-9_-]*`, and `schema.valid` enforces it on schema fields. The pseudo-field names (`path`, `key`, `code`, `collection`, `schema`, `namespace`) are reserved and so is any name starting with `$`; the list is closed, and a new pseudo-field is a breaking change. An unknown field name is an error. Scope of a plain condition: the collections selected, or all. In `ref.*(f)` and `refby.*(f)`, `f` must be a `ref` or `ref[]` field (or `$body`, valid only there) defined in a schema of this namespace or an import; the scope of the condition after `ref.*(f)` is `f`'s `target` schemas (everything reachable when `"*"`), after `refby.*(f)` the schemas that define `f`, and for `$body` everything reachable.
- **Syntax.** Whole expression is one argument, no whitespace tolerance (`status = open` errors with the hint `did you mean status=open?`); case-sensitive; empty value is an error in `--where`/`--if` (`--set k=` still removes); on an array `=` is "some element matches" and `!=` "no element does"; the condition after `ref.*(f).` is plain (no nesting; nothing after `)` but `.EXPR`); ordering comparisons take one value; each value of a list is coerced on its own and one failure fails the expression; in `--set` an unescaped `*` is an error and `,` in a scalar value is literal.
- **Omitting `.EXPR`.** `ref.all(f)` and `refby.all(f)` without a `.EXPR` are errors with the hint `use ref.any(f) or ref.none(f)` ("every arrow passes a condition that is not there" is always true). For `ref.*` without `.EXPR`, `any` and `none` count arrows from the value written in the field, dangling refs included, so a ticket whose only blocker points at nothing does not look unblocked. Dangling refs are found with `ref.any(f).path!=*` (a dangling ref has no document, so no `path`, which satisfies `!=` under the negation rule); the example is in the design. `refby` has no dangling case: its arrows come from documents that exist.

**Answers to the ticket's own questions:** quoting and escaping (above); `k=a,b` on an array field means some element matches either value; `k=*` on an array means non-empty; `k!=*` splits at the `!=` (longest operator), field `k`, value `*`, meaning absent or empty; `ref.any(f).status!=x` parses the `ref.any(f)` prefix first, then `.status!=x` as a plain condition; `ref.all` needs `.EXPR`, `any` and `none` do not; out-of-enum values are an error except with a glob (unchanged), unknown fields are an error, absent things fail positive conditions and satisfy `!=`.

**Amended in `docs/design.md`:** Query section (Grammar block, `all` rows now require `.EXPR`, Omitting `.EXPR`, Pseudo-fields, Reached documents, Absence and negation, Values and escaping, Names and scope, Syntax, Comparisons), and `schema.valid` (naming and reserved names) in Validation rules and the `validate` list.

**Passed on:** ticket 7 (the docs recommend single quotes around expressions; `\` is also a shell escape, and PowerShell and cmd differ). Ticket 9 needs fixtures for each error the grammar names.

## Not verified

Nothing was run; no parser exists. The grammar was not fed a corpus of real expressions. Every query example in the design (about 25) was checked against the new rules by reading, not by execution. One broke a rule: `refby.all(parent)` used a field that no schema in the design defines, which the new scope rule would reject; it now reads `refby.all(blocked_by)`. The others hold. Two examples (`ref.any(sources).status=retired`, `--collection wayfinder,decisions --where status=open`) depend on schemas the design does not show (`learning`, `decision`) defining `status`. Interaction of the naming rule with existing user schemas is untested because none exist.

**Amended by [13](13-multiple-namespaces-in-one-typdoc.md):** the scope of a query is the namespaces chosen by the scope order (prefix, `--namespace`, `TYPDOC_NAMESPACE`, working directory) inside one project and its imports; the `namespace` pseudo-field is `default` in a one-namespace project and an alias for documents reached through an import.

**Note from [7](7-platform-and-lock.md):** the advice to wrap an expression in single quotes is now a covered-shells rule (sh and bash, each only while a test runs it; zsh is not covered in v1), see Quoting in the shell in the design.
