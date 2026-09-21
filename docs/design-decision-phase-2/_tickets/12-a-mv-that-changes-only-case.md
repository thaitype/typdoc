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

## Answer

<filled in on resolve>
