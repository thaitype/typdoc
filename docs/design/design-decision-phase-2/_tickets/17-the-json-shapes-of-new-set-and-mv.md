# 17: The `--json` shapes of `new`, `set` and `mv`

Type: wayfinder:grilling
Status: resolved
Blocked by: 1, 11, 16

## Question

Story 1 fixed the `--json` shapes of the read commands and of the error object, and with them the rules every shape follows: every command prints one object with a named field for its result, never a bare value; there is one way to name a document (`path`, `namespace`, `key`, `project`) and every shape that mentions one uses it; a value in the output is a fact about the document, never about the command that asked; the output promises only what a caller cannot work out from what it already has; and a consumer ignores fields it does not know. The write commands were left to this story.

Decide each shape against those rules:

- **`new`.** The natural result is the document it created, in the same document shape `get` prints. Decide whether it is the whole document — which repeats every default and `auto` value the command filled in, and is genuinely what the caller could not know — or only the name plus the key. The text form prints the bare key, and the two forms should be answering the same question.
- **`set`.** Decide whether the result is the document after the write, and whether it says which fields changed. "Which fields changed" is a fact about the command, not about the document, which the rules argue against — but a caller that sent five fields and had `auto: update` touched as well cannot work it out from the result alone, which argues for it. Decide, and record the reasoning either way. Also decide what `--if` failing prints: it exits 3 with nothing written, and nothing written is a result, not an error, yet exit 3 is not success.
- **`mv`.** It has more to say than the others: the document under its new name, what it could not rewrite and why (unreachable projects, mentions, body links with the rule off), and — depending on ticket 1 — whether it finished. Decide the field that carries the unrewritten refs, and whether each entry is a reference in the shape `refs` already uses, since that shape exists and carries `written` and a position.
- **The schema check after a move.** [Decision 16](16-mv-across-collection-boundaries.md) settled
  that a `mv` landing on a schema the document does not satisfy is carried out, exits 0, and reports
  what the schema rejects in the payload. So `mv`'s shape needs a field for that result, and it is
  the field a caller branches on instead of reading the exit code. Decide whether it holds findings
  in the shape `validate` already uses, which would let one consumer read both.
- **Order.** Every array in the output declares its order. Say the order of each array these three introduce.
- **The lists in the test suite.** `unimplemented_commands` and `unproduced_exit_codes` hold the commands with no golden and the exit codes no test produces. The write half of both empties in this story; exit code 3 in particular is produced only by a write. Confirm that the shapes decided here are the ones the goldens pin.

## Answer

**Decided: `new` and `set` print the document and nothing about the write; `mv` adds the refs it
could not rewrite and the findings of the destination's schema; an `--if` that is false goes to
standard error like every other non-zero exit.**

Story 1's rules decided every one of these, and the work was applying them rather than choosing
freely. Where a rule and a convenience pulled apart, the rule won, and the reasoning is recorded
so the same argument does not have to be had again.

### `new`: the whole document

`{ "document": <document> }`, the same object `get` prints.

The whole document rather than the name and the key, because `new` fills in every default and
every `auto` field, and those values are precisely what the caller cannot work out from what it
gave. That is the rule about what the output promises, used in the direction it usually cuts the
other way. One shape for a document everywhere also means a caller that already reads `get` reads
this with no new code.

The text form printing the bare key is not a disagreement. It answers the same question with less
of it, which is what a text form is for.

### `set`: the document after the write, and not what changed

`{ "document": <document> }`.

The ticket set out the argument for a `changed` field: a caller that sent five fields and had an
`auto: update` touched as well cannot work out from the result which of them moved. That is true,
and it is not enough.

**The test that decides it:** would the value differ for the same document reached another way? A
document whose `status` is `done` has `status: done` however it got there. A `changed` holding
`["status", "updated_at"]` would say `["status"]` after one route and nothing at all after
another, for the same document in the same state. So it is a fact about the command, which the
rules put outside the output, and the rule is not a formality here — a field that describes the
call is a field every other caller has to learn to ignore.

**What the case behind the argument actually needs.** A caller that must act only when a document
is in a given state is doing compare-and-set, and the design already has the mechanism: `--if`,
checked under the same lock as the write, with exit 3 when it is false. That is a stronger answer
than a `changed` field, because it decides before the write rather than reporting after it.

**The limit, stated rather than hidden.** A caller that genuinely wants a before and an after has
to `get` before it writes, and that read is not under the write's lock. Nothing in v1 closes that,
and nothing in v1 needs it.

### An `--if` that is false

Exit 3, nothing written, and the error object on standard error.

