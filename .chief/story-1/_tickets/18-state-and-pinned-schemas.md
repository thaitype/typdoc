# 18: Reading the state file and pinned copies of remote schemas

Type: implementation
Status: open
Blocked by: 9, 17

## What this delivers

- The state file read for `state.missing`, `config.state-orphan` and `config.state-uncoded`.
- Pinned copies read from `vendor/schemas/<sha256>` and hashed to check the name, `config.vendor-missing`, `config.vendor-edited`, `config.schema-unpinned`, and the drift check on qualified `target` names under `schema.valid`.

## Done when

- `state.missing` has its three fixtures (no file with documents, a file without the entry, a new collection reporting nothing); the pinned copies used are made by hand; nothing is fetched or written.
