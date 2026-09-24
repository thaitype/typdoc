# 13: Writing `state/<namespace>.json`

Type: wayfinder:grilling
Status: resolved
Blocked by: None (can start immediately)

## Question

`.typdoc/state/<namespace>.json` holds, per collection, the highest number `typdoc new` has allocated in that namespace. Story 1 reads it and reports `state.missing`. Story 2 writes it, from `new` and from `mv --renumber`, and it is the file that stops a number being reused after a document is deleted — so a defect here hands two documents the same key.

The design says the number is replaced "in place", then the result is re-parsed and compared with what was meant before the temp file is renamed. That describes editing an existing entry. It leaves these open:

- **First write.** A collection with no coded documents in the namespace and no record is new, and nothing is reported: "the first `typdoc new` creates the file or the entry". Decide what a created file looks like — key order, indentation, trailing newline — since it is committed and will be diffed and merged by users. Two worktrees on the same namespace are expected to conflict on merge, which only works as a signal if the format is stable enough that a conflict means a real disagreement.
- **A file that parses but is wrong.** `last` holding a string, a negative number, a float, or a number below the highest existing document. `state.missing` covers a missing record; it does not cover a present one that is nonsense. Decide whether each is a config error with an id, a validation finding, or something `new` refuses with its own message, and note that `new` must refuse rather than guess, because guessing issues a key that already exists.
- **Both files in `--renumber`.** It writes the destination's `last` and moves a document out of the source. Are both state files written, and in what order relative to the file move and to the ref rewrites, so that an interruption at any point cannot issue a number twice? An entry written before the document exists is safe (a number is skipped); a document written before the entry is not (the number is issued again next time).
- **Entries that are no longer used.** A collection whose last coded document has been deleted keeps its `last`, which is the point. A collection that no longer exists in the config keeps one too. `config.state-orphan` covers a state file for a namespace that is gone; nothing covers an entry for a collection that is gone. Decide whether that is reported, and whether any command ever removes an entry — the safe answer is that nothing does, and it should be a decision rather than an omission.

## Answer

**Decided, in two rounds. The file's format was settled first; the rest is settled below.**

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

**Where this ticket meets decision 15.** A number issued twice cannot show up as a duplicate key,
because a coded collection's `match` takes `{key}` once and allows no globs, so the key fixes the
path within a namespace. It shows up as a write whose destination already exists, which
[decision 15](15-a-write-whose-destination-already-exists.md) refuses with exit 7 and guards with
`O_EXCL` under the lock. The two tickets are one hazard seen from the state file and from the file
being written.

**Written into `docs/design.md` by that first round:** the State paragraph gives the form of a file
typdoc creates and says plainly that updating an existing entry leaves the rest of the file
untouched.

---

**Decided: a `last` that is there but wrong is its own finding, an entry for a retired collection
is kept, nothing ever removes an entry, and `mv --renumber` writes one state file, not two.**

### A `last` that parses but is wrong

Measured against the binary built from this branch, with a project holding `WF-1` and `WF-2`:

| `last` written in the file | Reported today | Exit |
| --- | --- | --- |
| `"three"`, `null`, `-5`, `2.7`, a number past what can be held | `state.missing` | 2 |
| `1`, with `WF-2` on disk | nothing at all | 0 |

Both lines are wrong in their own way.

**The first tells the user something they can see is false.** The message reads "the collection
`tickets` has documents in this namespace and no `last` recorded in its state file", and the
repair it points at is to restore the file from version control or create it. The record is right
there in the file. A user who opens it finds `"last": -5` and no way to connect it to what they
were told. So a value that is present but unusable is the finding **`state.malformed`**, separate
from `state.missing`, and its message gives the value that is written and what is expected.
`new` and `mv --renumber` refuse to issue a number for that collection in that namespace, as they
do for a missing record and for the same reason: a guessed number is a key that already belongs to
a document.

**The second is silent, and it is the one that matters.** `last: 1` with `WF-2` on disk passes
with no finding. Nothing breaks today, because allocation takes the larger of the highest existing
number and `last`, so the next number is still right — it is right by accident, while a file that
is telling the reader something untrue sits there unremarked. That is **`state.behind`**, at
`warn` rather than `error`: nothing is wrong yet, and a rule that stops a run over a number that
is still correct would be a false alarm, but a record nobody checks is how a wrong one comes to
be trusted.

