# 15: What does a write do when the file it would create already exists?

Type: wayfinder:grilling
Status: open
Blocked by: None (can start immediately)

## Question

The design describes `new` as: allocate the number, name the file from the collection's `match` template, fill defaults and `auto` fields, validate, then write. For a path-identified schema the user gives the path themselves: "A document without a code is named by whoever creates it (`typdoc new <path>`); typdoc only checks the path against `match`."

It never says what happens when that file is already there. `typdoc new notes/meeting.md` on an existing `notes/meeting.md` would, read literally, write the file — and the atomic write makes it a clean, complete overwrite of something the user spent time on, with no undo.

The same gap is on the other side of `mv`: "`mv` reads both its arguments in this way, and its second may name a file that does not exist yet." *May* name a file that does not exist yet is not the same as *must*, and nothing says what happens when it does exist.

Decide, for each of the four cases:

- `new <path>` where the path exists. Refuse with which exit code and which id — 5 is "not found", so this needs its own answer, and the natural one is bad arguments (1), which then has to be told apart from a malformed path.
- `new <CODE> "<title>"` where the file the `match` template produces exists. The number came from `max(highest existing, last) + 1`, so this should be impossible; decide what happens when it occurs anyway, because "impossible" states that are not checked are how a number gets issued twice. Note the design already accepts a hand-made file with a higher number, so the file-name space is not typdoc's alone.
- `mv <from> <to>` where `<to>` exists. Refusing is the obvious answer; say what tells a user the two are distinct files rather than the same one in different case, which is ticket 12.
- `mv --renumber` where the destination's `match` template produces an existing name — the same shape as the second case, in the destination namespace.

Also decide whether the check is made before or under the lock. Before the lock it is advice that can go stale between the check and the write; under the lock it is a guarantee, and the create can use `O_EXCL` so that the filesystem enforces it rather than typdoc.

## Answer

<filled in on resolve>
