---
title: GitHub is how Markdown is read
status: active
---

**A Markdown link, anchor or file name must work where people read the files: on GitHub.**

## Why

The documents typdoc manages are read in a browser far more often than through typdoc. A link that
passes `validate` but does not open on GitHub is a broken link to every reader, so typdoc checks
what GitHub does, not what a more lenient reading would allow.

## What follows

- A heading's slug follows GitHub's algorithm, and anchors are checked against it (`SPC-14`).
- A body link names its file exactly; a space must be written `<…>` or `%20`, since CommonMark
  does not allow it bare (`SPC-1`, `body.links`).
- A slug may not contain `#`, which a link reads as the start of an anchor.

## Where it stops

GitHub is the reference for how Markdown renders. It is not a reason to limit what typdoc accepts
beyond what renders correctly: a file name in any language is fine.
