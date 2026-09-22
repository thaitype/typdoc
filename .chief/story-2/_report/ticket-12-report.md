# Ticket 12: `mv --renumber`

Resolved. Commit `3085193`, merged `8c216fc` with no conflicts. `cargo fmt --check` clean,
`cargo clippy --workspace --all-targets -- -D warnings` clean, public-text check clean, all re-run
on the merged tree. `scripts/test.sh` with `TMPDIR` on a disk-backed folder: 971 passed / 0 failed /
1 ignored (948 before this ticket).

## Outcome

done

## What it does

`typdoc mv FROM --renumber NAMESPACE` takes one positional in this mode; giving both a destination
and `--renumber`, or neither, is exit 1. The next key is allocated from the destination namespace's
own state, through `Project::allocate_key`, extracted from `new`'s own allocation block and shared by
both. Refused at exit 1: a same-namespace destination, a `project::` prefix on either argument, an
uncoded source, an unknown namespace. Destination state is written before `mv::commit` runs at all,
matching decisions 1 and 13's ordering — the source writes nothing, since its `last` never goes
down. The new key prints bare on stdout, as `new` already does; `document.key` in `--json` is now
populated for a coded destination, which a pre-existing bug in `mv_result` had hardcoded to `None`.

A real gap surfaced by this ticket and fixed: `mv::rewritten_path_ref` had no branch for a key-shaped
ref, because before `--renumber` a key-shaped ref could never point at a document plain `mv` was
moving. A bare key is promoted to the target's sibling-prefixed form; a sibling-prefixed key keeps
key form with both prefix and key updated.

## Checked by running

- The same-namespace refusal writes nothing and leaves `last` unchanged, asserted on bytes.
- An interruption between the state write and the document appearing skips a number and never
  issues one twice, proven at the primitive level: `write_state` then `commit` against a fake file
  system stopped at two different points, and separately, the state an interrupted run leaves is
  built by hand and shown to give the next `mv --renumber` the following number, never the skipped
  one.
- A ref left behind fails validation as an ordinary `refs.resolve` finding rather than resolving to
  another document.
- `--renumber` with no value, and a `project::` prefix on either argument, are exit 1.
- Five things were shown able to fail, one at a time, each restored: the allocation formula, the
  same-namespace check, key-form ref rewriting, the `auto: moves` value, and the JSON key field.
- The existing write ban reaches every new function, including one added during the review response
  (`allocate_key`); not re-demonstrated inside `typdoc-fs`, matching ticket 11's own choice not to
  repeat that half when no `typdoc-fs` code is touched.

## Notes

- **What the interruption tests do not prove, and why not yet:** neither test drives a live kill
  through `Project::mv_renumber` itself, only the composability of the primitives underneath it — a
  mutation swapping the state-write and the move produced no red test, since that ordering is only
  observable under a real interruption. A live-interrupt test needs a signal/kill mechanism, which is
  ticket 4's territory and wasn't reachable from this branch point; this mirrors exactly how ticket
  11's own tests cover `mv::commit` directly rather than `Project::mv`.
- `--renumber` given no value relies on clap's own error text, not a hand-written sentence — matching
  every other missing-required-argument case in this codebase already.
- A key ref renumbered into the holder's own namespace keeps its sibling-prefixed form rather than
  downgrading to bare, pinned by a test: "keeping each ref's written form" means keeping the
  category, not minimizing the spelling, the same reading already established for path refs.
- This ticket's own build was interrupted partway by the same machine-wide crash the loop hit
  generally; recovered with the diff re-verified and every gate re-run from scratch before
  committing, so nothing here rests on a pre-crash assumption.
