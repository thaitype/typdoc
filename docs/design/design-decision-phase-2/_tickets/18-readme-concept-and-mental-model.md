# 18: A `Concept and Mental model` section in the README, separating namespace from collection

Type: documentation
Status: open
Blocked by: None (can start immediately)

Not a `wayfinder:*` type on purpose: nothing here is decided, so this ticket is not on the map's
frontier and does not hold `/chief-plan` up. It is work to do, written down where the rest of
story 2's paper lives.

## The work

Add a `Concept and Mental model` section to the README that separates two words: **namespace** and
**collection**.

The two are easy to swap, and they are the vocabulary the tool is built out of. `--namespace` and
`--collection` are flags; `config.namespace-name`, `config.namespaces-entry`,
`config.collection-name`, `config.collection-parse`, `config.collection-schema` and
`collections.overlap` are error ids a user reads when something is wrong. Somebody holding the two
words the wrong way round reads that whole set of ids backwards, and nothing in the output corrects
them. A reader meeting typdoc for the first time has no reason to have the distinction already.

## The shape to use

Four parts, in this order.

**1. An example project with two namespaces by two collections.** Namespaces `story-1` and
`story-2`; in each, a `_tickets` collection whose schema has the code `WF`, and a `_notes`
collection with no code. Two of each, so neither axis can be mistaken for the other — one namespace
with two collections, or two namespaces with one, would each leave the reader able to collapse the
picture back into a single idea.

**2. Real `typdoc list` output** with the columns `namespace`, `collection`, `key` and `path`,
ordered so that `WF-1` is visibly there twice, once in each namespace. That one row pair carries
most of the lesson: the same key, two documents, no conflict.

**3. A table with two axes**, namespaces down the side and collections across the top, and the
sentence that a document always sits at one intersection — never in a namespace alone, never in a
collection alone.

**4. One closing sentence that separates them.** Along these lines, to be written properly in the
section:

> A collection says what kind of thing this is and which set of rules checks it. A namespace says
> what this thing shares its numbering with, and who holds the lock while it is written.

## Constraints

**The output in the README is produced by running it, not written by hand.** `docs/getting-started.md`
already works this way and says so in its first paragraph.

Worth knowing before starting, because it changes what care is needed: nothing checks this. The
shell-examples harness reads `docs/design.md` only, and what it checks is how a shell splits an
example's words before typdoc sees them — not that any printed output is real. So a fabricated
output in the README would pass the whole suite. The discipline is the only guard there is.

**No names, and no trace of who explained what to whom.** The rule that every line reads as the
repository owner's own work still binds, and this section is about a confusion, which is exactly the
kind of subject that invites writing whose confusion it was. Write what the two words mean and why
they are easy to swap, and leave people out of it.

## What was checked before this ticket was written

- `typdoc list` has a text output and it works today; `validate`'s text output does not exist yet
  ("the output without --json is not built yet"), so the section should lean on `list`.
- `list --fields` accepts the pseudo-fields, so the four columns in part 2 are available now:
  `typdoc list --fields namespace,collection,path` printed those columns against a real project.
  `key` is a pseudo-field on the same list and belongs in the same flag.
- So part 2 needs no new command and no change to the binary. This ticket is writing, not building.

## Answer

<filled in on resolve>
