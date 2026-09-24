# 34: M-21 — narrow ticket 30's write-time acyclic check to only refuse a NEW cycle

Type: implementation
Status: claimed
Blocked by: None (ticket 30 already merged; this narrows its behavior)

**Mild's decision (M-21), 2026-09-24, relayed by Aria — this is the narrow reading of the
question ticket 30's own report flagged, not decided there.** Design: "no cycle forms" — read
literally, this is about a write's own *effect* (does it form a new cycle), not a state
invariant (is this document, right now, sitting on any cycle at all, for any reason).

**Repro:** a document already sits on a cycle (say `WF-1 blocked_by WF-5` and `WF-5 blocked_by
WF-1`, both already on disk, however that happened). `set WF-1 title=…` — a write that never
touches `blocked_by` at all — must **succeed**. Today (post-ticket-30), it is wrongly **refused**,
because ticket 30's fix flags `refs.acyclic` for `path` whenever `path` sits on ANY cycle in the
post-write scan, regardless of whether this particular write is what put it there. `validate`
still correctly reports the pre-existing cycle either way — that's unaffected and correct.
Removing a ref that breaks a cycle must also stay allowed (this already works today, don't
regress it: the post-write scan finds no cycle once the breaking edge is gone).

## Where this lives today (ticket 30's code, needing narrowing)

`crates/typdoc-core/src/project.rs`'s `set_collected` and `validate_new_candidate` — read ticket
30's own report (`.chief/story-3/_report/ticket-30-report.md`) first for the exact shape it
built: `ref_project_for_candidate`/`prescan_refs`'s `candidate` parameter substitutes this write's
own not-yet-on-disk text for the document being written, then the resulting acyclic findings are
filtered to `finding.path == path` and added to `findings` unconditionally whenever `path`
appears in them at all. That's the over-broad part: `path` can appear in the post-write cyclic
set for a reason that has nothing to do with this write (an acyclic field the write never
touched, cyclic before the write and still cyclic after, unrelated to whatever this write's own
`sets` actually changed).

## The fix

Only flag a `refs.acyclic` finding for an acyclic field that this write's own `sets` actually
**changed the value of**. Concretely, per acyclic field `F` on the document's schema:

1. Compare `F`'s value in `before_fields` (what's on disk now) against `after_final` (what this
   write's candidate would produce). If `F`'s value is unchanged, skip `F` entirely for this
   check — no cycle-detection work needed, and no finding possible from it, regardless of whether
   `F` already sits on a pre-existing cycle. This is what makes `set WF-1 title=…` (touches no
   acyclic field) skip the check completely and always succeed on this axis.
2. If `F`'s value DID change, run the candidate-substituted cyclic scan for `F` (reuse ticket 30's
   existing machinery — `ref_project_for_candidate`/`prescan_refs`'s candidate substitution,
   `refs::cyclic_nodes`) and check whether `path` is now in `F`'s post-write cyclic set. If so,
   this write's own change to `F` is what closes the cycle (whether it's brand new, or the same
   two documents via the same field re-closing after a prior break, or a different pair of
   documents than before through the same field) → this counts as "forms a cycle" → include the
   `refs.acyclic` finding.
3. This is correct for the "breaking a cycle" case too: if `F` changed and the post-write scan for
   `F` finds NO cycle, nothing is added — already the case today, don't regress it.
4. `before_fields`/`after_final` are already computed and available in `set_collected` (see its
   existing `fields_changed` usage nearby, ticket 30's own diff, or the diff from whichever ticket
   built `auto: update`'s change-detection) — reuse the same values, don't recompute them. For
   `validate_new_candidate`/`new`'s two forms, there is no "before" (the document doesn't exist
   yet) — every acyclic field `new` sets should be treated as "changed" (there's no prior value to
   compare against, so every acyclic ref value `new` gives the document is new by construction),
   which is already how ticket 30 built the `new` side; only `set`'s side needs the
   before/after-comparison narrowing this ticket adds.

## Tests

1. **Red first** (Mild's instruction): the exact repro — a pre-existing cycle (built directly on
   disk in the test fixture, bypassing `set`/`new`'s own write-time check, e.g. by writing the
   fixture files directly rather than via the CLI, since the CLI itself would now refuse to create
   one), then `set WF-1 title=…` — confirm this FAILS on the current (post-ticket-30, pre-this-
   ticket) code, i.e. confirm it's wrongly refused today, before making any change.
2. After the fix: the same repro succeeds (exit 0, `title` written), and a subsequent `validate`
   run on the same project still reports the pre-existing `refs.acyclic` cycle (unaffected,
   unfixed by this write — this ticket does not change `validate`'s own behavior at all).
3. Confirm ticket 30's own original repro (`set WF-5 blocked_by=WF-1` when `WF-1` already has
   `blocked_by: WF-5`) is STILL refused — this ticket narrows, it does not remove, ticket 30's
   check. Re-run ticket 30's own tests (the exact repro, the `new`-closes-cycle case, the longer
   3-document chain case) and confirm all still pass unchanged.
4. A "re-closes via a different pair" case: `F` already cyclic (A↔B), a write changes `F` on a
   THIRD document C to point into that same cycle in a way that closes a new A-B-C cycle — refused
   (the field changed, and C is now on a cycle through F).
5. A "breaks then the write is unrelated" case: cycle already broken (no cycle on disk at all),
   `set` on an unrelated field of a formerly-cyclic document — succeeds (already covered by case 1
   really, but worth its own explicit assertion since it's the literal Mild-given example).
6. Full gate run: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
   `TMPDIR=/home/thw-home/.cache/typdoc-tmp scripts/test.sh`.

## Docs

If this changes any user-visible text (it shouldn't — the exit code, finding shape, and message
for an actual refusal are unchanged; only which writes trigger a refusal changes), update
`docs/reference/commands.md` and `skills/typdoc/references/commands.md` (ticket 33's new doc
locations — `docs/commands.md`/`docs/projects.md` no longer exist, replaced by
`docs/reference/`) to match. Check both files for any prose describing ticket 30's acyclic-check
behavior and correct it if it currently overstates the check as unconditional.

## Done

- A write that does not change any `acyclic` field's value never triggers a `refs.acyclic`
  refusal, regardless of whether the document already sits on an unrelated pre-existing cycle.
- A write that changes an `acyclic` field's value to something that closes a cycle (new pair,
  same pair re-closing, or a third document joining an existing cycle) is still refused.
- A write that changes an `acyclic` field's value to break a cycle still succeeds.
- Ticket 30's own tests all still pass.
- `validate`'s own reporting of pre-existing cycles is completely unaffected.
- All three gates green.
