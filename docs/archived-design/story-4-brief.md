# Story 4 — brief

Story 4: `/chief-plan` with the normal grill — small story, no wayfinder — then plan and build.

Branch from `origin/main` (`27805f8`, v0.2.0). The local checkout is still on
`story-3-catalog-and-release`, and the worktree `.worktrees/story-4-typdoc-skill` is story 3's
merged skill work — the name is old, not this story.

## Release target

Story 4 ships **v0.3.0**, released through the crates.io workflow below.

## 1 · Ignore some namespaces matched by a wildcard — the core of the story

Motivation: a project with several namespaces already configured under one wildcard (e.g.
`story-*` covering `story-1`, `story-2`, ...) needs to exclude some of them from validation for
now without moving them out of the wildcard's reach, so adoption can happen one namespace at a
time instead of migrating every matching folder at once.

Decided:
- **Syntax: `!` in the same list**, like `.gitignore` — `"namespaces": ["story-*", "!story-1", "!story-2"]`.
  The same form works in `--namespace` / `TYPDOC_NAMESPACE`.
- **An excluded namespace is invisible to typdoc** — not validated, not queried, and a ref into it
  resolves as not found.

Fog for the map (not yet decided): order of patterns and whether a later pattern can re-include;
a `!` pattern that matches nothing; `--namespace` naming an excluded namespace; writes into one
(`new`, `mv --renumber`); what imported projects see; state files of an excluded namespace.

## 2 · M-24 — `.typdoc/config.json` becomes optional

Motivation: `.typdoc/config.json` holding only `{ "version": 1 }` carries no information a
missing file wouldn't already imply, so requiring the file is unnecessary ceremony.

Decided: **a `.typdoc/` folder with no `config.json` is a project**, read as
`{ "version": 1 }`; `typdoc validate` warns when the folder holds nothing (there is no `check`
command; the intent is a warning from `validate`).

Consequences to carry: project discovery looks for the folder, not the file, in every command;
"no file = version 1" must stay true forever. Docs touched: README, getting-started,
how-to/adopt, reference/project-files, and the skill.

## 3 · `mv a.md a.md` exit 7 message

Moving a document to its own exact path gives the message for the case-only rename
("file system does not tell the two names apart"). Fix the message for the identical-path case.

## 4 · Publish to crates.io

Direction: publish through a PR-gated workflow, not an ad hoc manual publish.

Model: `~/gits/thaitype/reeve/.github/workflows/publish.yml` (workflow_dispatch with version +
dry_run, ubuntu/macos gate, version check, `cargo publish --dry-run`, publish with
`CARGO_REGISTRY_TOKEN`, build binaries onto the release). reeve is one crate; typdoc is a
workspace: publish `typdoc-core` → `typdoc-fs` → `typdoc` (`typdoc-testkit` is `publish = false`).
**The token is a secret, added separately; do not handle it.** Publishing is irreversible: dry
run first, and the real publish waits for that separate step.

## Rules that still hold

- Behaviour change → the skill (`skills/typdoc/`) and user docs change in the same PR.
- User docs don't refer to code; links are markdown links with human words.
- No program reads markdown as spec (M-1).
- Push as `mildronize` with the one-shot credential helper; merging is a separate, later step.

## Out of scope — parked, not this story

Decisions-as-collection, slug in coded filenames (no change recommended), collection-level
`filename.pattern` vs `README.md` (needs a decision first). Comment cleanup per the
`comment-review` skill is the **next** story — don't mix it in.
