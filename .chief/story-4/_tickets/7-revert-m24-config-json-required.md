# 7: Revert M-24 — `.typdoc/config.json` becomes required again

Type: implementation
Status: resolved
Blocked by: None (can start immediately; reverts ticket 3's M-24 half, keeps `collections.empty`)

## What this delivers

`config.json` becomes required again, undoing ticket 3's M-24 change, before any release ships
it — reverting now costs nothing; reverting after a version publishes `config.json` as optional
would be a breaking change. `collections.empty` (the other half of ticket 3) stays: it's still
useful for a project that has `config.json` but no collections yet.

The reasoning, recorded in `docs/design/spec/SPC-7.md` as a `Decided ...` note: `version` in
`config.json` is the version of every file format typdoc owns — the config file, collection
files, `lock.json`, the schema format, and state files. Making the file optional means promising
forever that "no file" means version 1 of everything, with no version actually visible anywhere
in the project.

## Scope

- `project.rs::discover()`: both branches (`TYPDOC_DIR` and the ancestor walk) go back to
  `config_file(&dir).is_file()` — a project is marked by `.typdoc/config.json` existing, not by
  the `.typdoc/` folder alone.
- `config.rs::read_config_json()`: a missing file goes back to an ordinary IO error (via
  `Error::io_at`), not `Ok(Map::new())`.
- `error.rs`: `NoProject`/`NoProjectAt` messages go back to naming `.typdoc/config.json`, not
  `.typdoc/`.
- `collections.empty` (validate.rs, rules.rs, catalog/rules.md): untouched — still a
  `validate`-only, non-fatal `Finding` that fires when a project (which now always has
  `config.json`) has zero collections.
- Tests (`crates/typdoc/tests/config.rs`): `no_project_error()`'s helper goes back to asserting
  the message contains `.typdoc/config.json`. The ticket-2-added exact-wording test
  (`the_no_project_message_reads_exactly_no_project_folder_not_no_project_file`) is removed — it
  existed only to pin the now-reverted wording. The three renamed `..._the_typdoc_folder` tests go
  back to their original `..._the_config_file` names. The two ticket-3 tests that directly
  asserted the removed behavior (`a_typdoc_folder_with_no_config_json_at_all_loads_as_version_1`,
  `a_bare_typdoc_folder_with_nothing_in_it_still_lets_list_run`) are removed. The two
  plain-file-named-`.typdoc` tests stay, per instruction — their assertions didn't depend on
  which discovery mechanism was in effect, since a plain file at that path fails either way.
- Tests (`crates/typdoc/tests/validate.rs`): the two `collections.empty` tests that built their
  scratch project with a bare `.typdoc/` folder and no `config.json` now write a minimal
  `config.json` too, so the project is still discoverable under the reverted `discover()`.
- Docs reverted to their pre-story-4 wording wherever they claimed `config.json` is optional or
  that discovery looks for the folder rather than the file: `README.md`, `docs/getting-started.md`
  (only the discovery-marks-the-project sentence — the `collections.empty` tutorial demo stays,
  since that's still accurate), `docs/how-to/adopt-an-existing-folder.md`,
  `docs/reference/project-files.md`, `skills/typdoc/SKILL.md`,
  `skills/typdoc/references/project-layout.md` (also reverts "`version` is the only required key,
  and only when `config.json` exists at all" back to "`version` is the only required key"),
  `skills/typdoc/references/validation.md` (drops the M-24 cross-reference from the
  `collections.empty` row, keeps the row itself), `CHANGELOG.md` (removes the two now-false
  `### Changed` bullets, adds a one-line `### Added` bullet for `collections.empty` on its own).
- `docs/design/spec/SPC-7.md`: the opening paragraph and Discovery paragraph go back to naming
  `.typdoc/config.json`. New `## The version number` section, `Decided ...` throughout except one
  paragraph marked explicitly as still open: the version number's scope (every file format
  typdoc owns — `config.json`, collection files, the schema format, `lock.json`, state files;
  a local schema is read in its project's own version; a remote schema's own format marker is
  left to the remote-schemas decision, not this one), when it bumps (an existing key changes
  meaning/type, is renamed/removed, or a file's structure changes) versus when it doesn't (a new
  state-entry field an older typdoc's own write leaves untouched — verified directly, not just
  asserted: a scratch state file with an extra field, run through a real `typdoc new`, came back
  with that field intact and only `last` changed; a new `config.json` key, refused loudly as
  `config.unknown-key` rather than silently accepted), and that an unknown number is
  `config.version` and stops the command. Left open: whether the number only ever goes up by one,
  and whether a newer typdoc reading an older project reads it as is or says how to convert it.
  `docs/migrating-design/design.md` needed no further change — the sections it already lost to
  `SPC-7` stay lost; `SPC-7` having correct content again is what matters, not restoring anything
  there.
- `docs/design/spec/SPC-8.md`, the wildcard-exclusion behavior in `SPC-7` and
  `skills/typdoc/references/project-layout.md`, and everything in tickets 1/2/4/5/6 — untouched,
  none of it depended on M-24.

## Testing

- `cargo build --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt
  --check`: clean.
- `scripts/test.sh`: 1057 tests passing (1060 minus the 3 tests removed for testing
  now-nonexistent behavior).
- `typdoc validate` on the repo: exits 0.
- Public-text gate scoped to every touched file: clean.
