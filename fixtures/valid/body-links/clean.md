---
---

# Section One

A real [link](clean.md#section-one) that resolves, and a [reference link][ref] too.

[ref]: clean.md#section-one

Version numbers like [note](version 1.2) are not reported: the extension check needs a letter.

An undefined reference like [nope][absent] is plain text to CommonMark and is not reported.

A fenced block hides a fake definition from the parser:

```text
[fake]: should-not-be-real.md
```

Mentions: WF-1 is a real key and resolves; UTF-8 has the shape of one but is not a known code.
