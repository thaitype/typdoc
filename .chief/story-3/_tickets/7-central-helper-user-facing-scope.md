# 7: Does the central JSON-body helper become a user-facing capability? (M-9)

Type: wayfinder:grilling
Status: resolved
Blocked by: None (can start immediately)

**Decider: Mild.** Raised by Aria as M-9, 2026-09-23; answered the same day.

## Question

M-4's words (ticket 6) — *"เพื่อให้ helper รู้ว่าต้อง validate JSON"* — read two ways:

1. A **test-side helper**: `typdoc-core` gains a function the coverage tests call to read a
   catalog document's JSON body into typed data, with no change to any command's behavior.
2. A **user-facing feature**: `typdoc validate` itself checks a catalog document's body against
   its declared body-type field (and reports a rule/exit-code if it's wrong), something a real
   user of `docs/design/catalog/` documents — not just this story's test suite — would see.

The first is what "story 3 removes markdown-reading from code, not migrate everything" would
suggest on its own; the second is a real, if small, new capability of `typdoc validate`. Which
did Mild mean?

## Answer

**M-9, Mild:** (A) — internal. The helper is used by the tests to read catalog documents (a
strict JSON parse of the body, driven by the body-type field); `typdoc validate`/`get` do **not**
learn body types in v0.2.0. That is designed later, together with plain `.json` documents (the
same future work M-6 already named).

This confirms ticket 5's rewiring answer needs no revision: the three test call sites are the
helper's only callers this story, and `pub` (not `pub(crate)`) is still the correct visibility —
not because of a fourth caller that didn't materialize, but because `typdoc-core/tests/rules.rs`
and `typdoc/tests/coverage.rs` were always outside a `pub(crate)` boundary regardless of this
answer.
