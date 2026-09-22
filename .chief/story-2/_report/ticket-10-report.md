# Ticket 10: `typdoc set`

Resolved. Commit `d85260b`, merged `0c258bd` with three add-add conflicts against ticket 8,
resolved as unions with no design decision needed. `cargo fmt --check` clean,
`cargo clippy --workspace --all-targets -- -D warnings` clean, public-text check clean (one wording
slip in an earlier report, caught here, fixed before this commit), all re-run on the merged tree.
`scripts/test.sh` with `TMPDIR` on a disk-backed folder: 882 passed / 0 failed / 1 ignored (862
before this ticket).

## Outcome

done

## What it does

`Project::set` dispatches on whether a document is in a namespace. Both paths acquire a
`NamespaceLock` before reading the file, deciding `--if` and validating, so a condition and the
write it guards cannot be separated by another process; a document outside every namespace uses its
own lock (`.typdoc/locks/.loose.lock`, invented for this case since no namespace applies and a lock
is required to write at all) and skips validation, since there is no schema.

The candidate text is built by reading the file's current fields, applying the write's field
operations through `FrontmatterWriter` — two new operations, `set_list` (replace a field's whole
value) and `remove_field` (remove a field entirely), since neither of ticket 5's existing operations
expresses `set`'s own grammar — splicing the result with the unchanged body, and re-parsing it for
both validation and the printed document. `frontmatter.transitions` is built and checked here,
alongside the existing per-document checks run directly against the candidate text; any error
finding refuses the write before `write_atomically` is called. `auto: update` stamps only when a
value actually changed; writing an `auto` field directly is refused; `k=` removes a field.

`--json` prints the document after the write, `{ "document": ... }`, in `get`'s own shape, with no
list of what changed. A false `--if` exits 3 naming the condition; a refused `auto` write exits 2.

## Checked by running

- The transitions fixture — the first `fixtures/broken/` entry whose declared command is a write —
  runs on a staged copy, exits 2, names the finding, and leaves both the staged file and the
  repository's own copy byte-identical.
- A false `--if` leaves the file byte-identical, asserted on bytes.
- A document outside every namespace writes as an ordinary write with `namespace` left out of
  `--json`, asserted against the contract's own default.
- A twenty-digit `count` field untouched by the write prints unconverted on stdout and reads back
  the same through a follow-up `get --json`, since parsed-JSON comparison cannot tell the digits from
  a converted float — proving ticket 7's printing fix and ticket 5's value promise hold together
  through this command, not only by inheritance.
- Eight things were shown able to fail, one at a time, each restored: the existing write ban reaching
  the new code (library and test), `check_transitions`, the auto-field-direct-write refusal,
  `fields_changed`, the new `set_list`/`remove_field` operations, the `--if` evaluator, and the
  outside-every-namespace detection.
- Building the required golden under `fixtures/output/set/` exposed that the golden harness ran every
  case directly in the fixture's own folder — safe for a read, not for a write. Fixed to stage
  through the same mechanism ticket 2 built; the fixture it runs against was confirmed
  byte-identical before and after the fix.

## Decision

- **Issue:** during its own code review, this ticket found `set`'s no-namespace path decided `--if`
  from a read taken before the lock was acquired — a window the design's own words ("`set` holds it
  across read, `--if`, validation and write") rule out.
- **Chosen:** reordered to acquire the lock first, matching the in-namespace path. Every gate was
  re-run after the fix with no change to the counts.

## Notes

- **Ticket 3's hostname thread is closed.** `Env::hostname()` is added; the real implementation
  reads `/proc/sys/kernel/hostname`, every test implementation returns a fixed string.
- **`git-common` lock mode is refused plainly**, not attempted: `git rev-parse --git-common-dir`
  resolution is not built by this ticket.
- **Ticket 3's `order_locks` canonicalization thread is still open**: `set` only ever takes one lock,
  so nothing here calls it. `mv`, which takes two, is where it has to be closed.
- **Ticket 5's two open writer questions are not exercised by `set`**: a whole-list replacement goes
  through the new `set_list`, and a whole-field removal through the new `remove_field`, so neither of
  `append_item`'s or `remove_item`'s documented single-item behavior is on `set`'s path. They stay
  exactly as ticket 5 left them.
- `SetOp`/`parse_set_op` (the `k=v`/`k=` parser) live separately from `Project::set`'s orchestration,
  so `new`'s `--set k=v` can reuse them directly if its grammar matches, which the design suggests it
  does.
- A false `--if` uses an ad-hoc `details[].rule` id, `"set.if"`, on the same footing as `config.*`
  ids — not a `validate`-family rule, so it needs no fixture or `RULES` entry.
- `--if` supports only a plain `field op value` condition; a `ref.*`/`refby.*` condition is refused
  as bad arguments rather than evaluated, judged out of proportion for this ticket since no worked
  example in the design combines them.
- Exit 4 through a real command needs the multi-process harness ticket 14 builds, which needs ticket
  9; it correctly stays out of reach here.
