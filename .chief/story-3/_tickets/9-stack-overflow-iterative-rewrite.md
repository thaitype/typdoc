# 9: Rewrite `cyclic_nodes`'s `visit` as an iterative walk

Type: implementation
Status: resolved
Blocked by: None (can start immediately)

Ticket 1 (wayfinder research) found the root cause and reproduction; this ticket builds the fix
it recommended. See ticket 1's Answer for the full analysis — not repeated here.

## The work

Rewrite `visit`, the inner function of `cyclic_nodes` in `crates/typdoc-core/src/refs.rs:486-512`,
as an explicit iterative depth-first search using a heap-allocated work stack (a `Vec` of frames,
each tracking a node and its position in its child list) instead of the call stack. Same
three-colour algorithm, same return value, for every input — this changes where the growth lives,
not what the function computes.

## Tests (see `testing-decisions.md`, "The stack-overflow fix")

1. **Before the rewrite:** add the regression test — a synthetic one-way chain of at least
   200,000 `acyclic`-referenced documents, no cycle, run on a thread built with
   `std::thread::Builder::stack_size` set to an explicitly small value (not this machine's
   default). Confirm it fails (aborts) against today's recursive `visit`. Capture this in the
   ticket's report.
2. Apply the rewrite.
3. Confirm the regression test now passes, and that its failure mode before the fix showed up as
   a failure in `scripts/test.sh`'s own output, not a harness that silently exited early.
4. Re-run the four existing `cyclic_nodes` unit tests (two-node cycle, self-loop, no-cycle chain,
   cycle-with-a-tail) unchanged — same inputs, same expected outputs.

## Done

- `visit` is iterative; no recursion remains in `cyclic_nodes`'s walk.
- The regression test is committed, was shown red before the fix (report says so), and is green
  after.
- The four existing unit tests pass unchanged.
