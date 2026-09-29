---
title: Markdown is read as Markdown readers read it
status: active
---

**A link, an anchor or a file name must work in the Markdown readers people use, not only in
typdoc.** typdoc checks Markdown by the common standard, CommonMark, rather than by a reading of
its own.

## Why

The documents typdoc manages are read in editors, in browsers and on code-hosting sites far more
often than through typdoc. A link that passes `validate` but does not open in those readers is a
broken link to every reader, so typdoc checks what they do, not what a more lenient reading would
allow.

## What follows

- A body link is a Markdown link, read as Markdown readers read it: a path from the document it is
  in, `./` and `../` included. It is never a key, and it carries no namespace or project prefix,
  which no Markdown reader follows. A name that reaches a document from anywhere belongs in a ref
  (`PRN-11`).
- A body link names its file exactly. A space must be written `<…>` or `%20`, since CommonMark does
  not allow it bare (`SPC-1`, `body.links`).
- A slug may not contain `#`, which every Markdown reader takes as the start of an anchor.
- Where CommonMark says nothing, typdoc follows the reading most Markdown readers share. Heading
  anchors are an example: CommonMark defines none, and typdoc derives them the way GitHub does,
  since that is the form most readers and tools reproduce (`SPC-14`).

## Where it stops

The Markdown standard is the reference for how a document renders. It is not a reason to limit
what typdoc accepts beyond what renders correctly: a file name in any language is fine.
