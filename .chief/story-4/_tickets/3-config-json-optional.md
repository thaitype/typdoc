# 3: `.typdoc/config.json` becomes optional (M-24)

Type: implementation
Status: open
Blocked by: None (can start immediately)

## What this delivers

A `.typdoc/` folder with no `config.json` is a valid project, read as `{"version": 1}`. `typdoc
validate` warns (not stops) when the project has no collections at all, regardless of what else
`.typdoc/` holds. Demoable: a bare `.typdoc/` directory with nothing in it (or only `state/`/
`locks/`) still lets every command run; `validate` shows a `collections.empty` warning; `list`/
`get` on that same project return normal exit codes.

## Scope (contract §2)

- `project.rs::discover`: `config_file(&dir).is_file()` → `dir.join(TYPDOC_DIR).is_dir()` in both
  branches (`TYPDOC_DIR` env var and the ancestor search). Use `.is_dir()` specifically, not
  `.exists()` — a plain file named `.typdoc` must not count as a project.
- `config.rs::read_config_json`: missing file → treat as an empty object (`{"version": 1}`
  equivalent — matches every default `Config::load` already produces). Present-but-malformed
  stays on today's error path (`config.parse`/`config.version`).
- New `collections.empty` — **not** a `config.*` id, **not** added through `Config::load`'s
  `Report`. Implement as a `validate`-only `Finding` (`Severity::Warn`), mirroring
  `state_retired_finding`'s shape (project-level: `path = TYPDOC_DIR`, `namespace`/`collection`/
  `key`/`field`/`position` all `None`). Fires when `collections.is_empty()` after a successful
  load. Add to both `rules.rs::ALWAYS_ON` (not configurable) and `rules.rs::RULES` (needs
  `fixtures/broken/collections.empty/`).
- Skill (`skills/typdoc/`) and user docs (README, getting-started, how-to/adopt,
  reference/project-files) currently state or imply `config.json` is required — update in this
  same ticket/PR.

## Testing (contract's Testing Decisions, M-24 bullets)

- Discovery: a plain file named `.typdoc` (not a directory) → `discover()` reports no project.
- A project with no `config.json` at all still loads as version 1 (reuse/extend an existing
  minimal fixture or add one under `fixtures/valid/`).
- `fixtures/broken/collections.empty/` per `RULES`'s own fixture convention.
- The non-fatal proof that actually matters: a zero-collection project still returns a normal
  exit code from `list`/`get`; only `validate` shows the `collections.empty` warning.
