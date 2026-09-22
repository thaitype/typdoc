# 15: What does a write do when the file it would create already exists?

Type: wayfinder:grilling
Status: resolved
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

**1. All four cases are refused. Nothing is overwritten.** `new <path>` where the path is there,
`new <CODE>` where the template produces a name that is there, `mv` where the destination is there,
and `--renumber` where the destination name is taken: each writes nothing.

The reason is recorded because it is the whole argument: overwriting a user's work with no undo is
the worst thing this tool can do, and the atomic write makes that overwrite perfect and traceless.
There is no partial file to notice and no fragment to recover from. Measured, to show it is not a
theoretical worry: a file holding `IMPORTANT USER WORK` was gone and replaced in full by a plain
rename over it, with no error and no message.

The second case should not be reachable — the number came from `max(highest existing, last) + 1`,
so nobody has used it — and it is checked anyway. An impossible state that nothing checks is how a
number comes to be issued twice.

**2. Refused with exit code 7, a new code.** Not code 1.

The design sets its own test for adding a code: a new code is added only when the caller has to act
differently. This case passes it. A bad argument means go and fix the command. A destination that
exists means choose another name or open the file that is already there. And code 1 says the call
is a defect and is not retried, which is not what happened here: the call was correct in every part,
and what was wrong was the state of the world, not the request.

7 is the next free number and is added to the design's table with that meaning.

If work on decision 9 turns up a reason for a different number, it is raised before anything moves.
It is not changed quietly.

**3. The check is made under the lock, and the file system enforces it.** `new` creates the file
with `O_EXCL` rather than looking first and then writing. A look taken before the lock is only
advice, and it can go stale between the looking and the writing; `O_EXCL` puts the rule in the file
system instead of in typdoc's memory, where it cannot be forgotten by the next person to touch the
code.

## The part of 3 that `O_EXCL` does not reach, and that is not decided here

`O_EXCL` is an argument to opening a file. It covers `new`. It does not cover `mv` or
`mv --renumber`, which put the document in place with a rename, and a plain rename replaces an
existing destination without a word — that is the measurement above.

So two of the four cases need a rename that refuses to replace, not an `O_EXCL`. What exists:

- On Linux, `renameat2` with `RENAME_NOREPLACE`. Run on this machine (kernel 7.0.0-31-generic) it
  returned `EEXIST` and left the existing file untouched, which is the behaviour wanted.
- On macOS the equivalent is `renamex_np` with `RENAME_EXCL`. Not run; macOS is not a supported
  platform.
- Rust's `std::fs::rename` is the plain rename and replaces, so neither is reached through it.

Which call is used on each platform, and how that sits with `typdoc-core` staying free of direct
file system calls, belongs with [decision 7](7-the-write-seam-and-the-clock.md), which decides the
operations of the write seam. The design records that the two kinds of rename are different
operations with different requirements: the one that puts a document where no file was must not
replace, and the one that completes an atomic write of an existing file replaces on purpose. Reading
them as one operation is exactly how a `mv` comes to overwrite a document.

## Where this ticket meets decision 13

[Decision 13](13-writing-the-state-file.md) and this one are the same hazard from two sides. A coded
collection's `match` takes `{key}` exactly once and allows no globs, so a coded document's path is
fixed by its key and, within one namespace, two documents cannot hold the same key. A number issued
twice therefore never appears as a duplicate key. It appears here, as a command about to create a
file that is already there — which is why the check in case two above is not optional, and why the
guard belongs in the file system rather than in a comparison typdoc remembers to make.
