# 6: The single lock-acquisition path, and an interrupt that arrives while working

Type: wayfinder:grilling
Status: open
Blocked by: 3, 4, 5

## Question

Story 1's test strategy tests signals by holding a lock in a shipped binary and sending it a real signal, with no test-only code in the binary. That proves something about every command only if every command reaches a lock through one piece of code. The design never says that it does; it was written down as something the contract still has to settle.

Three questions sit together here.

**One path.** Is "every command takes and releases its locks through one acquisition path" a rule of the code, checked by something rather than remembered? If it is, what checks it — a private constructor, a type that only that path can make, a lint? A rule that depends on the next person remembering is not a rule.

**The gap at the start.** There is an instant between creating the lock file with `O_EXCL` and registering it for removal on a signal. A signal in that instant leaves a lock file with no owner, and the design forbids typdoc from ever taking over or deleting a lock another process created — so the leaked lock blocks that namespace until a person removes it by hand. Decide how the window is closed, or, if it cannot be closed, state the residual window plainly and say why it is acceptable.

**An interrupt while working, not waiting.** The design's wording ("removes its own lock ... on the system's interrupt signals") reads naturally for a process that is waiting. A signal that arrives in the middle of the work has more cases: between the temp file and the rename of one document; between the renames of two documents in one `mv` (ticket 1); after the document is written and before `last` is recorded in the state file, which would issue the same number twice on the next run. Decide what each of those leaves behind, and which of them the design has to state rather than leave to the implementation.

Also decide whether the cleanup runs from the handler or from a flag the handler sets, given that the identity check before removal is not async-signal-safe, and what happens if a second signal arrives while the cleanup is running.

## Answer

<filled in on resolve>
