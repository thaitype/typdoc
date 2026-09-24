# Ticket 2 Report

## Ticket
Excluding a namespace with existing state must not break project load (the `config.state-orphan`
load-fatal conflict found during contract). Also in scope: fix the stale `NoProjectAt`/
`NoProject` error text left over from ticket 3's folder-based discovery.

## Outcome
done

## Decision
- **Issue (scope item 2's own note):** the ticket text claimed there wasn't a test pinning the
  old `NoProjectAt`/`NoProject` message's exact wording. Verified while there: partially true —
  `crates/typdoc/tests/config.rs`'s `no_project_error()` did assert `.contains(".typdoc/config.json")`,
  a real but partial lock, not a full-sentence pin.
- **Options considered:** none needed — straightforward to fix once found.
- **Chosen:** updated that helper's assertion to the new wording, and added a dedicated test
  (`the_no_project_message_reads_exactly_no_project_folder_not_no_project_file`) pinning the
  complete literal sentence for both `NoProject` and `NoProjectAt`.
- **Judgment call (ticket said wording was build's own call):** reworded to `"no .typdoc/ in
  {from} or above it"` / `"{dir} has no .typdoc/"` — structurally parallel to the old message,
  consistent with the codebase's existing trailing-slash `.typdoc/` folder convention.

## Notes
- `namespaces::Resolved` gains `excluded: BTreeSet<String>` (names matched then removed by a
  later `!`); threaded through `Config`/`state::orphans`'s `known` set. True-orphan edge case
  (folder gone, `!` entry matches nothing) still fires `config.state-orphan` as designed.
- All 3 contract-named gate tests confirmed genuinely red pre-fix (exit 2 on `config.state-orphan`
  for tests 1/2; test 3, the true-orphan control, was already green pre-fix as expected) and
  green after. 3 additional unit tests added on `resolve()`'s new `excluded` field semantics.
- Full workspace suite green (`--features typdoc/test-stand-in`), clippy/fmt clean,
  `/chief-review-code` clean both axes. Checked (and independently re-checked at merge) for
  story/ticket references in code comments per the standing rule — none found; pre-existing
  "M-24"/"decision N" citations are an established codebase convention, not the same thing.
- Commit `0bf0a04` on `story-4-namespace-ignore`.
