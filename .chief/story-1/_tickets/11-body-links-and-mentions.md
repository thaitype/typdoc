# 11: Body links, anchors and mentions

Type: implementation
Status: open
Blocked by: 6, 10

## What this delivers

- The link forms that are checked (inline, image, reference-style with definitions reported once and their use count, duplicate labels, text that looks like a link) and `$body` as a ref field.
- `body.links`, `body.anchors` (percent-decoded, case-insensitive) and `body.mentions`.

## Done when

- Every case listed for ticket 10 of the decisions has a fixture, including `version 1.2` not reported, an undefined `[t][ref]` not reported, and definitions inside code that are not definitions.
- Each rule leaves `unimplemented_rules`.
