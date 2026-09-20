# 9: Rules on schemas, collections, keys and file names

Type: implementation
Status: resolved
Blocked by: 8

## What this delivers

- `schema.valid` (duplicate names or codes, `extends` cycles, undeclared overrides, invalid options, field names, import names colliding with URL schemes), `collections.overlap`, `keys.unique`, `filename.pattern`.
- Findings with `path`, `namespace`, `collection` and `key` as the design's finding shape says.

## Done when

- Each rule leaves `unimplemented_rules` and has fixtures in `broken/`; a project with two namespaces that share a key is clean, one with a key twice in a namespace is not.
