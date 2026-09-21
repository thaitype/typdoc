# 14: How `<project-hash>` is computed for `git-common` locks

Type: wayfinder:grilling
Status: resolved
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

**Decided: hash the project folder's path relative to the root of the worktree.** Bring the path to
one form first — separators `/`, no trailing separator — then SHA-256 it and keep the first 16
hexadecimal characters.

Why that path and not another:

- The absolute path is exactly what differs between two worktrees, which is the case the mode
  exists for, so it cannot be what identifies the project.
- The path relative to the worktree root is the same in every worktree of the repository, which is
  the property the mode needs.
- A worktree that keeps the project at a different relative path is a different project under this
  rule and takes a different lock. That is the intended reading and is written into the design as
  such, rather than left for a reader to work out from the formula.

**Collisions.** Sixteen hexadecimal characters are 64 bits, so two projects in one repository can
hash the same. What follows is that they share a lock and wait for each other; nothing is corrupted
and nothing is lost. Recorded as a known consequence, not guarded against.

**Stability.** The hash is stable across versions of typdoc. If it ever changes, a new binary is
blind to a lock an old binary is holding, so a change is a breaking change and is treated as one.

**`git rev-parse --git-common-dir` may answer with a relative path.** It is canonicalized before a
lock path is built from it.

Two notes for the build, neither of them a decision: `sha2` is already a dependency of
`typdoc-core`, used for schema pins, so this needs no new one; and the worktree root comes from
`git rev-parse --show-toplevel`, which the design does not name yet because it names no git command
but `--git-common-dir`.

Still open, and not part of this decision: what happens when `lock` is `git-common` and the project
is not in a git repository at all, and what happens when the `typdoc/` directory inside the common
directory cannot be created. Both are error-shape questions and belong with decision 9's family.

**Written into `docs/design.md`:** three paragraphs after the lock mode table define
`<project-hash>`, state the collision consequence, and promise stability.
