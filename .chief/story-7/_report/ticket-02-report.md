# Ticket 02 Report

## Ticket
Keys written with a slug: one shared reader (`argument::read_key`) for arguments and refs, refs
resolved by key with the written slug kept, the configurable rule `refs.slug` (validate and the
`new`/`set` pre-write check), and mentions pinned as key-only.

## Outcome
done (697d1e1)

## Decision
- **Issue:** the contract said `WF-5-anything` names `WF-5`; SPC-14 reads the form only when the
  rest is a slug (SPC-17), and SPC-2 says a key never ends in `.md`.
- **Options considered:** read any text after the key as a slug; read only a valid slug, and
  nothing ending in `.md`.
- **Chosen:** only a valid slug, and never text ending in `.md`. `WF-5-a#b` is no key (exit 1 as
  an argument, a relative path as a ref), and `WF-1-x.md` stays a path, so a relative ref to a
  document without a code named that way cannot resolve to `WF-1`. The contract's Arguments
  bullet now says so.

## Notes
- Verified by running: fmt, clippy (workspace), `scripts/test.sh` 1099 passed, `typdoc validate`,
  public-text check (no hit in a changed file). A planted fault that named the key instead of the
  file in the `refs.slug` message turned
  `refs_slug_warns_at_the_ref_when_the_written_slug_is_not_the_files_or_the_file_has_none` red;
  restored from saved bytes.
- `refs.moved` also finds a slugged key ref through its key with the slug dropped.
- `mv::rewritten_path_ref` already keeps a written slug under a new key for `--renumber`
  (unit-tested only); ticket 05 adds the CLI tests and keeps the file's own slug. Until then a
  renumbered slugged file loses its slug and a slugged ref to it draws `refs.slug`.
- The `refs.slug` message names the path segment that carries the key: the file for
  `tickets/{key}.md`, the folder for `{key}/README.md`.
