# 9: The error id and exit code of a write the re-read guard rejects

Type: wayfinder:grilling
Status: resolved
Blocked by: None (can start immediately)

## Question

**Dissolved by [decision 20](20-the-frontmatter-writer.md); see the answer at the end.** The
question below assumes a guard that no longer exists.

## The question as it was first asked

Every write goes through a mandatory guard: after editing, the result is re-read with the reader and compared with the intended change before the temp file is renamed into place; if it does not match, the write is rejected and there is no automatic fallback. The guard is load-bearing, because the writing crate is young.

Nothing says what the user sees when it fires.

- **Which exit code?** The design's table has seven. This is not bad arguments (1): the call was fine. It is not validation failed (2): the document was valid and the change was legal. It is not `--if` false (3), not a lock (4), not missing (5). It is arguably I/O (6), but an I/O failure is described as a problem of the environment that may be retried, and this one will fail again in exactly the same way, so telling a caller to retry is telling it something false. Decide, and if none of the seven fits, decide whether an eighth is added — the rule is that a new code is added only when the caller has to act differently, and "this document cannot be edited by typdoc, edit it by hand" is arguably a different action.
- **Which id?** Every error carries an id in `details[].rule` naming its specific cause. Name this one and say where it is listed.
- **What the message says.** The user's real question is "what do I do now". The honest answer is that typdoc cannot make this edit to this file, and the fallback is to edit by hand. Decide whether the message shows the difference it found, and how much of the file it may print, given that frontmatter can hold values the user would not want in a log.
- **Is it reported for `new`?** `new` writes a whole file rather than editing one, so the guard may have nothing to compare; say whether it runs there and, if so, against what.
- The state file has a guard of its own — `new` "replaces only the number, in place, then re-parses the result and compares it with what it meant to write before renaming". Decide whether that is the same id and code as a document's guard or a different one, and note which it stops: a rejected state write after a document was written is ticket 6's territory.

## Answer

**Nothing is decided, because nothing is left to decide.** The re-read guard is dropped in
[decision 20](20-the-frontmatter-writer.md), so there is no rejection to give an exit code, an
id or a message to.

The reason the question existed is worth keeping. The guard was there because `yaml-edit` was
young and a document's own shape could defeat it: a block scalar or an anchor in the frontmatter
turned an ordinary `set` into a refusal, which is a state of the user's file and something the
user has to be told about and act on. That is what made the exit code and the message worth
deciding.

With `yaml_serde` writing the text the reader already holds, no document's shape defeats the
write. A guard would only catch a round-trip defect in the crate that every read in the program
already trusts without a second check, and such a defect is not one typdoc can fix. That typdoc
assembles the block it meant to is proved by a test at the boundary of typdoc's own code, not by
code that ships and re-reads its own output.

The two sub-questions go with it. Whether `new` runs a guard: there is none to run. Whether the
state file's guard is the same id and code: the state file is written by the same path, from
values typdoc holds, and has no guard either. What a malformed `last` does is a different
question and stays where it is, in [decision 13](13-writing-the-state-file.md).
