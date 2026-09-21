# 12: A `mv` that changes only the case of a path

Type: wayfinder:grilling
Status: open
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

<filled in on resolve>
