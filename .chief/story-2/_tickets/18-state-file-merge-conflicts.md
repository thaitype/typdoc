# 18: Write down how to resolve a merge conflict in a state file

Type: implementation
Status: claimed
Blocked by: None (can start immediately)

Moved here from the design-decision registry: nothing about it was ever a decision to make, only
writing to do, and a decision registry is not where a story keeps its own work. It stays a separate
ticket from [17](17-readme-concept-and-mental-model.md) rather than folding into it, because it is a
different subject in a different file, and folding two unrelated pieces of writing into one ticket
is how one of them ends up half done.

## The work

Write, as guidance rather than as a rule in the code: **when `.typdoc/state/<namespace>.json`
conflicts in a merge, take the higher `last`. Never take a side.**

And, for the same reason: **never revert a state file to an older version.**

## Why it has to be written down

The file is committed, and the design says in as many words that a conflict here is expected: two
worktrees on the same namespace change the same line, and the conflict is called a louder signal
than the duplicate key it prevents. So this is not a rare case someone might hit. It is the case the
design is counting on.

And a conflict in a small JSON file is the kind a person resolves quickly, usually by keeping their
own side, because their side is the change they were just making. Keeping the lower side is the
failure.

## Why the damage does not show up when it is done

This is the part worth spelling out, because the harm looks impossible at first.

Take the lower side, and `last` is now below a number that was already handed out. Nothing breaks
yet. `typdoc new` allocates from the larger of the highest number that exists in the collection and
`last`, so as long as the document holding the high number is still there, the file on disk covers
for the wrong `last`, and every key that follows is correct.

It breaks only for a key that was issued and whose document no longer exists: one that was deleted,
or moved out by `mv --renumber`. There is no file left to count, so `last` is the only remaining
memory that the number was ever used. With `last` too low, `typdoc new` hands that key out again, to
a different document.

Then a ref written before, pointing at that key, resolves — to the wrong document. And `validate`
reports nothing, because there is nothing wrong to find: the key exists, the ref resolves, the
document is valid. The failure produces no finding at all.

That is also why reverting the file is the same mistake wearing different clothes.

## Where it goes

`docs/projects.md` has a `## State` section, which is where someone who is looking at
`.typdoc/state/<namespace>.json` ends up. The guidance belongs there, next to the description of the
file, not in the full design document — the reader who needs it is holding a conflict marker, not
reading the design front to back.

Keep it short: the rule first, the reason after it, and the sentence about not reverting. Somebody
in the middle of a merge reads the first line and needs it to be the answer.

## Answer

<filled in on resolve>
