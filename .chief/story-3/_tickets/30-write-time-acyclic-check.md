# 30: M-18 — `set`/`new` must refuse a write that forms a cycle on an `acyclic` field

Type: implementation
Status: resolved
Blocked by: None (can start immediately)

**Mild's decision (M-18), 2026-09-24, relayed by Aria, found running a real release build.**
This predates story 3 — the design already states the right behavior (§Refs, "Write-time checks
(`new`, `set`): … no cycle forms on `acyclic` fields") — this is a bug fix, not a new decision.

**Repro:** `WF-1` already has `blocked_by: WF-5`. Running `set WF-5 blocked_by=WF-1` (which would
close the cycle `WF-1 → WF-5 → WF-1` on the `acyclic` field `blocked_by`) exits **0** today —
only a later `validate` run reports `refs.acyclic`. It must instead exit **2**, write nothing,
and report `refs.acyclic` in `details`, at write time, same command.

## The bug, precisely

`crates/typdoc-core/src/project.rs`:
- `Project::set_collected` (search `fn set_collected`, around line 816): line ~943,
  `let (ref_project, _acyclic) = self.ref_project()?;` — the acyclic-cycle findings are computed
  from a fresh whole-project scan and then **thrown away** (`_acyclic`). The comment right above
  it (lines ~934-942) explains why this was left unchecked: the scan reads every document **from
  disk**, before this write's `candidate` text exists there, so a cycle it finds is evidence about
  the pre-write state, not about `candidate` — checking it directly would be wrong in both
  directions (a false refusal for an unrelated pre-existing cycle elsewhere, or a false pass for
  a cycle this very write just created, since the on-disk copy of this document is still the old
  one). That reasoning is correct as far as it goes, but the conclusion it draws — leave it
  unchecked and let a later `validate` catch it — is wrong per the design. The fix is to make the
  *evidence* correct (scan with this write's candidate substituted in), not to give up.
- `Project::validate_new_candidate` (search `fn validate_new_candidate`, around line 1274): the
  identical pattern, `let (ref_project, _acyclic) = self.ref_project()?;` around line 1298, used
  by both of `new`'s forms.

`Project::ref_project` calls `Project::prescan_refs` (search `fn prescan_refs`, around line
3732), which reads **every** document in `self.index` from disk (`fs::read_to_string(&entry.file)`),
builds one edge list per `acyclic`-marked field name across the whole project, and runs
`refs::cyclic_nodes` (in `crates/typdoc-core/src/refs.rs`) on each field's edges, producing one
`Finding` per document that sits on a cycle, per field.

## The fix

Give `prescan_refs` (or a new, small wrapper around it — your judgment on the cleanest shape) a
way to substitute one document's fields with the write's own `candidate` fields instead of
reading that document from disk, for exactly the one path being written. Then, in both
`set_collected` and `validate_new_candidate`, call this candidate-aware scan instead of the
plain one, and check whether the resulting acyclic findings include one for the path being
written. If so, that's a real `refs.acyclic` finding for `candidate` — extend `findings` with it
(the same `findings` vector the existing `check_refs` results already go into), exactly like any
other finding this function already produces. The existing gate
(`if findings.iter().any(|f| f.level == Severity::Error) { return Err(Error::Invalid { findings }); }`
in `set_collected`, and whatever `new_coded`/`new_uncoded` do with `validate_new_candidate`'s
return value) already refuses the write correctly once the finding is present — you do not need
new refusal logic, only to make the finding actually appear when it should.

Think carefully about correctness in both directions your own comment-reading already flagged:
- A cycle that exists elsewhere in the project, untouched by this write, must NOT cause this
  write to be refused (no false refusal).
- A cycle this write's own field value creates — whether directly (the immediate edge just
  written closes a loop) or indirectly (through a longer chain via other documents, same
  `acyclic` field) — MUST be caught (no false pass). Ticket 1/9's iterative `cyclic_nodes` walk
  already handles arbitrarily long chains without a stack-overflow risk; reuse it as-is, don't
  reimplement cycle detection.
- `set_loose` (documents outside every collection — no schema, so no `acyclic` field is even
  possible) needs no change; don't touch it.
- Every OTHER caller of `Project::ref_project`/`prescan_refs` (the read-only paths: `validate`,
  `get`, `refs`, `toc`, etc. — search for all call sites) must be completely unaffected: they
  should keep scanning the real on-disk state, with no candidate substitution. Do not change
  `prescan_refs`'s existing behavior for any caller that doesn't explicitly ask for a
  substitution.

## Tests

1. **Red first**: write the exact repro as a test against the CURRENT code, confirm it fails
   (exits 0 today, test expects exit 2) before making any fix — this is Mild's own instruction
   ("test red first").
2. After the fix: the same test passes — exit 2, nothing written to `WF-5`'s file (confirm via
   reading the file back, not just the exit code), and a `refs.acyclic` finding present in
   `details` (check the exact JSON shape `--json` uses for a write refusal elsewhere, e.g.
   `validate`'s or another write command's `Error::Invalid` rendering, and match it).
3. A second cycle test through `new` (not just `set`) — creating a new document whose own
   `acyclic` field value would close a cycle with existing documents — refused the same way.
4. A longer chain (3+ documents) closing a cycle through a write, to exercise the indirect case,
   not just the immediate two-document case from the repro.
5. A regression test: a write that does NOT create any cycle, in a project that already has an
   unrelated, pre-existing cycle elsewhere (through the same `acyclic` field, on documents this
   write does not touch) — succeeds normally, unaffected by that unrelated cycle. This is the
   "no false refusal" case your own investigation must get right.
6. Confirm every existing test in `crates/typdoc-core/tests/` and `crates/typdoc/tests/` touching
   `refs.acyclic`, `set`, or `new` still passes — full gate run.
7. `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
   `TMPDIR=/home/thw-home/.cache/typdoc-tmp scripts/test.sh`.

## Done

- `set`/`new` refuse (exit 2, nothing written, `refs.acyclic` in `details`) any write that would
  close a cycle on an `acyclic` field, whether the new edge is the write's own or completes a
  longer existing chain.
- A write that creates no cycle succeeds normally, including in a project with an unrelated
  pre-existing cycle elsewhere.
- Every read-only caller of `ref_project`/`prescan_refs` is unaffected.
- All three gates green.
