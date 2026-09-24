# 12: A `mv` that changes only the case of a path

Type: wayfinder:grilling
Status: resolved
Blocked by: None (can start immediately)

## Question

Paths are compared with their case on every platform, and keys cannot differ only in case; that was settled for story 1. What a `mv` from `Notes/Thing.md` to `notes/thing.md` does was left to this story.

On a case-sensitive filesystem it is an ordinary move. On a case-insensitive one, which is the default on macOS and is possible on Linux, the source and the destination are the same file: a rename may succeed and change nothing, may succeed and change the name's case, or may fail, depending on the filesystem. Meanwhile typdoc, which compares with case, believes it has two distinct paths, so it will rewrite every ref from the old spelling to the new one and then find that the file it renamed is the file it started from.

The bad outcome is not the rename. It is that refs get rewritten to a path that the filesystem may not actually have produced, and a later `validate` on a case-sensitive machine — CI, a colleague, a container — reports every one of them as broken.

Decide:

- Whether `mv` detects that source and destination are the same file rather than comparing the strings, and what it uses to detect it (the file identity the system provides is already used before removing a lock).
- What it then does: refuse with a message, perform the case change through an intermediate name, or perform it and report that the filesystem may not have kept the case.
- Whether typdoc probes the filesystem's case behaviour or refuses to guess. A probe costs a file creation in the project and can be wrong for a different directory on the same machine.
- Whether this decision is made at all for v1, given that macOS is not a supported platform and a case-insensitive Linux filesystem is uncommon. Recording it as a known gap with a clear error is a legitimate answer; letting it corrupt refs quietly is not.
- What the destination that differs only in case means for the "two files with the same key is a validation error" rule and for `filename.pattern`.

## What the prototype found

Run 2026-09-21. Nothing is decided here; the ticket stays open.

**Everything below is what this machine's filesystem does. It is not a general truth.** The results
come from ext4 under `$HOME` and tmpfs at `/tmp`, both case-sensitive. A case-insensitive filesystem
behaves differently at every point, and macOS is case-insensitive by default — and the design does
not claim macOS is supported, so nothing here has been checked on the platform where the question
actually bites.

### What could not be tested here, stated rather than inferred

No case-insensitive filesystem was available. Mounting one needs privileges this user does not have
(`sudo` asks for interactive authentication), and `mkfs.vfat` is not installed. So the
case-insensitive half of this ticket is untested, and the sentences below about it say what follows
from the interfaces, not what was observed.

### On this machine (ext4, case-sensitive)

- `A.md` and `a.md` are two files. After creating `A.md`, `a.md` does not exist; creating it gives a
  second file with a different inode (`2239176` and `2239635` in the run).
- A case-only `mv A.md a.md` is an ordinary rename and behaves as one.
- `rename(x, x)`, the same path for source and destination, succeeds and does nothing. That is what
  POSIX asks for, and it is what happened.

### Where this meets decision 15, which is the part worth carrying forward

[Decision 15](15-a-write-whose-destination-already-exists.md) requires that the rename putting a
document where no file was must refuse to replace an existing file, which on Linux means
`renameat2` with `RENAME_NOREPLACE`. Run here:

| Call | Result |
| --- | --- |
| `RENAME_NOREPLACE` from `s.md` to `s.md` (same path) | `EEXIST` |
| `RENAME_NOREPLACE` to a destination that does not exist | succeeds |
| `RENAME_NOREPLACE` case-only, `U.md` to `u.md`, on this filesystem | succeeds |

The first row is the finding. The no-replace rename refuses when the destination names a file that
is already there — and on a case-insensitive filesystem, `A.md` and `a.md` name the same file. So on
such a filesystem a case-only move is a rename onto an existing destination, and the mechanism
decision 15 chose would refuse it, `EEXIST`, without anything having gone wrong.

That is not observed, for the reason given above; it follows from the first row and from what
case-insensitivity means. It is the specific thing to test on a machine that has such a filesystem.

The third row shows the other half: on a case-sensitive filesystem the same call succeeds, because
the destination genuinely does not exist.

### What this leaves for the decision

The detection question in the ticket — whether `mv` compares the two paths as strings or asks the
filesystem whether they are the same file — now has a concrete consequence attached. Comparing
strings says `A.md` and `a.md` differ everywhere. Comparing file identity, which the design already
does before removing a lock, says they differ here and are the same on a case-insensitive
filesystem. The two answers disagree exactly where the bug would be.

## Answer

**Decided: `mv` asks the file system whether the two paths are the same file, and refuses when they
are. typdoc does not probe what the file system does in general, and v1 goes no further than a
clear refusal.**

### First, the prototype's finding has to be translated

The probe was run while [decision 15](15-a-write-whose-destination-already-exists.md)'s mechanism
was a no-replace rename, and its finding was that such a call refuses a case-only move on a
case-insensitive file system with `EEXIST`, because there the two names are one file.

