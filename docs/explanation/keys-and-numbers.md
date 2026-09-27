# Keys, numbers, and why they're never reused

A numbered document, `WF-12`, looks simple: a code and a number. Most of what's interesting about
typdoc's concurrency and its state file comes from one promise about that number: once it has
been handed out, it's never handed out again.

## Why the promise matters

Refs point at keys. A ticket that says `blocked_by: [WF-12]` means one particular ticket.

Suppose `WF-12` is deleted and a later `typdoc new` hands out 12 again to an unrelated ticket. The
old ref now resolves, and nothing looks wrong: the key exists, the target is a ticket, `validate`
is happy. The ticket is simply blocked by the wrong thing, and nobody is told. Compare that with a
ref to a key that no longer exists, which `validate` reports straight away and which tells you
exactly where to look.

So typdoc would rather leave a gap in the numbers than reuse one. Gaps are ordinary: they're what a
deleted document leaves behind.

## The key identifies, the slug is for readers

A numbered document's file name can carry a slug after its key: `tickets/WF-12-lock-order.md`.
A bare `WF-12.md` tells someone reading a folder listing nothing about what's inside, and the slug
is there for them. It isn't part of the document's name. The document is `WF-12`: refs,
arguments, the output of `list` and `refs`, and `key` in `--json` all use the key alone, and the
slug shows up only in `path`.

The reason is the same promise as above. A slug describes the document, and descriptions go out of
date. If the slug were part of the name, rewording it would turn the document into a different
one, and every ref to it would break. Because the key is the name, a slug can be fixed at any time
with `typdoc mv`, a move to the same key with another slug, and every ref by the key alone stays
exactly as it was.

A ref can be written with the slug too, `blocked_by: [WF-12-lock-order]`, for whoever reads the
frontmatter. It resolves by the key, so it still points at the right document after the slug
changes. `validate` reports that the written slug is stale (`refs.slug`, a warning by default),
and `mv` rewrites such a ref to the new slug when it changes the slug itself.

Slugs play no part in numbering. `WF-12-lock-order.md` counts as number 12, exactly like
`WF-12.md`, and `typdoc new` picks the next number from `last` and the highest key without
looking at any slug. `typdoc mv --renumber` gives a document a new key and keeps its slug.

Since the key is what counts, two files with one key are two documents claiming one name, whatever
their slugs: `WF-5.md` and `WF-5-draft.md` in the same namespace are both `WF-5`, and `validate`
reports them as `keys.unique`. So are `WF-5-a.md` and `WF-5-A.md`, which are two files on Linux
and one on the default file system of macOS: `keys.unique` reports them before the difference in
case can matter.

## The state file

The number that matters isn't the highest key on disk. It's the highest key ever issued. Those
differ whenever something has been deleted, so typdoc records the second one:

```json
{
  "tickets": {
    "last": 12
  }
}
```

That's `.typdoc/state/<namespace>.json`. `typdoc new` reads it, picks one more than the larger of
`last` and the highest existing key, creates the file, and writes the new `last`. It never lowers
`last`, even when the document holding the highest number is gone, because lowering it is exactly
how 12 would come back.

A few consequences follow.

Commit the state file. It's the only record of numbers that were issued and later deleted.

On a merge conflict, take the higher number. Two branches that both created tickets will
conflict on `last`. The higher side includes every number either branch used. Taking the lower one
lets a number be issued twice.

Never revert it. Rolling the file back to an older version lowers `last` in the same way.

If it's missing, typdoc stops. When a numbered collection has documents but no `last`
recorded, `typdoc new` refuses to guess. The highest existing key might be lower than a number that
was issued and deleted, and a guess would be the reuse this is all meant to prevent. You restore
the file from version control or write it yourself, once, with the highest number you know was
used.

An entry for a collection you removed is kept. If a collection is retired and later brought
back, its entry is the only thing that remembers which numbers were used. `validate` mentions it
as a warning and nothing ever deletes it.

## The lock

Two people, or two agents, running `typdoc new WF ...` at the same moment would both read `last:
12` and both write `WF-13`, unless something stops them. That something is a lock per namespace:
a file under `.typdoc/locks/` that `new` creates before reading `last` and removes after writing
it. The second command waits for the first, then reads `last: 13` and writes `WF-14`.

The same lock covers `set`, so that `--if` really is a compare-and-set: the condition is checked
and the new value written with no chance for another writer to slip in between. `mv` takes the
lock of every namespace it writes to.

A write that can't get the lock within `--lock-timeout` seconds (default 5) gives up with exit 4
and says who holds it. typdoc never deletes a lock it didn't create, however old it is: a lock
that looks stale might belong to a slow command that's still running. If the owning process has
really gone, the message says so and names the file to delete.

A command interrupted with Ctrl-C removes its lock before it exits. A process killed outright, or a
machine losing power, leaves the lock behind, and the next write reports it.

## Numbers per namespace

Each namespace has its own numbers and its own lock, so `story-1:WF-1` and `story-2:WF-1` are
different documents and creating tickets in one story never waits on another.

Moving a numbered document to another namespace with `typdoc mv --renumber` gives it the next
number there and retires its old key: the old namespace's `last` is untouched, so the old number is
never reissued, and every ref to it is rewritten to the new key.

## What the lock doesn't cover

The lock works between typdoc commands on one machine, in one working tree. Two git worktrees, or
two clones, each have their own files, so each can issue the same number on its own. `validate`
catches the duplicate (`keys.unique`) once the branches are merged, and the state file conflicts
on merge, which is the louder signal. Two separate stories in separate namespaces never conflict,
because they never share a state file.

A project on a shared network drive isn't supported.
