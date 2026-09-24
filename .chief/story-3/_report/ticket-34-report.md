# Ticket 34 Report

## Ticket

M-21: narrow ticket 30's write-time `refs.acyclic` check so a `set`/`new` is only refused when
this write's own change to an `acyclic` field is what closes the cycle — not merely because the
document being written happens to sit on any cycle at all, for any reason.

## Outcome

done

## The fix

`Project::set_collected` (`crates/typdoc-core/src/project.rs`, around line 955) already had
`before_fields` (on-disk values) and `after_final` (this write's candidate values) computed for
`auto: update`'s own change-detection. The `refs.acyclic` findings ticket 30 filtered to
`finding.path == path` are now filtered further: each `refs.acyclic` finding always carries
`field: Some(field_name)` (set in `prescan_refs`'s `cyclic_nodes` loop), so a finding is kept only
when that named field's value actually differs between `before_fields` and `after_final` —
reusing the existing `fields_map` helper (already used by `fields_changed`) to compare by value,
not by text:

```rust
let before_map = fields_map(&before_fields);
let after_map = fields_map(&after_final);
findings.extend(acyclic.into_iter().filter(|finding| {
    finding.path == path
        && finding
            .field
            .as_deref()
            .is_some_and(|field| before_map.get(field) != after_map.get(field))
}));
```

A field this write never touches (e.g. `set WF-1 title=…` leaving `blocked_by` alone) is now
skipped entirely, regardless of whether it already sits on a pre-existing cycle. A field the write
does change is still checked exactly as before: if the post-write scan finds `path` on a cycle
through that field, it's refused; if not (the write broke the cycle), nothing is added — unchanged
from ticket 30.

`validate_new_candidate` (the `new` side) was left completely untouched, per the ticket's
instruction: `new` has no "before" value for any field it sets — the document doesn't exist yet —
so every `acyclic` field `new` gives it is new by construction and should keep counting as
"changed." Its filter is still the plain `finding.path == path`, exactly as ticket 30 built it.

## Verification

**Red-then-green, the ticket's exact repro**
(`crates/typdoc/tests/set.rs::set_untouched_by_its_own_acyclic_field_succeeds_despite_a_pre_existing_cycle`):
`WF-1` and `WF-5` are written directly as fixture files on disk (bypassing the CLI's own
write-time check) already sitting on a `blocked_by` cycle; then `set WF-1 title=Updated`.
- Red (before the fix): exit 2, `refs.acyclic` in `details` — wrongly refused, confirmed before
  touching `project.rs`.
- Green (after the fix): exit 0, `title` written, and a following `validate --json` still exits 2
  and still reports `refs.acyclic` for the same pre-existing cycle (unaffected by this write, as
  required — `validate`'s own behavior is untouched).

**Ticket 30's own tests, re-run explicitly, all still pass unchanged:**
- `set_refuses_a_write_that_closes_an_immediate_cycle_on_an_acyclic_field`
- `set_refuses_a_write_that_closes_a_longer_chain_into_a_cycle`
- `set_is_not_refused_by_an_unrelated_pre_existing_cycle_elsewhere`
- `new_refuses_a_write_that_would_close_a_cycle_with_an_existing_document`
- `new_is_not_refused_by_an_unrelated_pre_existing_cycle_elsewhere`

**New test cases added (all green):**
- `set_untouched_by_its_own_acyclic_field_succeeds_despite_a_pre_existing_cycle` (the repro above).
- `set_on_an_unrelated_field_succeeds_when_no_cycle_exists_at_all` (the ticket's explicit
  "breaks then unrelated write" case).
- `set_refuses_a_write_that_closes_a_new_cycle_through_a_different_pair_on_the_same_field`: a
  pre-existing `WF-1`↔`WF-5` cycle plus `WF-5 -> WF-10` (a dead end); `set WF-10
  blocked_by=WF-1` closes a second, three-document cycle `WF-1 -> WF-5 -> WF-10 -> WF-1` — refused,
  since `WF-10`'s own field change is what closes it.

**Gates:**
- `cargo fmt --check`: clean.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean, no changes needed.
- `TMPDIR=/home/thw-home/.cache/typdoc-tmp scripts/test.sh`: **1023 tests passed across 56
  suites, 0 failed.**

## Notes

- `docs/reference/commands.md` (line ~244-247) already described the check narrowly ("a ref that
  would close a cycle on an `acyclic` field") — it never claimed the over-broad behavior, so it
  was left unchanged.
- `skills/typdoc/references/commands.md` (lines 182-186) did overstate it: "In 0.2.0 a document
  already on a cycle refuses any other write until the cycle is broken" was the exact over-broad
  behavior this ticket removes. Corrected to describe the narrowed rule: refused only when the
  write itself changes an `acyclic` field to a value that closes a cycle; a write that never
  touches an `acyclic` field succeeds even on a pre-existing cycle, which `validate` still reports
  on its own.
- No exit code, finding shape, or refusal message changed — only which writes trigger the same
  `refs.acyclic` refusal shape changed, as the ticket anticipated.