[Decision 7](7-the-write-seam-and-the-clock.md) has since replaced that mechanism: `mv` checks the
destination under the lock and then renames plainly. The finding survives the translation
unchanged, which is worth saying rather than leaving to be rediscovered. The check asks whether the
destination exists; on a case-insensitive file system `a.md` exists whenever `A.md` does, because
they are the same file; so the check refuses the move. Different mechanism, same outcome, and the
same question to decide.

### Identity, not strings — and the reason is narrower than it looks

`mv` compares the file the system reports, not the text of the two paths.

Strings say `A.md` and `a.md` differ everywhere. Identity says they differ on a case-sensitive file
system and are one file on a case-insensitive one. **The two answers disagree exactly where the
damage is**, which settles it: the comparison that is right in both places is the one to make.

The check already exists in the program, for the release of a lock, and it is the same check
[decision 7](7-the-write-seam-and-the-clock.md) put on the write seam. Nothing new is built.

**This is also the answer to whether typdoc probes the file system's case behaviour: it does not,
and it does not need to.** A probe would cost a file created inside the user's project, it can be
wrong for a different directory on the same machine, and it answers a question nobody asked. The
question is not "is this file system case-insensitive". It is "are these two paths the same file",
and the system answers that directly, for the two paths at hand, with no guess and no leftover.

### What happens then: a refusal, exit 7

A `mv` whose destination is the same file as its source is refused. Nothing is renamed and no ref
is rewritten. It is exit 7, the code
[decision 15](15-a-write-whose-destination-already-exists.md) created for a destination that
already exists, which is what this is: the destination exists, and it happens to be the source.

The bad outcome this ticket was opened about is avoided completely, because it comes from writing.
typdoc believing it has two paths would rewrite every ref from the old spelling to the new one and
then find that the file it renamed is the file it started from; a later `validate` on a
case-sensitive machine — a colleague, a container, CI — would report every one of those refs as
broken. A refusal writes nothing, so there is nothing to be wrong.

**The message has to say what happened, because the path the user typed looks different from the
one on disk.** It says that the file system does not tell the two names apart, that no change was
made, and that typdoc cannot make this change here. A message that only said "the destination
exists" would send the user looking for a file they would not find.

**The same refusal covers `mv x.md x.md`.** POSIX makes that rename succeed and do nothing, so
without the check it would be a command that reports success and changes nothing while claiming to
have moved a document. Refusing is the more honest answer and costs no extra code, since it is the
same comparison.

### Why not carry the move out through an intermediate name

It can be done: rename to a third name, then to the destination. Two things argue against it, and
the second is the one that decides.

It would be a special path through the command with the widest blast radius in v1, for a case that
the supported platform meets rarely.

And it would not be finished at the rename. The file system decides what spelling it keeps, and
typdoc would have to read the directory back to learn which one it got before it could rewrite a
single ref — and then either rewrite them to what it found, which is not what the user asked for,
or undo the move. That is a second mechanism, with its own failure in the middle, to reach an end
the user can reach with the tool their version control already provides for this exact case. The
cost is real and it is stated: `mv`'s reason for existing is that it rewrites refs, and a user who
renames by hand does not get that. On a case-insensitive file system they also do not need it
today, since both spellings open the same file there; they need it on the day the repository is
read somewhere case-sensitive, and that is the day `validate` tells them, by path, which refs to
fix.

### How far v1 goes

This far and no further, which the ticket named as a legitimate answer: a known gap with a clear
error rather than a mechanism.

The proportion is worth stating, because "uncommon" was the reason offered and it is not quite
right. The design supports Linux and says WSL is Linux and works. A repository kept under
`/mnt/c` in WSL is on a case-insensitive file system, and that is an ordinary way to work, not an
exotic one. So the event can be named — someone working in WSL on a Windows drive, changing a
document's name from `Thing.md` to `thing.md` — and that is exactly why the clear refusal is worth
building and the silence is not. It is also why this is not left undecided until macOS is
supported.

What v1 does not claim: that `mv` can change a name's case on such a file system. It says so, in
the command's own words, at the moment the user asks.

### The rules this meets

**Two files with the same key is a validation error.** It is not reached here. On a
case-insensitive file system two files whose names differ only in case cannot both exist, so the
pair the rule is about cannot be created. On a case-sensitive one they are two files and the rule
applies as it always has, unchanged by this decision.

**`filename.pattern`** compares with case, as every path comparison does
([phase 1, decision 22](../../design-decision-phase-1/_tickets/22-case-in-paths.md)). A file whose
name the file system stores in a case the collection's `match` does not expect is reported by that
rule. That is the correct behaviour and needs no exception: the rule is telling the user the truth
about what is on disk.

### Written into `docs/design.md`

`mv`'s own paragraph gains a sentence: a destination that names the same file as the source — the
same path, or a name differing only in case on a file system that does not tell them apart — is
refused at exit 7 with nothing written, and the message says that the file system does not
distinguish the two names. The Concurrency section needs nothing: the identity check it already
describes is the one used here.
