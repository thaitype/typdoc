# 12: `mv --renumber`

Type: implementation
Status: resolved
Blocked by: 8, 11

## What this delivers

- `typdoc mv FROM --renumber NAMESPACE`: the destination is the flag's value, `mv` takes one positional in this mode and two in the other, and `--renumber` with no value is bad arguments with a message saying a namespace is required (decision 11).
- The next number issued from the destination's `last`, which is written before the document appears under its new key; the source's `last` never goes down; one state file written, the destination's (decision 13).
- The new key printed bare on standard output.
- Refused: a destination that is the namespace the document is already in, and any attempt to cross a project.

## Done when

- The same-namespace refusal writes nothing and leaves `last` unchanged.
- An interruption between the state write and the document appearing skips a number and never issues one twice, shown by a test.
- A ref left behind fails validation rather than resolving to another document.
- `--renumber` with no value, and a `project::` prefix on either argument, are exit 1.
