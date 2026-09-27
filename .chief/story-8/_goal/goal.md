# Goal

When a write times out on a lock (exit 4), the message says what typdoc can really tell about the
lock's owner on every Unix, macOS included, and never calls a lock stale unless the system has
said that no process has the recorded id. The behaviour is `SPC-10` (When a lock is not
acquired); this story makes the binary do what it says.

Today both inputs come from `/proc`, which macOS does not have: every Mac records its host as
`unknown-host`, which matches itself, and every process reads as not running. A lock held by a
running typdoc is reported as "stale, delete <path>", and following that advice lets two typdoc
processes write one namespace.

From a user's point of view, when the story ends:

- On Linux and macOS, a lock held by a running process on this machine says to wait; a lock whose
  process is gone says it is stale and names the file to delete; a lock from another host says it
  cannot be checked from here.
- A lock held by a process of another user reads as running, not stale.
- Where typdoc cannot tell (the hostname cannot be read, the lock records `unknown-host`, the
  system offers no way to ask, or the recorded process id is not one a process can have), the
  message says it cannot be checked and to delete the lock only once that process is known to
  have stopped. It never says "stale" there.
- Taking, releasing and the validity of a lock do not change: the owner check still only chooses
  the wording.
- The changelog, the user docs and the typdoc skill list the endings the message can have.

## Out of Scope

- Anything about how locks are taken, released, ordered or timed out.
- A lock taken over or deleted by typdoc; an age threshold.
- The hostname and liveness on Windows (neither is asked there; every timeout takes the "host
  is not known" ending). Recorded, not built.
- Distinguishing a running typdoc from another process that was given the same id.
- A release or version bump; merging the PR is outside the story.
