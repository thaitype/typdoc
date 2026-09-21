# 4: What does an interrupted write leave behind, and may a temp file remain?

Type: wayfinder:grilling
Status: open
Blocked by: None (can start immediately)

## Question

The design promises that each file typdoc writes is written to a temp file in the same directory and renamed over the original, so a reader sees the old file or the new one whole. It does not say what happens to the temp file when the process does not reach the rename: killed, out of disk, or interrupted.

The temp file is in the same directory as the document, which is a directory a collection's `match` reaches. So a leftover temp file is not only litter:

- If its name ends in `.md`, a `match` glob may claim it, and it then appears in `list`, is checked by `validate`, and counts in the audit. A user sees a document they never created and cannot explain.
- If it is in a coded collection's folder and fits no `match` template, `filename.pattern` reports it as an error, which is a finding about a file the user did not write.
- If two runs are interrupted, two leftovers accumulate, and nothing removes them.

Decide:

- The naming and placement of a temp file, so that a leftover is recognisable as typdoc's, cannot be matched as a document, and cannot collide with a second concurrent writer's. Note the constraint that pulls the other way: the rename must be on the same filesystem, so a temp directory elsewhere is not available.
- Whether v1 promises that no temp file remains after an interrupt, or promises only that no *document* is damaged and accepts that a leftover can exist.
- If a leftover can exist, what notices it. An always-on rule that reports it is one answer; saying nothing is another, and it means a user's first encounter with it is confusion.
- Whether a run removes leftovers it finds from earlier runs, and if so on what evidence — a file that another live typdoc process is writing at that moment must not be removed, which is the same hazard as removing a lock that is in use.
- What the temp file's permissions are, and whether the renamed file keeps the original document's permissions. A rename replaces the inode, so an atomic write silently rewrites the mode and ownership unless something carries them over.

## Answer

<filled in on resolve>
