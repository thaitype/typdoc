# 14: Two writers that take the same lock

Type: implementation
Status: open
Blocked by: 3, 9

The goal's second criterion. Running several processes one after another and finding no duplicate
key proves nothing: it is the result an implementation with no lock at all gives whenever the runs
happen not to overlap. The collision is made to happen.

## What this delivers

- A deterministic test through the seam: two allocations on two threads in one process against one fake file system, with the fake blocking the first writer at the instant between reading `last` and writing it back, and releasing it once the second has reached the same point.
- A test across real processes: the shipped binary of ticket 4 holds the lock while `n` processes running `new` are started with a `--lock-timeout` long enough that none gives up, and the holder is released only once all `n` are waiting.
- A test of what the design actually claims for `git-common`: two worktrees can allocate the same number, and `validate` reports the duplicate after the merge.

## Done when

- Both tests are shown red against a build whose allocation reads and writes `last` outside the lock, and that demonstration is part of the story's evidence.
- The real-process run gives `n` distinct keys, `last` equal to the highest of them, `n` documents, and no document written over another.
- Neither test depends on timing to pass; the deterministic one cannot pass by being lucky.
