---
title: Writing files
status: active
migrated_from: docs/archived-design/design.md#concurrency
---

## Atomic writes

Each file typdoc writes, `lock.json` and the files in `vendor/` included, is written to a temp
file in the same directory and then renamed over the original, so a reader sees the old file or
the new one whole. What v1 promises is that a document is never left damaged. It does not promise
that no temp file is left behind: `SIGKILL` and a power cut cannot be intercepted, so a promise of
no leftover would be false in ordinary circumstances, and a promise that ordinary events make
false is worse than none. Atomicity is not durability: without an `fsync` before the rename a
power cut can leave the new file in place and empty, and whether typdoc syncs before renaming is
not decided.

## Temp files

A temp file's name follows a reserved shape, `.typdoc-tmp-<pid>-<random>`, and a file whose name
has that shape is never a document, whatever any `match` says. The walker skips such a name by
rule, before `match` is consulted, because a name cannot be made safe by falling outside a glob:
`*` matches a leading dot in a file name, and a project may write `match` as `*`, which matches
everything. The pid and the random part are there so that two writers working at the same instant
cannot choose one name. A leftover that a run meets is skipped and reported as a finding at
`warn`, and it is counted in an audit among the entries that were not read, beside the symbolic
links and the names that are not valid UTF-8.

## Two kinds of rename

Putting a document where no file was, in `new`, `mv` and `mv --renumber`, must not replace
anything: a destination that exists is exit 7 and the write does not happen. Completing an atomic
write of a file that is already there is the opposite case and replaces it on purpose, which is
what makes a reader see the old file or the new one whole. They are different operations with
different requirements, and treating them as one is how a `mv` comes to overwrite a document.

Where the two are enforced differs. `new` creates the file itself, so `O_EXCL` refuses the write
in the same call and nothing has to be remembered. `mv` and `mv --renumber` move a file that
exists, and the plain rename a file system offers replaces in silence, so they check the
destination under the lock and then rename. That leaves an instant between the check and the
rename. Under the lock no other typdoc can be in the namespace, so only a program that is not
typdoc can reach it, by creating a file at exactly that path in that instant; the instant is left
open knowingly. A call that takes a name or fails would close it, at the cost of a document that
briefly has two names and of a rule for meeting that state on a re-run, which buys less than it
costs. typdoc does not own the file system, and a lock binds typdoc alone.

## Removing a leftover

A command that holds a lock may remove leftover temp files within that lock's scope, and no
command removes one otherwise. The evidence is the lock: while it is held no other typdoc is
writing in that namespace, so a leftover there belongs to a process that has gone. The age of the
file is never a criterion, as it is never a criterion for a lock. A removal that fails does not
make the write fail: the document matters more than a tidy folder.

## Permissions

A rename replaces the inode, so the mode of the file that ends up in place comes from the temp
file rather than from the document that was there; a file set to `600` becomes whatever the umask
allows, silently. typdoc therefore carries the existing file's mode to the temp file before
renaming. It carries the mode and nothing else: the owner and group are not carried, since
changing them needs privilege typdoc does not have, and access control lists and extended
attributes are not carried either. A file that did not exist has no mode to carry and gets the
default.
