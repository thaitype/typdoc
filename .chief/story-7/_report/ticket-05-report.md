# Ticket 05 Report

## Ticket
`mv --renumber` keeps the file's slug under the new key; refs written with the slug get the new
key and the same slug.

## Outcome
done (28a9f05)

## Decision
- **Issue:** a slug already on disk that breaks the character rule (`WF-5-a#b.md`): SPC-2 says
  the slug is kept as it was and never dropped, and the contract said typdoc never writes an
  invalid slug.
- **Options considered:** refuse the renumber; keep the slug and report `filename.pattern`.
- **Chosen:** keep it and report it (exit 0). In a renumber the caller gives no slug, and the
  never-write rule is about a slug the caller gives (`mv` to a new name, `new --slug`). The
  contract's `--renumber` bullet now says so.

## Notes
- Verified by running: fmt, clippy (workspace), `scripts/test.sh` 1125 passed, `typdoc validate`,
  public-text check (no hit in a changed file). A planted fault that dropped a slug holding `#`
  on renumber turned `an_invalid_slug_already_on_disk_is_renumbered_as_it_is_and_reported` red;
  restored from saved bytes.
- Collections are project-wide, so the destination collection's `slug` is always the source's;
  the other form arises only from a name already in it, and that is what the test covers.
- A ref whose written slug is already stale keeps that slug under the new key, so `refs.slug`
  still reports it afterwards. Not pinned by a test.
- Left for ticket 06: `docs/how-to/move-and-rename.md`, `docs/explanation/keys-and-numbers.md`,
  the `--renumber` row in `skills/typdoc/SKILL.md`, and `CHANGELOG.md`.