It is not an error in the sense of a fault, and the temptation is to print the result on standard
output because nothing went wrong. The rule that pays better is the total one: **a zero exit puts
the result on standard output, a non-zero exit puts the error object on standard error.** A caller
can hold that in its head. The alternative is a table of which non-zero codes print where, and a
caller that gets it wrong parses nothing and reports a crash.

The error object earns its place here rather than being a formality: its `details` name the
condition that was false. A caller that gave several `--if` expressions cannot work that out from
anything else, so the payload is real.

### `mv`

`{ "document": <document>, "unrewritten": [<reference>, ...], "findings": [<finding>, ...] }`.

**`document`** is the document under its new name. That is also how `mv --renumber` reports the key
it issued: in the `key` of the document, which is where every other shape carries a key, rather
than in a field of its own. The old name is not repeated, because the caller gave it.

**`unrewritten`** is the reference object the design already has, taken from the direction
`refs --reverse` takes: the name is the document that holds the ref, with `field`, `written` and a
position. That shape exists, it carries `written`, and reusing it means the entries can be handed
straight back to whatever reads `refs`.

Each entry also has **`reason`**, because a caller that is told a ref was left alone and not why
cannot decide what to do about it: `imported-project` for a ref held by a project this one
imports, which [decision 2](2-locks-when-mv-writes-into-an-imported-project.md) made read-only
with no exception; `mention` for plain text the `body.mentions` rule checks, which `mv` never
rewrites; `links-rule-off` for a body link in a document whose `body.links` rule is off. `reason`
sits beside `unresolved` in the reference object as a second thing that can be said about a ref,
and it follows the rule that a new shape adds to the name rather than renaming part of it.

**`findings`** carries what the destination's schema rejects when a move lands a document in a
collection whose schema it does not satisfy.
[Decision 16](16-mv-across-collection-boundaries.md) settled that such a move is carried out and
exits 0, on the ground that exit 2 always means nothing was written and giving it two meanings
would leave a caller unable to tell whether the world had changed. That decision is what makes
this field necessary: **it is the field a caller branches on, in place of the exit code.** They
are the finding object `validate` prints, which is what lets one consumer read the result of a
move and the result of a check with the same code.

**No count of what was rewritten.** It is a fact about the command, and the same argument as
`changed` applies. Because a consumer ignores fields it does not know, adding one later is not a
breaking change, so nothing is lost by leaving it out now and something is lost by promising it
forever.

**A `mv` that stops partway is not this shape.**
[Decision 1](1-a-mv-that-fails-partway.md) settled that such a run says so and says the command
can be run again. That makes it an error: a non-zero exit, the error object on standard error,
naming what was done and that a second run finishes it. So there is no `complete` field on the
success path, because it would always be true, and a field that is always true promises nothing.

### Order

Every array declares its order, so these do.

- **`unrewritten`** is ordered as `refs --reverse` is: the documents that hold the refs, those of
  this project first and then those of imported projects by alias, each by `path` as under
  Sorting, and within a document the fields in the order they appear, then `$body` by position.
  The order is borrowed rather than invented, so a caller that merges this with a `refs` result
  does not have to re-sort.
- **`findings`** in `mv` is ordered as findings are everywhere: by `path`, then `line`, `col`,
  `rule` and `message`.
- `new` and `set` introduce no array of their own; `fields` is an object, and a list value inside
  it is in the order it is written in the document.

### The lists in the test suite

Confirmed, with the lists read rather than remembered. `registry::UNIMPLEMENTED_COMMANDS` holds
`new`, `set` and `mv` for this story and `pull` for story 3; `registry::UNPRODUCED_EXIT_CODES`
holds 3, 4 and 7, all three marked story 2. The shapes above are what the goldens pin, and each
entry leaves its list when the command that produces it lands:

- 3 comes from a `set --if` that is false, which is the only way to reach it.
- 4 comes from a command that cannot take a lock within `--lock-timeout`.
- 7 comes from `new` onto a path that exists, and from a `mv` whose destination exists
  ([decision 15](15-a-write-whose-destination-already-exists.md)).

An entry that outlives its work turns the suite red, which is the point of the lists, and none of
them may be edited to make a test pass.

### Written into `docs/design.md`

The command table gains rows for `new`, `set` and `mv`. A paragraph gives the two document shapes
and says why neither reports what changed; a second says where an `--if` that is false prints and
why; a third gives `mv`'s shape, `reason`'s three values, what `findings` is for, and that a run
which stops is an error rather than this shape. The Order paragraph gains `mv`'s two arrays.
