# 2: Which locks does `mv` hold when it rewrites refs inside an imported project?

Type: wayfinder:grilling
Status: resolved
Blocked by: None (can start immediately)

## Question

The design says, under Across namespaces: "An imported project is read-only here: no command writes into it, except that `mv` rewrites the refs it can see." Under Commands it says `mv` "Takes the lock of every namespace it writes, in path order."

Those two sentences do not meet. A lock lives at `<project>/.typdoc/locks/<namespace>.lock`, so it belongs to one project. An imported project has its own `.typdoc`, its own namespaces and its own locks. The Concurrency section never says that `mv` takes a lock in the imported project, and the exception above is the one case in v1 where a command writes a file that another project owns.

So a `mv` here can be editing a file in the imported project at the same instant as a `typdoc set` run inside that project, which holds that project's namespace lock and has every reason to believe it is alone. Both write the same file through a temp file and a rename, and the later rename wins whole: one of the two edits disappears with no error on either side. That is the kind of loss that leaves no trace to find afterwards.

Decide:

- Does `mv` take the imported project's namespace locks before rewriting refs there? If yes, in what order relative to this project's locks, so that two `mv`s in two projects that import each other's neighbours cannot deadlock — noting that imports are one-way but two projects can both import a third.
- What happens when the imported project's lock cannot be acquired within the timeout: does the whole `mv` fail with exit 4, or does it proceed and report the refs it did not rewrite as unrewritable, like an unreachable project?
- What happens when the imported project is on a filesystem the user cannot write to, or is a checkout at a commit they do not want touched? Writing into someone else's repository may be the wrong default even when it is possible.
- If `mv` does not lock the imported project, say so explicitly and say what makes that safe, because "no command writes into it, except `mv`" is then a promise with no mechanism behind it.

This is not a question about an unlikely race. It is a question about whether the one documented exception to "imports are read-only" is sound.

## Answer

**Decided: `mv` does not write into an imported project. No write crosses a project boundary at
all.** Whether that changes later is a question for later; in v1 it does not happen.

That removes the question this ticket was opened for rather than answering it. The ticket asked
which locks `mv` must hold when it writes in another project, and the answer is that it never
writes there.

**1. The exception is gone from the design.** The sentence that read "An imported project is
read-only here: no command writes into it, except that `mv` rewrites the refs it can see" now says
that an imported project is read-only with no exception, that no command writes a file in it, `mv`
included, and that `mv` reads those refs only so it can report them. The reverse-lookup table and
the numbered rule that both said `mv` rewrites refs in imported projects say the opposite now, and
`mv`'s own paragraph says it writes no file outside this project.

**2. What follows: those refs cannot be fixed by `mv`, and that is a reported success.** A ref that
lives in a project which imports this one, and points at the document being moved, is now one of the
things `mv` knows it cannot rewrite. That is the category [decision 1](1-a-mv-that-fails-partway.md)
already settled: the run finishes and exits 0, with a report.

**The report names where.** It says which project each unrewritten ref is in, and the refs
themselves — not a total. The user has to go and fix them by hand, and a number tells them that work
exists without telling them where to do it. This is written into the design as part of the case
rather than left to the implementation to decide.

**3. The lock question disappears, and the design says so out loud.** No write means no lock. The
design now states plainly that `mv` takes no lock in another project because it writes nothing
there, and that a lock belongs to the project that owns the file. Leaving that as an absence would
make the next reader derive it, and deriving it is how the exception got written in the first place.

**4. Decision 3 checked against this.** Its rule — locks are taken in the order of the lock files'
own paths — still holds and is still general: a total order over any set of lock files, which does
not depend on where the files come from. Two sentences in it, and one in the design's `Lock order`
bullet, offered a namespace of an imported project as a case the rule handles. That case no longer
arises, so the sentences were rewritten to say what is true now: the rule is a total order over any
set at all, and in v1 the set always comes from one project, because no command takes a lock outside
the project it runs in. Nothing about the decision changed; the examples that claimed something
untrue did.

The same correction was applied to [decision 1](1-a-mv-that-fails-partway.md), whose text listed "an
imported project that cannot be reached" among the things `mv` cannot rewrite. Being unreachable is
no longer what makes a ref unrewritable: being in another project is.
