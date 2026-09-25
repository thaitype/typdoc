# Ticket 7 Report

## Ticket
Revert M-24 (`.typdoc/config.json` optional) — `config.json` is required again, before any
release shipped it optional. `collections.empty` stays.

## Outcome
done

## Decision
No ambiguity to resolve — the ticket's scope was fully specified, including the exact outline for
the later `## The version number` addition to `SPC-7`. The one judgment call was where to record
the core reasoning: `docs/design/spec/SPC-7.md` already owns the project/discovery topic, so both
the `config.json`-required paragraph and the fuller version-number section went there, next to the
(now-reverted) claim they replace, rather than in a new document.

Before writing the version-number section's claim that an older `typdoc`'s write leaves an
unknown state-entry field untouched, verified it directly rather than trusting the summary as
given: built a scratch project with `.typdoc/state/default.json` holding `{"wf": {"last": 3,
"futureField": "kept-me"}}`, ran a real `typdoc new WF`, and confirmed the file came back with
`last` bumped to `4` and `futureField` still there, byte for byte. Traced why in the code too —
`state::rewrite`'s in-place patch only touches the `last` value's span when one already exists,
distinct from the `canonical()` fallback (used for a genuinely new entry) which does overwrite an
entry wholesale — so the claim holds specifically because this state entry already had `last`,
not automatically for every write.

## Notes
- Verified the two "M-24 was free to revert" claims directly rather than assuming them: `git log`
  confirms no `v0.3.0` tag or crates.io publish exists yet on this repo, so reverting a behavior
  change that only ever lived on an unmerged branch has no compatibility cost.
- The two tests kept per instruction (`a_plain_file_named_dot_typdoc_is_not_a_project_via_the_ancestor_walk`,
  `..._via_typdoc_dir`) needed no assertion changes — their own check is just "this is not a
  project," which `no_project_error()` still confirms correctly once its message-content
  assertion reverted; only their doc comments were reworded to drop the M-24/contract framing.
- The two `collections.empty` tests in `validate.rs` that used a bare `.typdoc/` folder with no
  `config.json` needed a minimal `config.json` added, or `discover()` would fail to find the
  project before `collections.empty` ever got a chance to fire — caught by actually running the
  test suite, not by reading the diff and assuming it would still pass.
- `CHANGELOG.md`'s `### Changed` section for 0.3.0 is gone entirely — both of its bullets were
  M-24-specific and nothing else occupied that category. `collections.empty` moved to its own
  line under `### Added`, since it's still shipping independently of M-24.
- `docs/migrating-design/design.md` needed no changes: the Namespaces/Discovery/Model sections it
  lost to `SPC-7` in ticket 6 stay lost — `SPC-7`'s content being correct again is what restores
  accuracy, not un-deleting anything from the working copy.
- `cargo build`/`clippy`/`fmt` clean. `scripts/test.sh`: 1057 passing (1060 − 3 removed tests that
  asserted now-nonexistent behavior: the exact-no-project-wording test and the two
  bare-`.typdoc`-folder-is-valid tests). `typdoc validate` on the repo exits 0. Public-text gate
  scoped to every touched file: clean.
