# Ticket 04 Report

## Ticket
`mv` changes a slug: a coded document moves to a name its own collection template reads with the
same key; refs are rewritten in their written form; `auto: moves` records the previous path.

## Outcome
done (90e1dde)

## Decision
- **Issue:** SPC-2 says a body link left on the old file name is reported by `refs.moved` with
  the new path, but a moved record is a path from the project folder and a body link written from
  another folder was looked up only by its written text, so it came out as `body.links`.
- **Options considered:** look the link up by its written text only (the old behaviour); also
  look it up by the path joined from the holder's folder.
- **Chosen:** also by the joined path. It is what SPC-1 (`refs.moved`: a body link to a moved
  target is `refs.moved`, not `body.links`) asks for, and it applies to every move, not only a
  slug change. It is a user-visible fix and goes in the changelog.

## Notes
- Verified by running: fmt, clippy (workspace), `scripts/test.sh` 1121 passed, `typdoc validate`,
  public-text check (no hit in a changed file). A planted fault that dropped the same-folder check
  from the destination filter turned `another_key_in_the_same_folder_is_refused_and_nothing_changes`
  red; restored from saved bytes.
- A destination slug with an excluded character or an empty slug is refused in every `slug` mode,
  `none` included: typdoc never writes an invalid slug.
- A slug change reports no mentions as unrewritten: the key did not change, so a mention of it is
  still right.
- A recorded path points `refs.moved` to the document's current path, also for a coded document;
  a recorded key form (`ns:KEY`, from `--renumber`) still points to the key.
- `Template::key` is gone; `read` serves its callers.
- Known limits, not changed here: with a `{key}/README.md` template a slug change moves the
  `README.md` file only, and other files in the old folder stay there (the same as any `mv`,
  which moves one file). Frontmatter path refs written from another folder are still looked up by
  their written text, so they get `refs.resolve` rather than `refs.moved`. A destination another
  collection's glob also matches is not refused.
- `mv_result` also reports `filename.pattern` for `--renumber` into a collection whose `slug` does
  not expect the name; ticket 05 adds its tests.
