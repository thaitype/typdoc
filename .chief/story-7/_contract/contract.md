# Contract

The behaviour is `docs/design/`: `SPC-17` (Collection files; Slugs in file names), `SPC-14`
(Frontmatter values; Canonical form), `SPC-2` (`mv` explained; Arguments that name a document;
`new`), `SPC-1` (Default levels; `refs.slug`; `body.mentions`) and `SPC-12` (Naming a document).
This contract fixes the shapes the code takes and the choices the spec leaves open. When the
two disagree, the build stops and the disagreement is reported. Neither side is bent to fit the
other.

## Reading a file name

- **Where the key ends** (`SPC-17`). For a coded segment, the number is the maximal run of digits
  after `<CODE>-`. If the next character is `-`, a slug follows, and it runs to where the
  segment's own literal text after `{key}` begins, found by taking that text off the end of the
  name. A coded template holds no glob, so this split is unique.
- A name is a **member** of a coded collection when it fits the template up to the end of its key.
  Each member carries one of three name states:
  - `Expected`: no slug, or a valid slug, in a form the collection's `slug` accepts.
  - `UnexpectedForm`: no slug under `required`, or a slug under `none`.
  - `InvalidSlug`: a `-` follows the digits, but what comes after it is empty or holds whitespace,
    `/`, `#` or `:`.
- A template of the T1 kind, whose text right after `{key}` starts with a digit or `-`:
  - under `optional` or `required` it is `config.match-template`, and the message says to set
    `slug` to `none`;
  - under `none` it matches exactly as it does today (the digit backtracking in
    `template.rs` `fits_capture` stays), and it never looks for a slug.
- Every other template under `none` reads a slug, and the member's state is `UnexpectedForm`.
- `Template::render` takes the slug: `render(key, slug: Option<&str>)`. It serves `new` and
  `mv --renumber`.
- The index keeps each member's slug (`None` when the name has none), so that `refs.slug` and
  `mv` can read it without parsing the name again.

## Collection files

- `slug` is a new key in `config::read_collection`, with the value `"optional"` (the default when
  absent), `"required"` or `"none"`.
- Any other value, a value that is not a string, or `slug` in a collection whose schema has no
  code, is `config.collection-slug`. It is a config error that stops loading (exit 2), like
  `config.match-template`.
- `config.collection-slug` enters `rules.rs` `RULES`, `catalog/config-errors.md` (before
  `config.match-template`), and `fixtures/broken/config.collection-slug/`.

## `filename.pattern`

