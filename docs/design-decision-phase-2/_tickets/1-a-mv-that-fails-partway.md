# 1: What does a `mv` that fails partway leave behind?

Type: wayfinder:grilling
Status: open
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

<filled in on resolve>
