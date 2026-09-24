# Ticket 9 Report

## Ticket

Rewrite `cyclic_nodes`'s `visit` (`crates/typdoc-core/src/refs.rs`) as an iterative DFS, fixing
the stack overflow ticket 1's research found on a long one-way `acyclic` chain.

## Outcome

done

## Notes

Rewritten as an explicit iterative DFS over a heap-allocated `Vec<Frame>` (`Frame { node,
next_child }`), same three-colour algorithm and push/pop/colour order as the recursive form.
Reviewed the diff directly before merging (not just trusting the build report): the cycle-back
check (`stack.iter().position(...)` on a Gray hit) and the finish/pop/blacken sequence match the
original semantics.

Regression test `a_very_long_one_way_chain_with_no_cycle_does_not_overflow_the_stack`: a 200,000-
node one-way chain, run on a thread with an explicit 1 MiB stack. Confirmed red against the old
recursive code via `scripts/test.sh` itself (exit 101, `has overflowed its stack` / SIGABRT in
the tool's own output, not a harness that exited early) before the rewrite landed; green after,
alongside all four pre-existing `cyclic_nodes` unit tests unchanged.

Strict-mode gates (fmt, clippy, `scripts/test.sh` full workspace) all pass. A code review pass
flagged one minor, non-blocking smell (small duplication between `stack: Vec<&str>` and the new
`frames: Vec<Frame>`, since `Frame.node` mirrors what `stack` already holds at that depth) —
left as is since folding it in would widen this ticket past a same-output rewrite.

Merged into `story-3-catalog-and-release` as `e4be97f`.
