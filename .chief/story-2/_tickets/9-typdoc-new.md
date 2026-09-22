# 9: `typdoc new`

Type: implementation
Status: claimed
Blocked by: 3, 5, 8

## What this delivers

- Allocation under the namespace's lock: the larger of the highest number that exists in the collection and the collection's `last`, plus one, written back as the new `last` before the command returns.
- Defaults and `auto` fields filled, the document validated, then written; the file created with `O_EXCL` so a destination that exists is refused at exit 7 with nothing written (decision 15).
- The key printed bare on standard output; `--json` printing `{ "document": ... }`, the whole document, as decision 17 fixes.
- A scope holding more than one namespace exits 1 with the choices; `state.missing` stops the command at exit 2 with nothing written.

## Done when

- Exit 7 is produced for a path given on the command line that exists, and for a name a template produces that exists.
- A run that refuses leaves no file and no changed `last`, asserted on the bytes.
- The number is never reused after a document is deleted, shown against a state file that records it.
- The key on stdout can be captured and fed back to `get`.
