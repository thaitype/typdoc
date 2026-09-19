# 2: How is the next key number derived when `.typdoc/` holds no counter file?

Type: wayfinder:grilling
Status: resolved
Blocked by: None (can start immediately)

## Question

`typdoc new` "allocates the next number in the collection's counter under the lock", but the `.typdoc/` folder listing (config, lock, vendor, write.lock) has nowhere to store a counter.

Decide: derive the number from the highest existing key in the collections sharing the counter, or persist a counter file (committed or ignored). Consider: reuse of a number after the highest-numbered document is deleted (a body link or commit message may still cite it); collections sharing one `counter` across schemas; and the worktree case, where `lock: git-common` can still allocate the same key in two worktrees. Amend `docs/design.md` with whatever is decided.

## Answer

Decided with the human, 2026-09-19, after two reversals (below). Supersedes the first answer, which stored counters in `lock.json`.

**Rule:** `typdoc new` allocates the larger of (a) the highest existing number in the collection and (b) the collection's `last`, plus one, and records it as the new `last`, all under the namespace's write lock.

- **`last` is the never-reuse guarantee.** Delete `T-12` and the highest remaining file is `T-11`, but `last` still says 12, so the next number is 13. Counting files alone would reuse 12, and a body link, commit message or chat message citing it would silently point at a different document. A file created by hand with a higher number is still respected; the two sources can only disagree by leaving a gap, which is harmless.
- **The `counter` option is removed.** Sharing one number sequence across collections was the design's answer to a problem that should not exist: in `chief`, decision-tickets and implementation tickets are one ticket shape distinguished by a field (the `chief-wayfinder` skill: "one shared numbering sequence, one shared file shape, distinguished by the `Type:` field"). They are now one schema in one collection, told apart by `kind`. Each code counts on one sequence of its own.
- **A coded schema serves exactly one collection.** Otherwise two collections naming it would share a sequence by the back door. Two collections naming one coded schema is a config error; so is `last` on a collection whose schema has no code. An uncoded schema may still serve several collections (`notes` and `drafts` using `note.json`).
- **No new file.** `last` lives in the collection's own definition, where its `schema` is named. Because nothing shares a sequence, it is stored exactly once, and two branches that both allocate the next number change the same line, so git reports the conflict; that is louder than the duplicate key alone. The worktree caveat in the design (two worktrees can allocate the same key, `validate` catches it after merge) still stands.
- **`lock.json` goes back to holding only remote-schema pins.** The question of renaming it (its name collides with `write.lock` and the `lock` config key) reverts to what it was before this ticket; not decided here.

**Rejected, and why**
- Deriving from files alone: reuses a deleted number silently.
- A separate counter file, and a `counters` section in `lock.json` (the first answer): the human wanted no new file, and a shared counter made the value live in two places.
- The value in `config.json` as it stands, or in the schema file: a schema may be remote and pinned by hash (writing to it breaks the pin) or shared by several collections; a schema also defines shape and should not carry state.
- A shared `counter` across collections: removed as above. Cheap to add back later as an additive feature; not possible to remove later without breaking users.

**Amended in `docs/design.md`:** the config example and the `collections[]` table (`counter` replaced by `last`), the match-template paragraph (one coded schema per collection), Collection vs schema wording, the `schemas/wayfinder.json` `kind` values, the remote-schema example, `typdoc new`, the Commands intro, Config errors, Concurrency (what is locked; the `git-common` note), and the Wayfinder worked example (one collection, frontier filtered by `kind`). The `.typdoc` folder listing and `lock.json` example are back to their original wording.

**Follow-on, now a ticket:** where the collection definitions live. The human proposed one file per collection with `schema` as a field, so that state sits with the definition; that is [12](12-collection-definition-files.md). Until it is decided, `last` sits in the `collections[]` entry in `config.json`, where the design already defines a collection.

## Not verified

No code exists yet, so nothing was checked by running it. The design was grepped after the edits: no remaining mention of `counter`, `counters` or `IMP`, and `lock.json` appears only where it means remote-schema pins. The example ticket schema keeps its name `wayfinder` and code `WF` although it now covers implementation tickets; the key prefix is a naming choice, not made here.
