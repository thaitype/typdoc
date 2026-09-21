# 14: How `<project-hash>` is computed for `git-common` locks

Type: wayfinder:grilling
Status: open
Blocked by: None (can start immediately)

## Question

With `lock` set to `git-common`, a lock lives at `$(git rev-parse --git-common-dir)/typdoc/<project-hash>-<namespace>.lock`, and the project lock at `<project-hash>.lock`. The design never says what `<project-hash>` is a hash of, or with what.

It has to be decided before anything is written, because the whole point of the mode is that two worktrees of the same repository agree on the same path. If two worktrees compute different hashes for the same project, both take "the" lock and neither sees the other, which is worse than no locking: the mode exists to prevent exactly that, so a user who turned it on believes they are protected.

Decide:

- What is hashed. The absolute path of the project folder differs between worktrees, so it cannot be that. The path of the project folder relative to the worktree root is the same in both — unless one worktree has the project at a different relative path, which is possible. The common directory itself is shared but does not distinguish two projects inside one repository, which is why the namespace is a separate part of the name.
- Which hash and how much of it is used, and what happens on a collision: two projects sharing a lock is a stall, not a corruption, but it should be a known consequence rather than a surprise.
- Whether the hash is stable across versions of typdoc. If it changes, an upgrade makes a running lock invisible to the new binary. If it is promised stable, that promise has to be recorded.
- What `git rev-parse --git-common-dir` returning a relative path means here, since it does, and relative to what.
- What happens when `lock` is `git-common` and the project is not in a git repository at all: an error with an id, or a fallback to `local`. A silent fallback gives a user the mode they did not ask for and believe they have.
- Whether the `typdoc/` directory inside the common dir is created by typdoc, and what happens when it cannot be.

## Answer

<filled in on resolve>
