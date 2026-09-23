---
title: Exit codes explained
status: active
migrated_from: docs/archived-design/design.md#exit-codes-and-errors
---

Exit codes let an agent branch on what happened without parsing stderr text.

| Code | Meaning |
| --- | --- |
| 0 | Success, including an empty `list` result and a well-formed query that matches nothing |
| 1 | Bad arguments: a malformed option or expression, or a key/write ambiguous across namespaces |
| 2 | Validation failed: schema, type, enum, transition or ref |
| 3 | An `--if` condition was false; nothing written |
| 4 | Lock not acquired within the timeout |
| 5 | Not found: the key, path or file the command was asked to act on does not exist |
| 6 | I/O: a file or directory cannot be read or written |
| 7 | The destination already exists: the write would replace a file that is there |

A new code is added only when a caller has to act differently in response to it: not-found may
lead to creating the document, bad-arguments is a defect in the call and is not retried, and I/O
failure is a problem of the environment that may be retried. Finer detail than the code lives in
`details[].rule`, an id naming the specific cause.

`docs/design/catalog/exit-codes.md` holds the eight codes as plain numbers — the set a test
compares the CLI's own exit paths against.
