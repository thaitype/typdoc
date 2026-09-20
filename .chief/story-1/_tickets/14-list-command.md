# 14: `typdoc list`

Type: implementation
Status: resolved
Blocked by: 3, 13

## What this delivers

- `--collection`, `--code`, `--where`, `--fields`, `--sort` (by the types in the design), `--limit` and `--ids`; `total` and `truncated`; the default text table; the scope by namespace.
- A golden for `list`.

## Done when

- `truncated` is true exactly when `total` exceeds the number listed (a test asserts it); a value in the output does not change with `--limit`.
- `list` leaves `unimplemented_commands`.
