# 4: What does an interrupted write leave behind, and may a temp file remain?

Type: wayfinder:grilling
Status: resolved
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

**1. What v1 promises.** v1 promises that the user's documents are not damaged. It does not
promise that no temp file is left behind.

The reason is worth keeping: `SIGKILL` and a power cut cannot be intercepted by anything, so a
promise of no leftover would be false in ordinary circumstances. A promise that ordinary events
make false is worse than no promise, because someone will build on it.

**2. A reserved name shape, and a walker that skips it by rule.** A temp file's name follows a
reserved shape, and the file walker skips a name of that shape by rule, before `match` is
consulted. It is not enough to pick a name that current globs happen to miss:

- A name that starts with `.` is still matched by `match` `*.md`, because `*` matches a leading
  dot in a file name — the design already says so under Which files a run reads, and running it
  agrees.
- A user may write `match` `*`, which matches everything there is.

So the skip cannot depend on the name falling outside a glob; it has to be a rule the walker
applies first.

The name carries a part that cannot repeat: the process id and a random component, so that two
writers working at the same instant cannot choose the same temp name. The shape is long and
specific enough that nobody writes such a name by accident. `docs/design.md` says plainly that a
file whose name has that shape is never a document — that is the property the rule rests on, and it
belongs in the document rather than in the walker alone.

**3. A skipped leftover is a finding, and it has a place in the audit's account.** Reporting it as
a finding is not enough on its own: an audit is an account of what a run met, and a file that is
reported but counted nowhere leaves the account short.

It shares a channel with the entries that are already skipped — a symbolic link, and a name that is
not valid UTF-8 — as one category of things the run did not read. That fixes an existing hole at the
same time. Measured on a small project built for it: with one symbolic link that `match` reached,
`validate --audit --json` reported the link under `findings` as `files.unreadable`, while the audit
accounted for three files (`checked.documents` 2, `no_frontmatter` 1, `uncollected` 0,
`overlapping` 0) out of the four entries `match` reached. The link was met, reported, and counted
nowhere.

In the shape of the output the new category sits beside `unreported` rather than inside it, for the
reason the design already gives for `overlapping`: `unreported` means outside `findings`, and these
entries are inside `findings`. So `audit` gains a list of the entries that were not read, and
`summary` gains its count beside `unreported` and `overlapping`, and the sentence that gives the
account includes it.

**4. The level is `warn`, not `info`.** A warning does not fail CI by itself; it fails only when the
user turns on `--strict`, and turning it on is a way of saying they want to know. Checked by
running, on a project with one `imports.absent` finding: `validate --json` reported it at `warn` and
exited 0, and `validate --strict --json` reported the same finding at `error` and exited 2.

**5. A command that holds a lock may remove leftovers within that lock's scope.** The evidence is
the lock itself: while it is held, no other typdoc is writing in that namespace, so a leftover found
there belongs to a process that is gone.

- The age of the file is not a criterion and is never used. An age threshold is a number chosen with
  nothing behind it, and the design already refuses one for locks.
- A removal that fails must not make the write fail. The user's work matters more than a tidy
  folder.

**6. Permissions after a rename.** A rename replaces the inode, so the mode of the file that ends up
in place comes from the temp file, not from the document that was there. Checked by running: a file
set to `600` was `664` after a temp file was renamed over it, following the umask, with no error and
no message — the inode number changed as expected.

So the mode of the existing file is carried to the temp file before the rename.

`docs/design.md` says exactly what is carried and what is not, and does not say "preserves the
file's permissions", because that would read as a promise that everything is handled:

- The mode is carried.
- The owner and group are not, since changing them needs privilege the program does not have.
- Access control lists and extended attributes are not carried.
- A file that did not exist before has no mode to carry, and gets the default.

## Still open, recorded here rather than left silent

**A temp file and a rename give atomicity, not durability.** Without an `fsync` before the rename, a
power cut can leave the new file in place and empty: the rename is ordered, the data need not be.
Nobody has decided whether typdoc syncs before renaming, what it costs, and whether the containing
directory is synced too. It is not decided here.

**The finding a command produces when it removes a leftover** — what id it carries and at what level
— belongs with decision 9's family of error ids and exit codes and is not answered here.

**The id of the finding for a skipped leftover, and whether it is always-on or configurable.** The
level is decided above. `files.unreadable` is an always-on rule reported at `error`, and one rule has
one level, so a finding at `warn` is a different rule sharing the audit category rather than the same
id. Naming it is an id question and goes with decision 9's family.
