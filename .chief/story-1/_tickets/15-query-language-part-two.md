# 15: The query language, conditions through refs

Type: implementation
Status: open
Blocked by: 12, 14

## What this delivers

- `ref.all`, `ref.any`, `ref.none`, `refby.*` and `$body`, counting from the value written in the field so dangling refs are included, and the errors (`ref.all` without an expression).

## Done when

- Pairs of complementary queries over a set that has a dangling ref; the scope of the condition after `ref.*(f)` follows `f`'s `target`.
