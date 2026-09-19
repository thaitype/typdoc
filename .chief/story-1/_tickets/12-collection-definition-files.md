# 12: Do collection definitions move out of `config.json` into one file each, and if so where and in what order?

Type: wayfinder:grilling
Status: open
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

