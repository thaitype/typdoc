---
title: Config errors explained
status: active
migrated_from: docs/archived-design/design.md#validation-rules
---

A config error is reported when `.typdoc/config.json` or a collection file loads, not when a
document is checked -- a different concept from the validation rules `SPC-1` explains, which
fire against documents. Each has an id, so a caller can branch on it without reading the
message, and so that each one can have a fixture that shows it fires.

What a config error does is decided by one question: does it make checking impossible? One that
does stops the command, prints the error object on standard error, and exits 2; the object
carries every config error that can be determined, not only the first, because
`validate --audit` exists to say what has to be fixed before adopting typdoc. An error that ends
the list early because the rest of the config could not be interpreted marks the object
`"complete": false`; every other config error object has `"complete": true`. A config error that
leaves checking possible is instead a finding in `validate`'s report, with its `config.` id and
`file`, and stops nothing.

`docs/design/catalog/config-errors.md` holds the machine-readable form of this same list: twenty
ids, each with the short text naming when it is reported. This document explains why the ids
exist and what happens once one fires; the catalog is what code and tests read.
