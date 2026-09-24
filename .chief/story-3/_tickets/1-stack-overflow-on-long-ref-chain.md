# 1: Root cause and fix for the long-ref-chain stack overflow

Type: wayfinder:research
Status: resolved
Blocked by: None (can start immediately)

## Question

Story 1 left a real crash on the unverified list: a stack overflow on a long ref chain. It has
not been re-verified since. Reproduce it on current `main` (tag `v0.1.0`, commit `62e6335`),
find the actual call path that recurses, and propose a fix approach (an iterative rewrite, a
depth limit with a reported error, or something else) sized for a build ticket. This is a
release blocker for v0.2.0 (Aria, on Mild's authorization) — chart HOW to fix it, not whether.

## Answer

**Root cause:** `visit`, the inner function of `cyclic_nodes` in
`crates/typdoc-core/src/refs.rs:486-512`, is an ordinary recursive depth-first search with no
depth limit. It is called once per unvisited document by `prescan_refs`
(`crates/typdoc-core/src/project.rs:3732-3805`) for every schema field marked `acyclic: true`,
project-wide, on every command that calls `ref_project()` (`validate`, `mv`, `get`, `list` all
reach it, per the call sites at `project.rs:934,943,1298,2444,2610,3523`). `visit` recurses into
every child regardless of whether a cycle exists, so a long **one-way** chain through one
`acyclic` field (A→B→C→…→N, no cycle at all) recurses to depth N just the same as a chain that
closes into a cycle — the crash needs no actual cycle, only length.

**Reproduction:** `cyclic_nodes` is `pub(crate)`, so a temporary unit test in the same module
(`refs.rs`, added and removed for this investigation, never committed) called it directly with a
synthetic one-way chain of N edges (`n0→n1→…→nN`), bypassing the CLI, file I/O, and index
entirely — isolating the defect to the recursion itself. Run via `cargo test -p typdoc-core --lib
refs::tests::<name>` on this machine's default 8MB test-thread stack:

| N | result |
|---|---|
| 5,000 | passes |
| 10,000 | `thread ... has overflowed its stack` / SIGABRT |
| 20,000 / 50,000 / 200,000 | same abort |

The crash threshold sits somewhere between 5,000 and 10,000 chained documents on a default stack
— well within reach of a real, non-pathological documentation project (a changelog, a long
narrative series, or any collection that chains entries with a `next`/`previous`-style `acyclic`
ref), not just an adversarial fixture.

**Recommended fix:** rewrite `visit` as an explicit iterative DFS using a heap-allocated work
stack (a `Vec` of frames tracking each node's position in its child list) instead of the call
stack. Same three-colour algorithm, same output, no behavior change — it only moves the growth
from the (fixed, small) thread stack to the heap, which removes the depth ceiling rather than
relocating it. Sized for a build ticket: the rewrite itself, plus a regression test with a long
one-way chain (e.g. 50,000+ nodes, no cycle) and a long chain ending in a cycle, both asserting
the same output the existing four unit tests already check (two-node cycle, self-loop,
chain-with-no-cycle, cycle-with-a-tail) — those four should keep passing unchanged since nothing
about the algorithm's semantics changes.

**Rejected: a depth limit with a reported error.** It would invent a new "maximum chain length"
concept that appears nowhere else in the design, the limit's value would be arbitrary rather than
principled, and it would fail a correctly-authored large project for no reason the design
actually cares about — `acyclic` exists to catch real cycles, not to cap how long a chain is
allowed to be. The iterative rewrite has no downside and removes the crash outright instead of
turning it into a different, still-somewhat-arbitrary failure.

## Requirements for the build ticket (Aria, verified `refs.rs:486` herself, agreed)

- The regression test is written and seen **RED before the rewrite**, while the crash still
  exists to point at — not written after the fix and merely shown to pass.
- Don't size the test to this machine: 5,000-passes/10,000-aborts is this session's stack, not a
  CI runner's. Use a chain far past any default stack (e.g. 200,000 nodes) and run it on a thread
  with an explicitly small stack (`std::thread::Builder::stack_size`) so it fails the same way on
  every machine, not just this one.
- A stack overflow aborts the whole test binary rather than failing one test. Confirm the red run
  shows up as an actual failure in `scripts/test.sh`'s output, not as a harness that stopped early
  and merely looked short — the gate must be provably able to catch this, not just happen to.
