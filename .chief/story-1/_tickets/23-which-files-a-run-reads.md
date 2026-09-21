# 23: Build what the walk was decided to do

Type: implementation
Status: open
Blocked by: None (can start immediately)

## What this delivers

The paragraph **Which files a run reads** in `docs/design.md` is decided and written; the walk does not do it yet. This ticket makes the two agree.

- A glob does not enter a folder whose name begins with `.`, and a literal segment does. The first half is what the walk already does for namespaces; the second is new for both.
- A `*` matches a leading dot in a file name.
- A symbolic link to a folder is not followed.
- `.gitignore` is not read, which is already true and only needs a test that says so.
- A directory entry a template reaches that is a symbolic link, or whose name is not valid UTF-8, is skipped and reported under `files.unreadable`, instead of stopping the run with exit 6. `files.unreadable` is in the design's always-on table and in `UNIMPLEMENTED_RULES`.

## Done when

- Each of the five has a test that fails before the change: a dot folder reached by a glob and by a literal segment, a dotted file name under `*`, a folder symlink, and an entry that cannot be read.
- `files.unreadable` leaves `UNIMPLEMENTED_RULES` and has a fixture under `broken/` with its exact expected set.
- A run that meets an entry it cannot read still answers about every other file, and the exit code it used to stop with is either produced elsewhere by a test or listed in `UNPRODUCED_EXIT_CODES`.
- The accounting invariant still holds on every fixture project, since what the walk reads is what the invariant counts against.

## Why it is its own ticket

The acceptance runs need this built, not merely decided: the repositories they run against keep most of their documents in folders whose names begin with a dot, so the numbers those runs produce depend on it.
