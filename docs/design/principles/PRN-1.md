---
title: The caller owns names
status: active
---

**typdoc never makes up a name and never rewrites one it is given.** The one name typdoc owns is
the number in a key, which it issues so that no two documents share one.

## Why

A name is how a person or an agent finds a document again, and only the caller knows what it should
say. A name typdoc derives, from a title for example, has to guess: a non-English title or one with
an emoji has no obvious file name, and a derived name changes meaning when the title does. A name
typdoc corrects is a name the caller did not choose and may not recognise.

## What follows

- `new` for a document without a code takes the path the caller types.
- A slug in a coded document's file name is given by the caller; typdoc does not derive it from
  the title, and a slug that breaks the rules is refused rather than repaired.
- The title and the file name are independent: changing the title never renames the file.
  Renaming is `mv`, which the caller asks for.

## Where it stops

typdoc still refuses a name that cannot work (a path no collection matches, a character a link
cannot carry). Refusing is not renaming.