- Stray files (names that fit no template in a coded collection's folder) are reported as today,
  with no collection and no key.
- New: a member whose state is `UnexpectedForm` or `InvalidSlug` is reported under
  `filename.pattern` at its level (default `error`). This finding does carry `collection` and
  `key`, since the file is a document (`SPC-12`, A finding).
- The message says which case it is:
  - `WF-1.md` under `required`: needs a slug.
  - `WF-1-x.md` under `none`: the collection takes no slug.
  - `WF-1-a b.md`: the slug holds whitespace (or `/`, `#`, `:`, or is empty).

## Keys written with a slug

- One shared function in `typdoc-core` reads `<key>-<slug>`: it returns the key and the written
  slug, or nothing. It replaces `argument::looks_like_key` at each of its callers:
  `argument.rs` (arguments) and `refs.rs` `classify` / `resolve_into_project` (refs).
- `links::looks_like_key_shape` (mentions) does not change: a key written with its slug is not a
  mention (`SPC-1`).
- `refs::code_of` keeps working on the key part alone.
- **Arguments.** `WF-5-<slug>` with a valid slug (with any `ns:` or `project::` prefix) is
  `DocumentArg::Key` for `WF-5`. Text after the key that is not a valid slug makes no key (SPC-14
  reads the form only when the rest is a slug), and nothing ending in `.md` is a key (SPC-2). The
  written slug is dropped and never compared with the file: a stale slug prints nothing (`SPC-2`).
- **Refs.** A frontmatter value in key form keeps its written slug next to the key. It resolves
  by the key (`Via::Key`), and `refs.codedByPath` does not fire for it.

## `refs.slug`

- Enters `rules.rs` `RULES` and `CONFIGURABLE` (no options), `catalog/rules.md` (after
  `refs.moved`), and `fixtures/broken/refs.slug/`.
- It is reported by `validate` for a frontmatter ref in key form whose written slug differs from
  the target's current slug. A target with no slug counts as different; a ref with no written
  slug is never reported.
- The finding is placed at the ref, like `refs.moved`. Default level `warn`
  (`effective_level(Level::Warn, "refs.slug", …)`).
- Message: `` `story-2:WF-5-json-output-shape` refers to `WF-5`, whose file is now `WF-5-json-shapes.md` ``.
- `new` and `set` check it before they write, as they check `refs.moved` (`SPC-2`, What `new`
  and `set` check before they write): reported at its level, refusing the write only at `error`.
  At the default `warn` a stale slug never refuses a write.

## `new --slug`

- clap: `--slug <SLUG>` on `new`. It is valid only with a code target: with a path target it is
  `BadArgument` (exit 1).
- It is checked before the lock and before a number is allocated, so a refused `--slug` spends
  no number:
  - the character rule;
  - `--slug` under `none`, and a missing `--slug` under `required`.
  Each is `BadArgument` (exit 1), and the message states the rule.
- The file is `render(key, Some(slug))`. The `O_EXCL` create and the exit 7 rule are unchanged.

## `mv`

- **Changing a slug.** A coded source may now move within its namespace when the destination
  fits the source's own collection template **with the same key**. That means another slug, or
  no slug. A destination naming another key, or outside the template, keeps today's refusal
  (exit 1, same messages).
  - Destination in `UnexpectedForm`: carried out, and `filename.pattern` is in the output's
    findings, exit 0 (`SPC-2`).
  - Destination with an `InvalidSlug`: refused, exit 1, nothing written. The spec does not say
    this case explicitly; the contract takes `new`'s rule, that typdoc never writes an invalid
    slug.
- **Refs.** A key-only ref is unchanged. A ref written with a slug is rewritten to the key with
  the new slug, or to the key alone when the new name has none; any `ns:` prefix is kept. Body
  links and path refs are rewritten as for any `mv`.
- **`auto: moves`.** The previous path is recorded, through the existing `mv_document_change`
  path, for a slug change.
- **`--renumber`.** The file keeps its slug: `allocate_key` passes the source's slug to
  `render`. A ref written with the slug gets the new key and the same slug. A destination
  collection whose `slug` does not expect the form is still written, with `filename.pattern` in
  the findings, exit 0.

## Output

- `key` stays the key alone everywhere (`document_name`, `finding_json`, `reference_json`, text
  blocks). There is no new field, and the slug is visible only in `path` (`SPC-12`).
- The `mv` text and `--json` output carry the new `filename.pattern` finding like any other
  finding of a move.

## Documentation in the same PR

- `CHANGELOG.md`, `## [Unreleased]`: slugs, `new --slug`, the `slug` collection key,
  `refs.slug`, and an **upgrade note** listing the three cases from `SPC-17` (A project from
  before slugs).
- User docs:
  - `docs/reference/project-files.md` (`slug` key);
  - `docs/reference/commands.md` (`new --slug`, `mv` slug change, arguments with a slug);
  - `docs/reference/validation.md` (`refs.slug`, the new `filename.pattern` cases);
  - `docs/how-to/move-and-rename.md` (changing a slug);
  - `docs/explanation/keys-and-numbers.md` (the key identifies, the slug does not).
- The typdoc skill: `skills/typdoc/SKILL.md` and its `references/` (commands, project layout,
  validation).
- No change to anything about the names of documents without a code.

## Mode

Strict mode: each ticket is built test-first at the seams the testing decisions name, and
`/chief-review-code` runs before each commit. On top of that I verify every ticket myself: gates,
the diff, and a planted fault in each new check to see it go red. Every commit keeps
`typdoc validate`, fmt, clippy, `scripts/test.sh` and the public-text check green. A catalog id
lands in the same commit as the code that reports it.
