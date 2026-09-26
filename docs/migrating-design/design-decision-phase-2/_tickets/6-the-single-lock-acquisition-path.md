# 6: The single lock-acquisition path, and an interrupt that arrives while working

Type: wayfinder:grilling
Status: resolved
Blocked by: 3, 4, 5

## Question

Story 1's test strategy tests signals by holding a lock in a shipped binary and sending it a real signal, with no test-only code in the binary. That proves something about every command only if every command reaches a lock through one piece of code. The design never says that it does; it was written down as something the contract still has to settle.

Three questions sit together here.

**One path.** Is "every command takes and releases its locks through one acquisition path" a rule of the code, checked by something rather than remembered? If it is, what checks it — a private constructor, a type that only that path can make, a lint? A rule that depends on the next person remembering is not a rule.

**The gap at the start.** There is an instant between creating the lock file with `O_EXCL` and registering it for removal on a signal. A signal in that instant leaves a lock file with no owner, and the design forbids typdoc from ever taking over or deleting a lock another process created — so the leaked lock blocks that namespace until a person removes it by hand. Decide how the window is closed, or, if it cannot be closed, state the residual window plainly and say why it is acceptable.

**An interrupt while working, not waiting.** The design's wording ("removes its own lock ... on the system's interrupt signals") reads naturally for a process that is waiting. A signal that arrives in the middle of the work has more cases: between the temp file and the rename of one document; between the renames of two documents in one `mv` (ticket 1); after the document is written and before `last` is recorded in the state file, which would issue the same number twice on the next run. Decide what each of those leaves behind, and which of them the design has to state rather than leave to the implementation.

Also decide whether the cleanup runs from the handler or from a flag the handler sets, given that the identity check before removal is not async-signal-safe, and what happens if a second signal arrives while the cleanup is running.

## Answer

**Decided: `signal-hook`, one acquisition path proved by a type, the handler does nothing but
wake a thread, and the window at the start of a lock stays open and is written down.**

### The crate

`signal-hook`, at an ordinary version requirement rather than an exact pin.
[Decision 5](5-signal-handling-crates.md) gathered the facts and left the choice here. It is the
only candidate that covers all three requirements in one dependency: ending by the signal
(`low_level::emulate_default_handler`), running the cleanup off the handler (`Signals`, where the
handler writes a byte and an ordinary thread does the work), and registering before any lock file
exists. `nix` is the runner-up and would cost two calls where this costs one; `ctrlc` is out,
because it cannot re-raise at any level and would mean adding `nix` or `libc` anyway for the one
behaviour that decides the question.

The exact pin is not carried over from the writer's story. That pin existed because a crate was
being chosen despite being two days old; this one is mature and widely depended on, and pinning it
exactly would mean a manual step for every patch release with nothing bought.

The standard library offers no way to intercept either signal, so the dependency buys the chance
to run before the default disposition, and nothing else. The Windows build failure the design
promises still comes from the program's own `compile_error!`, not from the dependency, since both
`signal-hook` and `ctrlc` do compile there.

### One acquisition path, proved rather than remembered

Story 1's test strategy sends a real signal to a shipped binary holding a lock. That proves
something about every command only if every command reaches a lock through the same code, and a
rule that lives in someone's memory is not a rule.

**Two things already in place hold the floor under it.** `typdoc-core`'s `clippy.toml` bans
`File::create_new` along with the rest of the writing half of `std::fs`, so a lock file cannot be
created anywhere but behind the write seam. And the lock's release is tied to the value's own
end of life, so a command cannot hold a lock past the scope that acquired it.

**What the test then proves.** With one path, the signal test in the shipped binary is a test of
every command, which is what story 1's strategy was counting on and what this decision supplies.

### The window at the start, which stays open

**Registration comes first.** The handler is registered before any lock file is created, at the
start of the run, not when the first lock is wanted. Registration needs no lock-related state, so
nothing forces the other order. That closes the large window the design was worried about — the
one that would exist if a run created lock files for a while and only then arranged to clean them
up.

