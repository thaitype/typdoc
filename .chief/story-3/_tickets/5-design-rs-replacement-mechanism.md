# 5: Design.rs replacement — the structured document(s), the central JSON-body helper, and rewiring dependents

Type: wayfinder:grilling
Status: resolved
Blocked by: None (can start immediately) — re-charted 2026-09-23 after M-1 (ticket 2) reshaped
the core; no longer blocked on it now that it's resolved. The one open sub-question (the
helper's user-facing scope) split out to ticket 7 (M-9, waiting on Mild) rather than blocking
this ticket's own resolution.

**Re-charted 2026-09-23.** The original question (a bare JSON file, tables generated into
`design.md`) is obsolete: M-1 rules out any code-to-markdown relationship at all, in either
direction. The replacement, per Mild's decisions in `.chief/story-3/brief.md`:

- The five sets `design.rs` currently extracts move into **four** `docs/design/catalog/*.md`
  documents (`CAT`, folder resolved: ticket 6/M-4), one per domain concept, not per current
  caller (Aria, 2026-09-23 — a caller reading two concepts today, like `coverage.rs` reading both
  commands and exit codes, is a fact about that test file, not about the domain):
  1. **rules** — see the reshaped shape below.
  2. **commands** — the command names.
  3. **exit codes** — the exit code set.
  4. **frontmatter losses** — the loss rows, prose kept short (ticket 3).
- **Rules document reshaped, not just relocated** (Aria): not two lists (always-on/configurable)
  the way `design.md`'s two tables and `rule_ids`/`always_on_rule_ids`/`configurable_rule_ids`
  are today. **One list of rule entries, each with a field saying whether it is configurable.**
  Two lists let the same id sit in both or in neither and still parse; one list with a field makes
  that state impossible to write.
  Each catalog document's body is JSON (format decided: ticket 3).
- Every catalog document carries a frontmatter field declaring its body type (e.g. `json`) —
  ticket 6/M-4's added requirement. The central helper validates the body according to that
  field, **never by inferring anything from the path** (so `docs/design/catalog/` is a
  human-organizing convention, not the thing that tells the helper what to do).
- Production code (**not** just tests — M-6 says "a central helper *in typdoc*", i.e.
  `typdoc-core`, not `typdoc-testkit`) gains a central helper for "a typdoc document whose body
  is JSON", used to read these documents back into typed data. Supporting plain `.json` documents
  instead of md-with-JSON-body is explicitly future work (M-6) — not this ticket.
- `design.md` and the old phase-1/phase-2 registries are **moved** (`git mv`) to
  `docs/archived-design/` — no longer at their old paths, never edited again there — and
  separately **copied** to `docs/migrating-design/` (a working copy emptied progressively as
  content moves to the new layout — M-7). Moving every piece of content out this story is
  explicitly **not** required; only removing all markdown-reading from code is.
- Prose explanation of these same rules (human-readable, code must not read it) becomes a
  *separate* kind of typdoc document under `docs/design/spec/` (`SPC`, ticket 6/M-4).
- This repo needs a `.typdoc/` project set up to manage these documents via the typdoc CLI itself
  (M-5, using the `v0.1.0` tag; skip and defer if a typdoc bug blocks a step).

## Question

**Rewiring.** All three current call sites (`typdoc-core/src/frontmatter.rs`'s test module,
`typdoc-core/tests/rules.rs`, `typdoc/tests/coverage.rs`) are dev-dependency-only uses of
`typdoc_testkit::design::*` (verified: `typdoc-testkit` is a `[dev-dependencies]` entry in both
`typdoc-core` and `typdoc`'s `Cargo.toml`, and the frontmatter.rs call site sits inside a
`#[cfg(test)] mod tests`) — today's design.rs machinery serves test coverage, not a runtime code
path. Does the replacement stay scoped the same way (test-only consumption of the central
helper)?

Whether the helper *also* gains a user-facing role (`validate` checking a catalog body against
its declared type) is a separate, open question — ticket 7 (M-9, waiting on Mild). Whatever that
answer, the three test call sites above still need to read the four new catalog documents instead
of `design.md`, so this ticket resolves that base case now and ticket 7 is additive on top of it,
not a blocker to it.

## Answer

Yes — the replacement is test-only, matching today's shape. `typdoc-core/src/frontmatter.rs`'s
test module, `typdoc-core/tests/rules.rs`, and `typdoc/tests/coverage.rs` each stop calling
`typdoc_testkit::design::*` and instead read the relevant catalog document(s) (rules; commands;
exit codes; frontmatter losses) through the new central helper, deserializing into a typed struct
per document instead of parsing markdown tables.

**Visibility, corrected from an error in this ticket's own Q7 draft (caught by Aria):** the
helper cannot be `pub(crate)` in `typdoc-core`. `typdoc-core/tests/rules.rs` is a Rust
integration test — a separate crate that only sees `typdoc-core`'s public API — and
`typdoc/tests/coverage.rs` lives in the `typdoc` crate entirely. Both need the helper to be
`pub`, exported from `typdoc-core`, regardless of how ticket 7 answers the user-facing-or-not
question. `typdoc-core/src/frontmatter.rs`'s test module is the one call site that could reach a
`pub(crate)` item, being in the same crate — but the helper's visibility is set by its widest
caller, not its narrowest, so `pub` it is.

Whether the helper is called *only* from these three test sites (ticket 7 answers "test-side") or
also from `validate`'s own command logic (ticket 7 answers "user-facing") does not change this
ticket's answer — it only adds a fourth, non-test caller on top.
