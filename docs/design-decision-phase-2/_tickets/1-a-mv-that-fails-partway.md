# 1: What does a `mv` that fails partway leave behind?

Type: wayfinder:grilling
Status: resolved
Blocked by: None (can start immediately)

## Question

`typdoc mv <from> <to>` renames one file and rewrites every ref to it that is visible from this project, in frontmatter and in body links, in every namespace of this project and in the projects it imports. One invocation therefore writes many files. The design says each file is written to a temp file in the same directory and renamed over the original, "so a reader sees the old file or the new one whole". That promise is about one file. It says nothing about the set.

If the command writes twelve files and the disk fills, or a permission is refused, or the process is killed at the eighth, the user is left with a repository in which some refs point at the old path and some at the new one, and nothing records which. The document itself may or may not have been renamed yet. A repository in that state validates as broken in a way that looks like the user's own mistake.

Decide what `mv` promises here:

- Is `mv` all or nothing over the whole set of files it writes, or is a partial result allowed as long as it is reported?
- If it is all or nothing, by what mechanism: write every temp file first and rename them all only after the last one is prepared (which narrows the window to the renames but does not close it), or a journal that a later run can finish or undo, or something else? Name the residual window that the chosen mechanism still leaves, because every mechanism leaves one.
- If a partial result is allowed, what does the command print and what is its exit code, and how does a user find the files that were already changed? Note that the design's exit table has no code for "did some of it".
- `mv --renumber` moves a coded document to another namespace and writes in both. Is the same answer true across two namespaces, or is that a separate promise?
- Does the order of the writes matter — for instance, renaming the file last so that a failure leaves every ref still pointing at a file that is there, rather than first so that a failure leaves refs pointing at nothing?
- What does `mv` do about the files it already knows it cannot rewrite (unreachable projects, plain-text mentions, body links when `body.links` is `off`)? The design says it reports them. Is a run that has such reports still a success?

## Answer

**1. `mv` does not promise to be all or nothing, because it cannot be.** A file system gives
atomicity one rename at a time, and `mv` writes many files. A promise it cannot keep is not
written.

The mechanism is: prepare a temp file for every file that will change, all of them, and then do
the renames in one run at the end. That moves the exposure from the whole of the work down to the
run of renames.

**The window that remains is the run of renames, and the design says so plainly.** It is not
closed, and it is not described as closed. A failure or an interrupt inside it leaves some files
renamed and the rest not.

**2. The document itself moves last, always.** The reason is recovery, not a smaller mess.

- Move the document first and stop: the same command cannot be run again, because the source it
  names is gone. The user is left to work out a different command from a half-finished state.
- Move it last and stop: the document is still where it was, so the same command run again
  finishes the job. The refs already rewritten name the new path, so the command does not find
  them when it looks for refs to the old path and leaves them as they are; the rest are rewritten.

So the command is its own way back, and no journal is needed for it.

**The obligation that comes with it: the failure message says the same command can be run again.**
Not only that it failed. A recovery path that the user has to deduce is not a recovery path. The
design also records that the project is inconsistent until the re-run, and that `validate` reports
it — the refs already rewritten name a path that is not there yet — so nobody reads those findings
as a second fault.

**3. What `mv` knows it cannot rewrite does not make the run a failure.** Plain-text mentions, body
links while `body.links` is `off`, and refs held by any other project: the run finishes, exits 0,
and reports each one in detail. ([Decision 2](2-locks-when-mv-writes-into-an-imported-project.md)
later widened the third of these: `mv` writes in no other project at all, so it is not only the
projects it cannot reach, and the report has to name the project each unrewritten ref is in.)

What `mv` promises is the refs typdoc tracks. Plain text was never one of them. An import that is
absent on this machine is ordinary by design, so failing on it would make `mv` fail routinely for
anyone who does not have that project checked out — which is most people, most of the time.

A project that needs certainty here already has the mechanism, and the design already writes it:
set `imports.absent` to `error`, and a run that cannot see an import is an error before `mv` is
reached. Pointing at what exists is better than adding a second way to say the same thing.

**4. `mv --renumber` carries the same promise, plus one ordering rule.** The destination
namespace's `last` is written before the document appears under its new key.

That does not contradict the rule that the document moves last: that rule orders the document
against the refs, and this one orders the document against the state file. They sit beside each
other.

**Skipped numbers are ordinary, and the design now says so.** A number recorded and then not used
is skipped. A collection that runs `WF-3` and then `WF-5` is not missing a document. Without that
sentence a user goes looking for a document that never existed. The alternative — letting a
document exist under a number the state file has not recorded — is exactly what issues that number
a second time.

## Noted while working through this: where a double-issued number actually surfaces

A coded collection cannot have a glob in its `match`: the design's placeholder table allows `{key}`
in a coded template "exactly once, no globs". So a coded document's path is determined by its key,
and within one namespace two documents cannot hold the same key, because they would be the same
path.

The danger of issuing a number twice therefore does not appear as a duplicate key. It appears as a
command about to create a file that is already there, which is [decision 15](15-a-write-whose-destination-already-exists.md).
That is the place to guard it, and the guard `new` should use — creating with `O_EXCL` so the file
system refuses rather than typdoc checking first — is on that ticket.

One boundary is worth keeping straight, because the repository holds a fixture that looks like a
counter-example. `keys.unique` is a real rule with a real fixture, and the fixture reaches it with
two collections, `alpha/{key}.md` and `beta/{key}.md`, whose schemas share a code. That fixture
trips `schema.valid` as well, because two schemas with the same code is itself a schema error. So a
duplicate key is reachable only where the configuration is already broken and already reported; in
a valid configuration, within one namespace, it is structurally impossible. Across namespaces the
same key can exist legitimately, which is why a coded document cannot move between them except by
`--renumber`.

**Open, raised by [decision 7](7-the-write-seam-and-the-clock.md).** The move that must not
replace an existing file is a hard link followed by removing the source, because `link` is the
only call in the standard library that takes a name atomically or fails, while `rename` replaces
in silence. Between those two steps both names exist and are the same file. A re-run then meets a
destination that is already there, which this decision wants to finish the work and
[decision 15](15-a-write-whose-destination-already-exists.md) wants to refuse at exit 7. They can
be told apart: a file and a link to it are the same file, and the system says so, by the identity
check that already runs before a lock is removed. The rule that follows is that a destination
which is the same file as the source means the move already happened, so the command removes the
source and reports success, while a destination that is a different file is the refusal. Not yet
decided, and both this decision's re-run story and decision 15's rule change if it is taken.