**Repairing either is the user's move, not typdoc's.** Both messages say the same thing: set
`last` to the highest existing number, or higher if a number was issued and its document has since
gone. typdoc does not set it from the files itself, and this is the part worth writing down,
because deriving `last` from the files is the obvious fix and it is wrong.

The measurement that decides it. A project with `WF-1`, `WF-2`, `WF-3`, `last: 3`, and a body link
in `WF-1` reading `[the third](WF-3.md)`, which `refs` resolves. Delete `WF-3`. The highest
existing number is now 2.

- **Deriving `last` from the files** makes it 2, so the next `new` issues `WF-3` again. The link
  in `WF-1` then resolves to a document that is not the one it was written about, and nothing
  reports it, because the link is well formed and so is the document.
- **Leaving `last` at 3** gives, from the same run:
  `tickets/WF-1.md:5:17  error  link target missing: WF-3.md  [body.links]`

The gap between the highest number issued and the highest that exists is not untidiness to be
cleaned up. It is what turns a silent wrong answer into a loud one, and it is the whole reason the
file exists.

### An entry for a collection the project no longer has

The question this ticket asked was whether that case is reported, since `config.state-orphan`
covers a state file for a namespace that is gone and nothing seemed to cover this. It is covered,
and the coverage is the problem. Measured:

```
typdoc validate --json    -> error, exit 2
typdoc list     --json    -> error, exit 2
typdoc get tickets/WF-1.md --json -> error, exit 2

.typdoc/state/default.json: the state file records `meetings`, which is not a coded
collection of this project: state applies only to a collection whose schema has a code
   [config.state-uncoded]
```

It is a config error, so it stops every command, reads included, and the way out it offers is to
delete the entry. Follow that and the record that `meetings` issued numbers up to 41 is gone;
bring the collection back later and numbering restarts from whatever documents survive, reissuing
keys that were retired. The tool would be requiring the user to destroy the thing it keeps the
file for, in order to run at all.

**Decided: an entry naming a collection the project no longer has is ordinary and is kept.** It is
the finding **`state.retired`**, at `warn`, which names the entry and stops nothing. A collection
can be retired and brought back, and the entry is the only record that its numbers were issued.

**Decided: no command ever removes an entry from a state file.** The ticket guessed that this was
the safe answer and asked for it to be a decision rather than an omission. It is one. Removing an
entry is discarding the only record of a set of issued numbers, and the program cannot know that
they will not be wanted; the user can, and the file is theirs to edit.

**`config.state-uncoded` is narrowed** to what it should have meant: a collection this project has,
whose schema has no code. That is a configuration that cannot be acted on and stays an error. The
two cases were being caught by one rule, and typdoc can tell them apart, because it knows whether
the collection is still in the config.

### Both state files in `mv --renumber`

**One file is written, the destination's.** This follows from rules already made and needs nothing
new: `last` is the highest number ever issued, never the highest that exists, and it never goes
down. `mv --renumber` takes a document out of the source namespace without issuing anything there,
so the source's highest-ever is unchanged and there is nothing to write. Writing it would mean
either writing the same value again, or lowering it to match what is left — which is the reissue
above, with the added harm that the document really has gone somewhere else and a reader following
the old key would land on an unrelated one.

The order was settled by [decision 1](1-a-mv-that-fails-partway.md): the destination's `last` is
written before the document appears under its new key. With only one state file written, that
order is the whole of it. An interruption at any point leaves either a number recorded and not
used, which is an ordinary skip, or nothing at all.

### Written into `docs/design.md`

The State paragraph gains `state.malformed`, `state.behind` and `state.retired`, with what each
means and how it is repaired, and the statement that typdoc never sets `last` from the files and
never removes an entry. The always-on rules table gains the three rules. `config.state-uncoded`'s
row is narrowed and says which case is no longer its own. The description of `new` says that a
record which is present but unusable stops it as a missing one does.

The same paragraph also loses the sentence saying `new` re-parses the result and compares it with
what it meant to write: that guard is dropped in [decision 20](20-the-frontmatter-writer.md), and
what remains true of the write is that it goes to a temp file and is renamed over the original, so
a reader sees one whole file or the other.
