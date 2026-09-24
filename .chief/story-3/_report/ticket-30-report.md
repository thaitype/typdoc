# Ticket 30 Report

## Ticket

M-18: `set`/`new` must refuse a write that closes a cycle on an `acyclic` ref field at write
time, not only on a later `validate` run — a real bug found running a release build.

## Outcome

done

## The fix

`Project::set_collected` and `Project::validate_new_candidate` (`crates/typdoc-core/src/
project.rs`) both used to call `self.ref_project()?` and throw away its `refs.acyclic` half
(`let (ref_project, _acyclic) = ...`), because that scan reads every document from disk — before
the write's own `candidate` text exists there — so a cycle it found would be evidence about the
pre-write state, not about `candidate`.

The fix makes the evidence correct instead of discarding it:

- `Project::prescan_refs` gained a `candidate: Option<(&str, &Indexed, &str)>` parameter
  (`path`, its index entry, its candidate text). When given, the one document at `path` is
  scanned from `candidate` instead of from disk; if `path` is not yet in `self.index` at all (a
  `new` write — not indexed, file doesn't exist yet), it is scanned as one extra document on top
  of the ordinary walk. With `candidate: None` (every other caller), both branches fall back to
  exactly the original behaviour.
- The per-document scan body was pulled out into a new `Project::prescan_one` helper (shared by
  the ordinary walk and the one extra `new` call), taking a `PrescanAccum { moved, edges }`
  bundle instead of two separate `&mut` maps (kept `prescan_one` under clippy's
  `too_many_arguments` threshold).
- A new `refs::Candidate` + `refs::resolve_one_for_candidate` (`crates/typdoc-core/src/refs.rs`)
  resolve a ref that names the candidate's own key (in its namespace) or path directly, without
  touching `Ctx::index` or the disk. This is the non-obvious half of the fix: a `new` candidate
  is not indexed and its file does not exist yet, so without this, nothing already on disk that
  points at its future key/path would ever resolve during the scan — making it structurally
  impossible for `new` to ever detect a cycle closed with a document that already points at the
  one being created. A `set` candidate's identity is already real and already resolves through
  `Ctx::index` on its own, so this never changes anything for it — the check simply never
  matches before falling through to plain `resolve_key`/`resolve_path`.
- `Project::ref_project` was split into `ref_project()` (unchanged signature, calls
  `prescan_refs(&codes, None)`) and a new `ref_project_for_candidate(path, entry, candidate)`
  (calls `prescan_refs(&codes, Some(...))`), both routed through a private `ref_project_inner`.
- `set_collected` and `validate_new_candidate` now call `ref_project_for_candidate`, filter the
  returned `acyclic` findings to `finding.path == path` (an unrelated cycle elsewhere the scan
  also, correctly, still finds is not this write's to refuse), and extend `findings` with the
  result before the existing `if findings.iter().any(|f| f.level == Severity::Error) { return
  Err(...) }` gate — no new refusal logic, exactly as the ticket specified.

`refs::cyclic_nodes` was not touched or reimplemented — it is reused as-is on the (correctly
substituted) edge list, exactly as the ticket required.

## Verification

**Red-then-green, repro test** (`crates/typdoc/tests/set.rs::
set_refuses_a_write_that_closes_an_immediate_cycle_on_an_acyclic_field`, the exact ticket repro:
`WF-1` has `blocked_by: [WF-5]`, `set WF-5 blocked_by=WF-1`):
- Red (fix stashed out): `left: 0, right: 2` — exited 0, wrote the file, exactly the bug
  described.
- Green (fix restored): exit 2, `refs.acyclic` in `details`, `WF-5.md`'s bytes unchanged
  (compared before/after, not just the exit code).

**Red-then-green, `new` cycle test** (`crates/typdoc/tests/new.rs::
new_refuses_a_write_that_would_close_a_cycle_with_an_existing_document`): `WF-1` has
`blocked_by: [WF-2]` (dangling, since `WF-2` doesn't exist), state's `last: 1` so `new WF` is
about to allocate exactly `WF-2`; giving it `blocked_by: [WF-1]` closes the cycle the moment it's
created.
- Red (fix stashed out): exit 0, document created.
- Green (fix restored): exit 2, `refs.acyclic` in `details`, no `WF-2.md` created, state file's
  `last` not burned (this is the test that specifically exercises the `refs::Candidate` /
  `resolve_one_for_candidate` mechanism — without it this test fails even with the rest of the
  fix in place, since `WF-1`'s forward reference to the not-yet-existing `WF-2` would never
  resolve).

**Other required tests, all green:**
- `set_refuses_a_write_that_closes_a_longer_chain_into_a_cycle` (3-document indirect chain via
  `set`).
- `set_is_not_refused_by_an_unrelated_pre_existing_cycle_elsewhere` and
  `new_is_not_refused_by_an_unrelated_pre_existing_cycle_elsewhere` (the "no false refusal"
  regression: an unrelated cycle on documents this write doesn't touch does not block it).

**Read-only callers unaffected — shown, not just asserted:** grepped every call site of
`ref_project`/`ref_project_for_candidate`/`prescan_refs` in `project.rs` after the change. Only
two call sites ever pass a candidate (`set_collected` line ~955, `validate_new_candidate` line
~1322). The only other two call sites (`Project::validate`'s two branches, ~2463 and ~2629) call
plain `ref_project()` → `ref_project_inner(None)` → `prescan_refs(&codes, None)`: with
`candidate: None`, the per-entry `match` always takes its `_ =>` (disk-read) arm, the
extra-document block is skipped (`if let Some(...) = candidate`), and the cycle-finding lookup's
`candidate.and_then(...)` is always `None` — byte-for-byte the original code path. `get`, `refs`
and `toc` were also checked: none of them call `ref_project`/`prescan_refs` at all (they resolve
refs through their own, separate logic), so they are trivially unaffected.

**Gates:**
- `cargo fmt --check`: clean.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean (one `too_many_arguments` hit
  during development, fixed by bundling `prescan_one`'s two accumulator maps into `PrescanAccum`
  rather than suppressing the lint).
- `TMPDIR=/home/thw-home/.cache/typdoc-tmp scripts/test.sh`: **1006 tests passed across 56
  suites, 0 failed.**

## Notes

- The `refs::resolve_one_for_candidate` addition goes slightly beyond what the ticket's "fix"
  section describes in words (it only mentions substituting the candidate's own text) — it was
  necessary once tracing through why the required `new` test case is possible at all: without a
  way for another on-disk document's forward reference to a not-yet-created path/key to resolve
  during this one scan, a `new` write could never structurally close a cycle with an existing
  document. Verified both directions carefully (`new`'s cycle test fails without this piece even
  with the rest of the fix in place; `set`'s tests are unaffected by it either way since a `set`
  candidate's identity is already real).
- Filtering the candidate-aware `acyclic` findings by `finding.path == path` means a `set`/`new`
  touching a document already sitting on an unrelated-to-this-write cycle *through its own
  field* (unchanged by this write's `sets`) would also be refused, not just a write that newly
  creates the cycle. This matches the ticket's literal instruction ("check whether the resulting
  acyclic findings include one for the path being written... extend `findings` with it") and
  the design's plain-language rule ("no cycle forms on `acyclic` fields") — flagging it here as a
  deliberate reading, not an oversight, in case Mild wants the narrower "only if this write
  itself changed the cycle" behavior instead.
