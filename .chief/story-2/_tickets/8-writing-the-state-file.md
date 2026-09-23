# 8: Writing the state file, and the rules that read it

Type: implementation
Status: resolved
Blocked by: 1, 3

## What this delivers

- The state file written by the commands that issue numbers, under the lock; the form of a file typdoc creates — keys in alphabetical order, two spaces, `\n` endings, a final newline — while updating an existing entry replaces only the number and reformats nothing (decision 13).
- `state.malformed` for a `last` that is present and unusable; `state.behind` at `warn` for a `last` below the highest that exists; `state.retired` at `warn` for an entry whose collection the project no longer has, stopping nothing.
- `config.state-uncoded` narrows to a collection that exists whose schema has no code. Its existing fixture is that case and stays as it is.
- Nothing is repaired and no entry is ever removed.

## Done when

- Each of the three rules has a fixture that turns it red, and a project where the state file is correct and the rule is silent.
- `state.malformed`'s fixture is one project whose namespaces carry a state file each, with `last` as text, as `null`, negative, a fraction, and too large, so `broken/` keeps one folder per rule.
- A test records the measurement that decides why these rules never repair: with a key's document deleted, deriving `last` from the files hands that key out again and an existing link resolves to the wrong document with nothing reported, where keeping the gap gives a missing-target finding from the same run.
- `state.retired` does not stop a read, which its predecessor as a config error did.
