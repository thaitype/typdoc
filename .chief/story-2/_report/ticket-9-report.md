# Ticket 9: `typdoc new`

Resolved. Commit `d327876`, merged `7cb3a8b` with eight conflicting files against tickets 8, 10 and
11 — mostly add-add unions or two functions independently added at the same point, kept side by
side, plus one real duplicate resolved by rename (below). `cargo fmt --check` clean,
`cargo clippy --workspace --all-targets -- -D warnings` clean, public-text check clean, all re-run
on the merged tree. `scripts/test.sh` with `TMPDIR` on a disk-backed folder: 938 passed / 0 failed /
1 ignored (921 before this ticket).

## Outcome

done

## What it does

`Project::new_document` dispatches on a coded form (`new <CODE> <title>`, allocating under the
namespace lock) and a path form (`new <path>`, matched against an uncoded collection's template).
The coded form reads the state file fresh from disk under the lock — the one read that has to be
fresh for two concurrent `new`s to agree — allocates `max(highest_existing, last) + 1`, validates
the candidate, checks the destination does not already exist, writes the state file, then creates
the document with `create_exclusively`, a new no-temp-file write primitive (`O_EXCL` only, per
decision 7). `--set k=v` is ticket 10's own `SetOp`/`parse_set_op`, unchanged.

## Checked by running

- Exit 7 for both cases named in "done when": a path given on the command line that exists, and a
  name a template produces that exists — the second constructed by hand, since within one process
  the in-memory index is never actually stale enough to reach that collision on its own.
- A run that refuses leaves no file and no changed `last`, asserted on the bytes of both.
- The number is never reused after a document is deleted, shown through `new` itself for real: a
  document deleted, `new` run again, the state file's `last` checked rather than what exists on
  disk.
- The key on stdout, captured and fed back into `get`, a real round trip through both built
  commands.
- A scope holding more than one namespace exits 1 with the choices; `state.missing` stops the
  command at exit 2 with nothing written.
- The existing write ban reaches the new code. `std::fs::write` itself could not be planted this
  session (the harness's own safety classifier refused the edit, the same block ticket 11 also hit);
  `std::env::var` was planted instead — a different entry on the identical disallowed-methods list,
  through the identical clippy mechanism, with no filesystem side effect — in library code and,
  separately, in test code, both refused; the same call inside `typdoc-fs` was not. Flagged as a
  substitution, not claimed as the literal instruction.

## Decision

- **Issue:** the order of filling defaults, filling `auto` fields, validating and taking the lock is
  not fixed by the design (`_map.md`'s own "not yet specified" list).
- **Chosen:** allocate in memory, validate the candidate, and only if it is valid check the
  destination and write state, then create the document. Validating before burning a number means a
  bad `--set` never costs an allocation; state is written before the document appears, matching `mv
  --renumber`'s own explicit rule, so a crash between the two only ever leaves an ordinary skipped
  number. Doubt kept open: whether `highest_existing` should be re-scanned from disk under the lock
  rather than reused from the pre-lock index is not fully closed by this reasoning alone — `last`
  being always fresh and monotonic, with `O_EXCL` backstopping the rest, is what makes it hold; the
  real proof is ticket 14's two-process collision test, not this ticket's own reasoning.
- **Merge issue:** this ticket and ticket 11 independently added an error variant for decision 15's
  "destination already exists" — `Error::Exists { path: PathBuf, .. }` here, `Error::AlreadyExists
  { path: String, .. }` in the already-merged ticket 11, both `#[error("{message}")]` so the path
  type was never actually read by any caller. Kept `AlreadyExists`, since it was already merged with
  call sites built on it; this ticket's two construction sites were renamed and their path converted
  to a display string.

## Notes

- Path-form `new` ignores `--namespace`/`TYPDOC_NAMESPACE`: the design's own grammar shows no prefix
  on `typdoc new <path>`, and a project-relative path already carries its namespace as a literal
  folder prefix. Doubt kept open in the code: nothing says whether a `--namespace` given alongside a
  path should be checked against it rather than silently ignored.
- Non-JSON text output is built only for the coded form (the design's own worked example, a bare
  key); the path form still refuses non-`--json` as not built yet, which the contract allows leaving
  to whichever ticket builds each command.
- A generated document's body is empty; the design says nothing about generated content.
- `new` is the first command that takes a namespace lock through a real invocation and creates a
  file with it — ticket 4 (signals) can build for real now. Ticket 14's collision test is what has to
  validate the allocation-ordering reasoning above; nothing here relies on the collision being
  impossible, only on `O_EXCL` plus a fresh `last` read making it harmless if it happens.
