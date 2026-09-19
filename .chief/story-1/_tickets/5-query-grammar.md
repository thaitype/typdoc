# 5: What is the exact grammar of a `--where` expression?

Type: wayfinder:grilling
Status: open
Blocked by: None (can start immediately)

## Question

The design gives a table of expressions but no grammar. Decide, precisely enough to write a parser and its error messages: quoting and escaping of values that contain `,`, `*`, `=`, spaces or parentheses; what `k=a,b` means when the field is itself an array; how `k=*` glob and `k=v` on an array interact; the disambiguation of `k!=*`; operator precedence when `ref.any(f).status!=x` contains `!=`; whether `ref.all(f)` with no `.EXPR` is legal for every quantifier; and what is an error versus "no match" (the design says an out-of-enum value is an error except with globs).

Amend `docs/design.md` with the grammar.

## Answer

