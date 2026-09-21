# 2: Which locks does `mv` hold when it rewrites refs inside an imported project?

Type: wayfinder:grilling
Status: open
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

<filled in on resolve>
