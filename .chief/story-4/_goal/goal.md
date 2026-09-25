# Goal

Story 4 ships v0.3.0, delivering:

1. **Wildcard namespace exclusion** — a `namespaces` entry can carry `!`-prefixed exclusions in
   the same list, gitignore-style: `["story-*", "!story-1", "!story-2"]`. This lets a project
   adopt typdoc one namespace at a time under a wildcard, instead of moving every matching folder
   at once. Decided: `!` is `namespaces`-only in this story — `--namespace`/`TYPDOC_NAMESPACE`
   do not support it (Out of Scope); passing one with a leading `!` still gets a clear syntax
   error, not a silent misread.
   - Patterns apply in list order; the last pattern that matches a folder decides whether it's a
     namespace — a later `!` excludes what an earlier pattern included, and a later plain entry
     can re-include what an earlier `!` excluded.
   - An excluded namespace is fully invisible: not validated, not queried, a ref into it resolves
     as not found; `--namespace`/`TYPDOC_NAMESPACE` naming one explicitly, or a write into one
     (`new`, `mv --renumber`), fails the same way as naming a namespace that doesn't exist at
     all.
   - An importing project sees nothing of an imported project's excluded namespace.
   - A namespace's `.typdoc/state/<namespace>.json` is left untouched while excluded — not read,
     written, or migrated — so re-including it later continues numbering from where it left off,
     with no reissued codes.
   - **An `!` entry that matches no folder is always silent** — no finding, in either form: an
     exact name (`!story-9`) or a glob (`!old-*`). This is a rule of its own for `!` entries, and
     it differs from a plain (non-`!`) entry, which keeps today's behavior: an exact name
     matching no folder is still reported (`config.namespaces-entry`), a glob matching nothing
     stays silent.
   - **Deliverable, not follow-on:** this changes user-visible behavior, so the skill
     (`skills/typdoc/`) and user docs that describe `namespaces`/`--namespace`/`TYPDOC_NAMESPACE`
     ship in the same PR as the behavior.

2. **`.typdoc/config.json` becomes optional (M-24)** — a `.typdoc/` folder with no `config.json`
   is read as a project with `{"version": 1}`; every command's project discovery looks for the
   folder, not the file. `typdoc validate` warns when the project has no collections at all,
   regardless of what else `.typdoc/` holds (state files, locks).
   - **Deliverable, not follow-on:** this changes user-visible behavior, so the skill
     (`skills/typdoc/`) and user docs (README, getting-started, how-to/adopt,
     reference/project-files) ship in the same PR as the behavior.

3. **`mv` onto its own exact path gets its own message** — `mv a.md a.md` (source and
   destination are the identical path) still exits 7, but no longer reuses the case-only-rename
   wording; it gets a message that actually describes the identical-path case.

4. **Publish to crates.io via a PR-gated GitHub Actions workflow** — modeled on reeve's
   `publish.yml`: `workflow_dispatch` with `version` + `dry_run` inputs, an ubuntu+macos
   test/clippy gate, then a publish step that handles `typdoc-core`, `typdoc-fs` and `typdoc` in
   dependency order, dry-run first (`typdoc-testkit`, `publish = false`, is never published).
   `Cargo.toml` gains the metadata crates.io requires (`description`, `license`, `repository`)
   via `[workspace.package]`, inherited by each published crate — license MIT, per the README.
   The workflow itself lands through a PR like every other change in this story; a green
   `dry_run: true` run of the workflow on the PR's branch, covering all three crates, is the
   evidence — its CI run id goes in the story report — for deciding whether to trigger the real
   publish. The `CARGO_REGISTRY_TOKEN` secret, the real trigger, and the merge are all handled
   outside this story's own automation — never touched here.

5. **Release mechanics for v0.3.0** — bump every published crate's version (`typdoc-core`,
   `typdoc-fs`, `typdoc`) to `0.3.0`, and every intra-workspace dependency version requirement
   that currently pins `"0.2.0"` (e.g. `typdoc-fs`'s and `typdoc`'s `typdoc-core = { version =
   "0.2.0", ... }`) to `"0.3.0"`. Add a CHANGELOG entry for v0.3.0. Update the skill's version
   line (currently states 0.2.0). Update the README's tag-based install instructions to
   reference the new tag. Creating the git tag itself happens separately, after merge — not part
   of this story's automated workflow.

## Out of Scope

- Decisions-as-collection, slug in coded filenames (no change), collection-level
  `filename.pattern` vs `README.md` (needs a decision first) — all parked, not this story.
- Comment cleanup per the `comment-review` skill — next story.
- Actually triggering a real (non-dry-run) crates.io publish — the workflow ships in this story;
  running it for real happens separately, after the story merges.
- Attaching built binaries to a GitHub release, the way reeve's workflow does — not in this
  story; `cargo install` already covers installation.
- `!`-prefixed exclusion in `--namespace`/`TYPDOC_NAMESPACE` — `namespaces` only, this story.
