---
title: State and numbering explained
status: active
migrated_from: docs/archived-design/design.md#config-typdocconfigjson
---

`.typdoc/state/<namespace>.json` holds, per collection, the highest number ever issued in that
namespace — by `typdoc new`, and by `mv --renumber` in the namespace it moves a document into.
It is the highest number issued, not the highest number that exists. The two differ whenever a
document is deleted, or moved out by `mv --renumber`, and the difference is ordinary and is left
alone. `last` is never lowered to match what is on disk, not even when the document that held
the highest number has gone: lowering it makes `typdoc new` issue that number a second time, and
a ref written `story-1:WF-9` before the move would then resolve to a different document, with
nothing to report because both the ref and the document are well formed. The gap is what keeps a
retired key retired.

```json
// .typdoc/state/story-3.json
{ "wayfinder": { "last": 7 } }
```

When typdoc creates the file, or adds an entry to one, it writes JSON with the keys in
alphabetical order, two spaces of indentation, `\n` line endings and a final newline. The file is
committed, so it is read as a diff and merged, and a form that does not move means a conflict
says that two people disagree about a number rather than that the formatting changed under them.
Updating an entry that is already there replaces only the number, in place, and so leaves the
rest of the file exactly as it was found. It is written by `typdoc new` and by `mv --renumber`,
the commands that issue numbers, and not by hand except as described next; it is what stops a
number being reused after its document is deleted.

**`state.missing`.** A collection that has coded documents in a namespace and no `last` recorded
there, because the file or the entry is missing, is the reverse of an orphan: either the record
was lost, or the documents were made before typdoc was used. The always-on rule `state.missing`
reports it, and `typdoc new` and `mv --renumber` refuse to issue a number for that collection in
that namespace, because the highest existing number may be lower than a number that was issued
and later deleted. To go on, restore the file from version control, or create it with `last` set
to the highest existing number, or to a higher number if one was ever used and deleted. A
collection with no coded documents in the namespace and no record is new, and nothing is
reported: the first `typdoc new` creates the file or the entry. A record can be lost without a
trace only when every coded document of the collection in that namespace has also been deleted,
since nothing is left to compare it with. The namespace `default` uses `state/default.json`, so
nothing in the config can change which file holds the numbers. `typdoc new` replaces only the
number, in place, and writes the result to a temp file which is renamed over the original, so a
reader sees the old file or the new one whole.

**`config.state-orphan`.** A file in `state/` that matches neither a current namespace nor one a
`!` entry is currently excluding is reported as the finding `config.state-orphan`, which names it
and stops nothing: delete it after removing a namespace, or rename it after renaming a folder or
after moving from one namespace to several (`default.json` becomes `<folder>.json`). An excluded
namespace's own state file is left alone entirely, matching or not, so it never counts as an
orphan while the exclusion holds.

**`state.malformed`.** An entry whose `last` is present but is not a whole number that can be
held — text, `null`, a negative number, a fraction, or a number too large — is `state.malformed`,
not `state.missing`: the record is there, and telling a user that a value they can see in the
file is absent sends them to restore a file that is not lost. The message gives the value that is
written and what is expected, and `typdoc new` and `mv --renumber` refuse to issue a number for
that collection in that namespace, because a number guessed here is a key that already belongs
to a document.

**`state.behind`.** An entry whose `last` is a whole number lower than the highest existing
number of that collection in the namespace is `state.behind`, at `warn`: allocation takes the
larger of the two, so the next number is still right, but the file is telling the reader
something untrue and a silent wrong record is how a real one comes to be trusted. Both
`state.malformed` and `state.behind` are repaired the same way, and the message says so: set
`last` to the highest existing number, or higher if a number was issued and its document has
since gone. typdoc does not set it from the files itself. The highest number that exists and the
highest that was issued are different whenever something was deleted, and lowering `last` to
match the files reissues a retired key — a reference to the old document then resolves to a new
one, with nothing to report, where keeping the gap leaves an ordinary missing-target finding that
says where to look.

**`state.retired`, `config.state-uncoded`.** An entry naming a collection this project no longer
has is ordinary and is kept. A collection may be retired and brought back, and its entry is the
only record that its numbers were issued; making a user delete it to go on would destroy exactly
what the file is for. It is the finding `state.retired`, at `warn`, which names the entry and
stops nothing, and no command ever removes an entry from a state file. An entry naming a
collection that does exist but whose schema has no code is a different thing — a configuration
that cannot be acted on — and stays the config error `config.state-uncoded`. Worktrees that work
on different namespaces never change the same state file; see `SPC-10` for two on the same
one.

`docs/design/catalog/config-errors.md` and `docs/design/catalog/rules.md` hold the
machine-readable ids the rules above produce; this document explains the behavior behind them.
