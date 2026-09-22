# 6: A field written with no value, kept apart from an empty string

Type: implementation
Status: claimed
Blocked by: 5

## What this delivers

- `reviewer:` and `reviewer: ''` stop being read as the same thing (decision 21). A write puts back the form it found, and `--json` shows the first as `null`.
- Inside typdoc the two go on meaning the same empty text, asserted rather than assumed, because making a bare field absent would turn documents that pass today into findings.
- The known gap `[empty-value]` is removed from `KNOWN_GAPS` in this change.

## Done when

- The test that pins the gap on both sides turns red in its old direction and is turned round, not deleted; the entry leaves the list in the same change.
- A document holding each form is written back with that form, byte for byte.
- A document with a bare field still validates as it did, shown by the fixtures that hold one.