**What is left is a few instructions wide and cannot be closed.** The kernel creates the lock file
inside the `O_EXCL` call; the process records that it holds it when the call returns. A signal
delivered between those two moments finds a lock file on disk that no list in the process yet
names. The cleanup cannot remove it, and must not guess: the design forbids typdoc from removing a
lock it did not create, and in that instant it has no evidence the file is its own. Recording the
intent before the call does not help, because then the evidence is missing in the other direction
— the file might equally be another process's, and removing it would be exactly the takeover the
design rules out.

**What it leaves, and why that is acceptable.** A lock file with no owner, in one namespace. The
next run that wants that namespace waits, times out, and exits 4 — and the message the design
already specifies gives the path, the pid, the host and the age, and says the owner is no longer
running on this machine and shows the path to delete. So the outcome is a stop with an accurate
explanation and a one-line fix, not a wrong answer and not a damaged file. Nothing the user owns
is lost, and nothing is written. Closing the window would need typdoc to remove a lock on evidence
it does not have, which trades a stop that explains itself for a class of bug where two processes
believe they hold the same lock.

The window is stated in the design rather than left to be found.

### An interrupt that arrives while working

Every case below ends the same way: the locks the process holds are removed, and the process then
ends by the signal. What differs is what the work leaves behind, and every one of them was already
decided — this ticket confirms that the signal path adds no new case rather than inventing
answers.

| Where the signal lands | What is left | Settled by |
| --- | --- | --- |
| Waiting for a lock | Nothing; no lock is held | — |
| Between writing a temp file and renaming it | The temp file; the document is untouched and whole | [decision 4](4-what-an-interrupted-write-leaves-behind.md) |
| Between two renames of one `mv` | Some refs updated, some not; the document has not moved, because it moves last | [decision 1](1-a-mv-that-fails-partway.md) |
| After the document has moved, before every ref is rewritten | The move is done and the run says so; `validate` reports the refs until the command is run again | [decision 1](1-a-mv-that-fails-partway.md) |
| In `new`, after the file exists and before `last` is written | Nothing wrong. The next number is the larger of the highest existing number and `last`, plus one, and the new file is now the highest existing one, so the number cannot be issued twice | the design's own allocation rule |
| In `mv --renumber`, between the two state files | The destination's `last` is written before the document appears under its new key, so the worst case is a number recorded and not used, which is an ordinary skip | [decision 1](1-a-mv-that-fails-partway.md) |

The case the ticket named as the dangerous one — a document written and its number not recorded —
does not arise, and not because the signal path is careful. It cannot arise because allocation
takes the larger of the two sources, so a file on disk counts even when the state file has not
caught up. That is worth saying plainly, because the same question will be asked again.

### The cleanup runs off the handler, and runs once

**The handler does one thing: it wakes a thread.** Comparing file identity means a `stat` and an
`fstat`, and removing a lock means an `unlink`; none of that is async-signal-safe, so none of it
belongs in a handler. `signal-hook`'s `Signals` gives exactly this shape — the handler writes a
byte, an ordinary thread reads it and does the work — and the work is the same identity check the
design already specifies before any release: compare the path's identity with the open file's, and
if they differ, or the open file's link count is zero, remove nothing and report that the lock was
taken away during the operation.

**Then the process ends by the signal**, through `emulate_default_handler`, not through an exit
code. A run stopped by `SIGINT` has to look stopped to whatever started it; giving it one of the
seven exit codes would tell a caller that typdoc decided something, when what happened is that it
was interrupted.

**A second signal while the cleanup is running does not cut it short.** The cleanup runs once, and
further deliveries of the same signal are absorbed until it has finished, after which the process
re-raises. The cleanup is bounded and small — an identity check and an unlink for each lock held,
and a command holds a handful at most — so the wait is measured in system calls, while what it
prevents is a lock left behind that blocks a namespace until a person deletes a file. `SIGKILL`
remains what it always is: immediate, uncatchable, and the way out if anything here is ever wrong.

### Written into `docs/design.md`

Concurrency's **Releasing** bullet says typdoc removes its own lock on the interrupt signals. It
gains what that means in practice: the cleanup does not run in the handler, the process then ends
by the signal rather than with an exit code, a second signal does not cut the cleanup short, and
the handler is registered before the first lock file is created. It also gains the residual
window at acquisition and what that window leaves, so that a stale lock with no owner is a
documented outcome with a known fix rather than a mystery.
