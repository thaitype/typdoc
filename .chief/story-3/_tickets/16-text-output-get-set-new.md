# 16: Text output for `get`, `set`, and both forms of `new`

Type: implementation
Status: claimed
Blocked by: None (can start immediately)

Shape is fixed by the contract's text-output table (M-10(a,b,d,f)). This ticket introduces the
one shared renderer the contract's write commands all reuse — ticket 21 (`mv`) depends on it.

## The work

1. A shared function that renders a document as the labeled block: one `name: value` line per
   field — `path`, `collection`, `schema`, `namespace` (and `key` when coded), then frontmatter
   fields in file order. A `number` field prints with the document's own digits (the write path's
   existing `[number-text]` rule already governs `--json`'s printing; reuse it, don't reimplement
   it for text).
2. `get` without `--json` prints that block for the document it read (`cli.rs:246-254`,
   replacing the "not built yet" refusal).
3. `set` without `--json` prints that block for the document as it stands after the write
   (`cli.rs:255-...`).
4. `new <path>` without `--json` prints that block for the newly created document
   (`cli.rs:181-...`, the branch currently refusing at `cli.rs:211`).
5. `new <CODE>` without `--json` prints that block too — **replacing** today's built bare-key
   output (`cli.rs`'s `Ok(document) => { ... stdout: format!("{key}\n") ... }` branch). This is a
   deliberate, accepted design change (M-10(f)); a caller that wants just the key uses `--json`.
6. Every `failure(true, ...)` / `failure_text(true, ...)` call site inside these four branches
   (`get`, `set`, both `new` forms, including the pre-flag-check parse errors in `new` at
   `cli.rs:200` and `cli.rs:220`) is changed to pass the command's real `json` variable. This
   fixes the already-reproducible bug: `typdoc new "not a code and not a path"` (no `--json`)
   currently prints the raw `--json` error object to stderr instead of `typdoc: <message>`.

## Tests (see `testing-decisions.md`, "Text output")

- A hand-written golden per command (`get`, `set`, `new <path>`, `new <CODE>`), each checked for
  the field-names principle directly (no bare, unlabeled line).
- The already-reproduced bug (`typdoc new` with a bad target, no `--json`, prints JSON) has a
  regression test asserting plain-text stderr — kept specifically because it was shown failing
  before this ticket, not invented fresh to match the fix.
- At least one more error case per command (a validation failure on `set`, a duplicate-target
  refusal on `new`) also asserts plain-text stderr.

## Done

- `get`, `set`, and both forms of `new` produce the labeled block without `--json`; none refuses
  with "not built yet."
- Every error path in these four branches prints plain text, not JSON, when `--json` wasn't
  given.
