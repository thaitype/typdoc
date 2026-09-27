# Ticket 01 Report

## Ticket
Read slugged file names: the key/slug split in `template.rs`, the collection's `slug` key and
`config.collection-slug`, member name states, and `filename.pattern` for a name in the form its
collection does not expect or with an invalid slug.

## Outcome
done (63354f1)

## Decision
- **Issue:** a name under `none` whose text after the key is empty or holds an excluded
  character fits both the `UnexpectedForm` and the `InvalidSlug` description.
- **Options considered:** report it as `InvalidSlug`; report it as `UnexpectedForm`.
- **Chosen:** `UnexpectedForm`. Under `none` any text after the key is the form the collection
  does not expect, and removing it settles both. The `filename.pattern` message under `none`
  names an empty slug without an empty pair of backticks.

## Notes
- Verified by running: fmt, clippy (workspace), `scripts/test.sh` 1079 passed, `typdoc validate`,
  public-text check (no hit in a changed file). A planted fault that let `#` into a slug turned
  `a_slug_that_is_empty_or_holds_an_excluded_character_is_a_member_with_an_invalid_slug` red;
  restored from saved bytes.
- The `config.match-template` catalog entry now also names the digit-or-`-` case, and its
  `explained_by` includes `SPC-17`.
- Left for later tickets: `new` does not yet take `--slug` or refuse under `required` (03);
  `--renumber` drops the slug (05).
- The case-only `keys.unique` test (`WF-5-a.md` and `WF-5-A.md`) runs on Linux only.
