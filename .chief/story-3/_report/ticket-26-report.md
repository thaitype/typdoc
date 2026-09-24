# Ticket 26 Report

## Ticket

Restore the `config.*` rule-id coverage that was silently dropped when `rules.rs`'s test was
rewired off `design.md` onto the catalog documents: add a fifth catalog document,
`docs/design/catalog/config-errors.md`, holding the 20 `config.*` config-file-validation-error
ids and their "Reported when" text (design.md's old third "Validation rules" table), and check
it against `typdoc_core::rules::RULES`'s `config.*` subset in `crates/typdoc-core/tests/rules.rs`,
both directions, restoring the goal's "coverage continues unchanged" criterion for the full
43-id set (23 + 20).

## Outcome

done

## Verification

**Drift proofs (step 9, done for real, not assumed):**

1. Planted a made-up catalog-only id (`config.made-up-drift-proof`) in
   `docs/design/catalog/config-errors.md` with no matching code in `RULES`. Ran
   `cargo test -p typdoc-core --test rules every_config_error` -- went red with:
   `"the design names config error config.made-up-drift-proof, and it is in neither RULES's
   config.* subset nor unimplemented_rules"`. Reverted (removed the planted line by hand, since
   the file is new/untracked); re-ran -- green (3/3 passed).
2. Planted a made-up id (`config.made-up-drift-proof`) directly into `RULES` in
   `crates/typdoc-core/src/rules.rs`. Same test went red with:
   `"config error config.made-up-drift-proof is in RULES's config.* subset and the design does
   not name it"`. Reverted (`git diff` confirmed byte-for-byte identical to before the plant);
   re-ran -- green (3/3 passed).

**Id-set check (independent of the Rust test):** extracted `RULES`'s `config.*` subset (20 ids)
and `config-errors.md`'s ids (20 ids) with a standalone script -- exact match, no extras either
side.

**`typdoc validate`:** built the real binary (`cargo build -p typdoc --release`) and ran
`typdoc validate --json` against this repo's own `.typdoc/` project: 11 documents checked (5
catalog + 6 spec, including the two new ones), zero findings. Used the same binary to author
`docs/design/spec/SPC-6.md` (`typdoc new SPC ...`), which correctly bumped
`.typdoc/state/default.json`'s `spec.last` from 5 to 6 and allocated `SPC-6` -- confirmed via
`typdoc refs docs/design/catalog/config-errors.md --json` that `explained_by: [SPC-6]` resolves.

**Gates, all green:**
- `cargo fmt --check` -- clean.
- `cargo clippy --workspace --all-targets -- -D warnings` -- clean (exit 0; one pre-existing,
  unrelated `clippy.toml` config warning about `chrono::Local::now`'s disallowed-methods entry
  appears in `-p typdoc-core` output regardless of this ticket's changes and does not fail the
  `-D warnings` build).
- `TMPDIR=/home/thw-home/.cache/typdoc-tmp scripts/test.sh` -- **999 tests passed across 56
  suites** (up from 998 before this ticket's one new test in `rules.rs`).

**Strict-mode `/chief-review-code`** ran as two parallel sub-agents against `git diff HEAD`:
- Standards axis: no `.chief/_rules/_standard/` exists in this worktree, so review rested on the
  ticket brief, sibling-file conventions and the Fowler smell baseline. One finding acted on:
  the `reported_when` field was suppressed for `dead_code` with `#[allow(...)]`; changed to
  `#[expect(dead_code, reason = "...")]` to match the project's dominant "guaranteed-to-fire
  lint" idiom. A second finding (mild struct-shape duplication between `RulesCatalog`/
  `RuleEntry` and `ConfigErrorsCatalog`/`ConfigErrorEntry`) was left as-is per the reviewer's own
  note -- extracting a shared helper for two call sites would be premature.
- Spec axis: one finding acted on -- `validation_rule_ids()`'s own doc comment still said the
  `config.*` third table "the catalog does not hold" after the catalog gained
  `config-errors.md`; updated to point at the new document and the new test. Everything else
  checked out: all 20 `reported_when` strings verified word-for-word against
  `docs/archived-design/design.md` lines 722-741, id set verified 1:1 against `RULES`, no scope
  creep found.

All gates and the drift proofs were re-run after both review fixes; still green (999/999, fmt
and clippy clean).

## Notes

- No existing `SPC-*` entry covered config-file validation errors generally (checked all five:
  rules, commands, exit codes, frontmatter losses, text output), so a new one was added --
  `SPC-6.md`, `explained_by: [SPC-6]` on the new catalog document, `migrated_from:
  docs/archived-design/design.md#validation-rules` (the "Config errors" prose sits as a
  subsection under that H2, same anchor `SPC-1` already uses for the same section).
- Field names: `id` mirrors `rules.md`'s own field. `reported_when` is a new field with no
  existing analogous multi-word field in any of the four prior catalog documents to mirror
  exactly; chosen in snake_case to match the pervasive snake_case convention already used for
  catalog/spec frontmatter fields (`content_type`, `explained_by`, `migrated_from`,
  `superseded_by`) rather than the unrelated camelCase used for validation-rule *config options*
  (`inlineCode`/`fencedCode`), a different concept in a different schema. Flagged during review
  as a judgement call, not changed, since the contract itself notes the project's field-naming
  is mixed and leaves this kind of choice to the builder.
- Top-level JSON key chosen as `"errors"` (`{"errors": [...]}`), following the one-word-per-
  concept pattern of `rules`/`commands`/`codes`/`losses` in the other four documents.
- Schema required no changes: `schemas/catalog.json` already covers `title`, `content_type:
  json`, optional `explained_by: ref[]`, which is exactly what `config-errors.md` needed.
