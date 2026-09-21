# 13: Writing `state/<namespace>.json`

Type: wayfinder:grilling
Status: open
Blocked by: None (can start immediately)

## Question

`.typdoc/state/<namespace>.json` holds, per collection, the highest number `typdoc new` has allocated in that namespace. Story 1 reads it and reports `state.missing`. Story 2 writes it, from `new` and from `mv --renumber`, and it is the file that stops a number being reused after a document is deleted — so a defect here hands two documents the same key.

The design says the number is replaced "in place", then the result is re-parsed and compared with what was meant before the temp file is renamed. That describes editing an existing entry. It leaves these open:

- **First write.** A collection with no coded documents in the namespace and no record is new, and nothing is reported: "the first `typdoc new` creates the file or the entry". Decide what a created file looks like — key order, indentation, trailing newline — since it is committed and will be diffed and merged by users. Two worktrees on the same namespace are expected to conflict on merge, which only works as a signal if the format is stable enough that a conflict means a real disagreement.
- **A file that parses but is wrong.** `last` holding a string, a negative number, a float, or a number below the highest existing document. `state.missing` covers a missing record; it does not cover a present one that is nonsense. Decide whether each is a config error with an id, a validation finding, or something `new` refuses with its own message, and note that `new` must refuse rather than guess, because guessing issues a key that already exists.
- **Both files in `--renumber`.** It writes the destination's `last` and moves a document out of the source. Are both state files written, and in what order relative to the file move and to the ref rewrites, so that an interruption at any point cannot issue a number twice? An entry written before the document exists is safe (a number is skipped); a document written before the entry is not (the number is issued again next time).
- **Entries that are no longer used.** A collection whose last coded document has been deleted keeps its `last`, which is the point. A collection that no longer exists in the config keeps one too. `config.state-orphan` covers a state file for a namespace that is gone; nothing covers an entry for a collection that is gone. Decide whether that is reported, and whether any command ever removes an entry — the safe answer is that nothing does, and it should be a decision rather than an omission.

## Answer

**Partly decided. The file's format is settled; the rest of this ticket is not.**

**Decided: the form of the file typdoc writes.** JSON, keys in alphabetical order, two spaces of
indentation, `\n` line endings, and a final newline.

The reason is what the file is for. It is committed, so people read it as a diff and git merges it.
A form that does not move means a conflict says that two people disagree about a number, which is
the signal the design wants, rather than saying that the formatting shifted under them.

**Where that applies, and where it does not.** This is the form typdoc writes when it creates the
file or adds an entry to one. Updating an entry that is already there still replaces only the
number, in place, as the design says, and leaves the rest of the file exactly as it was found — a
file somebody formatted by hand is not reformatted by a `new`. The two rules do not conflict; the
boundary between them is written into the design so that nobody later reads the format rule as a
licence to rewrite the whole file.

**Not decided here, moved to the family of decision 9.** What happens when the file parses but
`last` is a string, negative, a float, or lower than the highest existing document in the
collection: whether each is a config error with an id, a finding, or something `new` refuses with a
message of its own. Those are error-id and exit-code questions and are being decided together with
the rest of that family, not here.

**Also still open in this ticket:** both state files in `mv --renumber` and the order of those
writes against the file move and the ref rewrites, and whether anything ever removes an entry for a
collection that no longer exists. Neither is answered yet.

**Written into `docs/design.md`:** the State paragraph now gives the form of a file typdoc creates
and says plainly that updating an existing entry leaves the rest of the file untouched.
