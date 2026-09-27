# Ticket 06 Report

## Ticket
Changelog with an upgrade note, the explanation and how-to docs, and a consistency pass over every
doc and skill file the story touched.

## Outcome
done (10417b5)

## Notes
- Verified by running: fmt, clippy (workspace), `scripts/test.sh` 1125 passed, `typdoc validate`,
  public-text check over every file changed since `0c0d62b` (no hit). The upgrade note lists
  exactly the three cases of `SPC-17` (A project from before slugs).
- `SPC-1`'s default-levels row for `filename.pattern` still named only a file that fits no
  template; it now also names the cases `SPC-17` adds (committed with this report).
- The spec counts agree with the catalogs: ten configurable rules, fifteen always-on, twenty-one
  config errors.
- Read-through fixes: the skill's validation reference (`filename.pattern` cases,
  `config.collection-slug`, the `config.match-template` case, `refs.moved` naming a new key or
  path) and `docs/explanation/how-typdoc-sees-files.md`, which said a coded file name is its key
  and never changes.
- The `get`/`list` output with `key` alone and `path` with the slug is covered by existing tests
  (`arguments.rs`, `validate.rs`), so no test was added.
- Example console output in the docs comes from runs of the built binary on scratch projects.
