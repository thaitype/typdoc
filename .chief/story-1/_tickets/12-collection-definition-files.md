# 12: Do collection definitions move out of `config.json` into one file each, and if so where and in what order?

Type: wayfinder:grilling
Status: resolved
Blocked by: None (can start immediately)

## Question

Ticket 2 put the number allocator's state, `last`, in the collection's own definition. Today a collection is an entry in the `collections[]` array of `.typdoc/config.json`, next to human-authored settings. The human proposed instead that each collection be its own JSON file, with `schema` as one of its fields, so that state lives with the definition it describes. Benefits argued so far: adding a collection means adding a file rather than editing a shared config; a merge conflict is confined to one collection; `validation.collections.<name>` can sit beside the collection it tunes.

Decide:

1. Split, or keep `collections[]` in `config.json` with `last` written there.
2. If split: the folder (for example `.typdoc/collections/<name>.json`), what stays in `config.json` (`version`, `name`, `imports`, `validation.global`, `lock`), whether `validation.collections.<name>` moves into the collection file, and whether each collection file carries its own `version`.
3. Ordering: the design calls `collections` an "ordered list". Establish whether order affects anything (overlapping `match`, default `list` order, error reporting). If it does, say how a set of files is ordered; if it does not, drop the word.
4. Discovery and failure: what `typdoc` does when a collection file is missing, unparsable or names an existing collection; how nested namespaces and the "nearest `.typdoc/config.json`" rule interact.
5. Writing: `typdoc new` must rewrite one number in a hand-authored JSON file without reformatting it. State the guard (for example reparse and compare before rename), consistent with ticket 1.

Amend `docs/design.md` (Config section, examples, Discovery, Concurrency).

## Answer

Decided with the human, 2026-09-19.

**Split: one file per collection at `.typdoc/collections/<name>.json`.** It holds `match`, `schema`, optional `refBase`, optional `validation` for that collection, and `last`. `config.json` keeps `version`, `name`, `imports`, `validation.global` and `lock`. Placement was settled by the design itself: a namespace is the folder that holds the documents, and `.typdoc/` sits at its top, so the collection files are beside the markdown already. Putting a definition inside a documents subfolder was rejected because a collection is not one folder: `match` is a template or glob, and several collections may share a folder.

Defaults, all accepted by the human:

1. **Name** is the file name without `.json`: ASCII letters, digits, `-` and `_`. Unique by construction.
2. **One `version`**, in `config.json` only. It covers every format typdoc owns: `config.json`, the collection files, `lock.json` (which loses its own `version`) and the schema format. Pinned copies of remote schemas follow the publisher's version in their URL.
3. **`validation`** for one collection moves into its file; `validation.global` stays in `config.json`. Merge order is unchanged (defaults, then global, then the collection). The config error "`validation.collections` names a collection that does not exist" disappears, since a collection that has no file cannot be named.
4. **No order.** "Ordered list" is dropped; nothing in the design used it (`list` sorts by key or path). Anything that lists collections sorts by name. A file matched by two collections is an always-on validation error, `collections.overlap`, never settled by precedence.
5. **Loading.** Every `*.json` in `.typdoc/collections/` is a collection; other files are ignored. An unparsable file, an unknown key or a missing schema is a config error naming the file, and `typdoc` stops rather than skipping it, since a skipped collection silently shrinks every result.
6. **Writing `last`.** `typdoc new` replaces only the number, in place, then re-parses and compares before renaming the temp file over the original. The guard is the same one ticket 1 recommends for frontmatter.
7. **Nested namespaces** are unchanged: the nearer `.typdoc/config.json` owns its files, and each namespace has its own `.typdoc/collections/`.

**Costs accepted:** a broken collection file stops every command in its namespace (as a broken config does today); rules for a namespace are no longer visible in one place; the format of a collection file cannot advance its version independently of the config.

**Amended in `docs/design.md`:** the Model table, Collection vs schema, the whole Config section (config example and table, new Collection files section with loading and writing), Discovery, the `.typdoc` folder listing, the `lock.json` example (no `version`), the remote-schema example, Validation rules (config shape, merge order, new `collections.overlap`), Config errors, and the memory-namespace worked example. Grepped afterwards: no remaining `collections[]`, `validation.collections` or "Ordered".

## Not verified

No code exists yet. The design has not been re-read end to end after these edits; only grepped. Whether `lock.json` without a `version` is comfortable for the pinned-copy checksum check was not examined beyond the design text.

