# 24: `shell_examples.rs` (both crates) extracts and runs design.md's own examples — a bigger M-1 violator than `design.rs` was

Type: wayfinder:grilling
Status: open
Blocked by: None (can start immediately)

**Found during a broader sweep, 2026-09-23, before ticket 12's build started** — while fixing
the scope Aria caught for `fixtures.rs` (this ticket's sibling fix in ticket 12), a full `rg`
across the workspace for every reader of `docs/design/design.md` turned up a much larger one that
no earlier ticket named. Reporting before any build proceeds, per the same discipline that caught
`fixtures.rs`: don't let the goal's final `rg` sweep be what catches this.

## What was found

- `crates/typdoc-testkit/src/shell_examples.rs` (226 lines): `examples()` walks `design.md` line
  by line, extracting every fenced-code line and inline code span that looks like a shell
  invocation (`typdoc ...`, `TYPDOC_...=`, or a `--flag`). `quoting_paragraph_spans()` extracts
  every code span from one specific paragraph by name. Both parse markdown structure (fences,
  inline spans, a paragraph found by its bold-text marker) to produce data other code consumes.
- `crates/typdoc/tests/shell_examples.rs` (605 lines): calls `design_text()` and the two functions
  above, then actually spawns a real shell (`sh`, `bash`) to run every extracted example against
  the built `typdoc` binary, asserting the output/args a stand-in binary recorded match a
  hand-declared or automatically-derived expected value.
- The file's own doc comment states the exact rationale M-1 rejects: "so the shell harness and
  the document share one list instead of a second one kept in test code that could drift from
  it." Mild's rule: *"no program reads markdown to extract spec as machine data — not to import
  it, not to check against it... Existing code that does this is debt to remove, not an exemption
  because it works."* This is that debt, at a much larger scale than the five sets `design.rs`
  extracted — it covers every worked shell example in the entire document, not five specific
  tables.

`fixtures.rs`'s `design_text()` (ticket 12's other fix) is the shared plumbing both `design.rs`
and this mechanism call through — removing only `design.rs`'s three narrow extractors while
leaving `shell_examples.rs` calling `design_text()` would leave `design.rs` gone but the
underlying violation, and the bulk of the code doing it, still standing.

## Question

This test harness has real value — it's what "every shown example was run for real against a
built binary" (the standing practice this story's own tickets 10/13/22 rely on) actually checks,
automatically, for the *whole* document, not just the examples someone remembered to re-run by
hand. Removing the auto-extraction loses that automatic coverage unless it's replaced with
something. Options, not decided here:

1. **Hand-list every example directly in the test file.** No markdown reading at all; the "one
   list" property is lost — a new example added to `design.md`/`docs/design/spec/` and not added
   here silently isn't tested. Simplest, most literal compliance with M-1, weakest coverage
   guarantee.
2. **A fifth structured document** (`docs/design/catalog/shell-examples.md` or similar,
   JSON-body, read through ticket 11's helper) holding the list of examples as data, with
   `docs/design/spec/`'s prose showing them for humans. Keeps the "one list, can't drift"
   property, fits M-1 (JSON is the source of truth, nothing reads the prose as data) — but the
   examples currently live embedded in flowing prose throughout `design.md`; extracting them into
   a separate structured list changes how they're authored (no longer just "write an example in
   the doc and the test finds it automatically").
3. **Something else** not yet proposed.

Also open: whether this belongs in this story at all, given its scale, or is worth carving out —
raised here rather than assumed, since M-1 doesn't say "eventually," but this story's own goal
(ticket 12's "done means design.rs is gone and nothing reads design.md as data") is not fully
true while this stands, however it's resolved.

## Answer

<filled in on resolve>
