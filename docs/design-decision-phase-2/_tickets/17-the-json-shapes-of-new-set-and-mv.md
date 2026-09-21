# 17: The `--json` shapes of `new`, `set` and `mv`

Type: wayfinder:grilling
Status: open
Blocked by: 1, 11, 16

## Question

Story 1 fixed the `--json` shapes of the read commands and of the error object, and with them the rules every shape follows: every command prints one object with a named field for its result, never a bare value; there is one way to name a document (`path`, `namespace`, `key`, `project`) and every shape that mentions one uses it; a value in the output is a fact about the document, never about the command that asked; the output promises only what a caller cannot work out from what it already has; and a consumer ignores fields it does not know. The write commands were left to this story.

Decide each shape against those rules:

- **`new`.** The natural result is the document it created, in the same document shape `get` prints. Decide whether it is the whole document — which repeats every default and `auto` value the command filled in, and is genuinely what the caller could not know — or only the name plus the key. The text form prints the bare key, and the two forms should be answering the same question.
- **`set`.** Decide whether the result is the document after the write, and whether it says which fields changed. "Which fields changed" is a fact about the command, not about the document, which the rules argue against — but a caller that sent five fields and had `auto: update` touched as well cannot work it out from the result alone, which argues for it. Decide, and record the reasoning either way. Also decide what `--if` failing prints: it exits 3 with nothing written, and nothing written is a result, not an error, yet exit 3 is not success.
- **`mv`.** It has more to say than the others: the document under its new name, what it could not rewrite and why (unreachable projects, mentions, body links with the rule off), and — depending on ticket 1 — whether it finished. Decide the field that carries the unrewritten refs, and whether each entry is a reference in the shape `refs` already uses, since that shape exists and carries `written` and a position.
- **Order.** Every array in the output declares its order. Say the order of each array these three introduce.
- **The lists in the test suite.** `unimplemented_commands` and `unproduced_exit_codes` hold the commands with no golden and the exit codes no test produces. The write half of both empties in this story; exit code 3 in particular is produced only by a write. Confirm that the shapes decided here are the ones the goldens pin.

## Answer

<filled in on resolve>
