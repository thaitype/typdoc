# Contract

## 1. Wildcard namespace exclusion

### Config shape

No schema change to `namespaces`: still a JSON array of strings (`namespace_entries` in
`config.rs`), each entry now optionally prefixed with `!`. **`!` is `namespaces`-only in this
story (decided 2026-09-24) — `--namespace`/`TYPDOC_NAMESPACE` do not support it** (revised from
the original plan; see the removed "Scope selection" subsection below and Out of Scope).

### Algorithm — `namespaces.rs::resolve`

Today's code (`entries.iter()` → `BTreeMap::extend`, order-independent) becomes an ordered pass,
gitignore-style:

- For each entry, in list order: split a leading `!` off into `(negate, pattern)`. Error
  messages keep quoting the full original entry (`!story-9`, not `story-9`), so a user can find
  the offending config line.
- Resolve `pattern` against the directory listing with today's unchanged `entry_folders` logic
  (glob/exact-name matching, symlink handling, non-UTF8 skip, the multi-segment/`**` refusal).
- `negate == false`: add every matched `(name, path)` to the accumulated set, as today.
- `negate == true`: remove every matched name from the accumulated set.
- Empty-match reporting forks on `negate`: unchanged for plain entries (exact name matching
  nothing → `config.namespaces-entry`; glob matching nothing → silent). A `!` entry matching
  nothing is **always** silent — exact name or glob alike, no report at all (decided 2026-09-24).
- `name_problem` (reserved/invalid names) and the `nested` check (`.typdoc/` inside a namespace
  folder) run only over the final accumulated set — an excluded namespace never reaches them,
  which is what makes it "not validated" by construction rather than by a separate check.

### `--namespace`/`TYPDOC_NAMESPACE` with a leading `!` — no new code, verified already correct

`select()`'s `glob_match` (`scope.rs:164-190`) validates each comma-separated item through
`plain_name` before matching: `!story-1` fails `plain_name` (`!` isn't ASCII alphanumeric, `-` or
`_`), so `names_only` is false and `glob_match` already returns "`` `!story-1` is not a namespace
name or a glob: a name uses ASCII letters, digits, `-` and `_`, and a glob only `*` ``" — a clear
syntax error, not a silent no-op or a literal-folder-name misread. Traced by hand against
`scope.rs` as written today; no code change needed here, only a regression test locking this in
(Testing Decisions).

### Items 3 & 4 — explicit `--namespace`/write into an excluded namespace

Fall out of the `resolve()` fix with **no new code**: once an excluded namespace never enters
`config.namespaces`, `select()`'s existing "not a namespace of this project" refusal
(`scope.rs:183-188`) already fires for `--namespace story-1` / `TYPDOC_NAMESPACE=story-1` when
`story-1` is excluded, and `new`/`mv --renumber`'s own namespace resolution goes through the same
`scope::choose`/`select` path. Verify this at build time with a fixture rather than adding a
separate check — a passing test is the proof this needs no new code, not an assumption.

### Item 5 — imports

`Project::load` for an imported project runs the same `Config::load` → `namespaces::resolve` the
top-level project does, so an imported project's excluded namespaces are already gone before
`imports.rs`/`scope.rs` ever see them. No change beyond the `resolve()` fix itself.

### Item 6 — state files (revised after a load-fatal conflict found during contract, see below)

`.typdoc/state/<namespace>.json` must survive untouched while its namespace is excluded, so
re-including it later continues numbering with no reissued codes. Verified in code: nothing
writes a namespace's state file except through `config.namespaces` (unaffected by this story), so
"don't touch" already holds for **writes**. The problem is **reads at load time**:

`state::orphans` (`state.rs:320-346`) flags any `.typdoc/state/<name>.json` whose `<name>` isn't
in the *current* `config.namespaces` — and `config.state-orphan` (`project.rs:5812-5820`) is not
a `validate`-only finding, it joins the same `report` every other config error does, which fails
the **entire project load** (`project.rs:5785-5791` explicitly contrasts this with
`state.retired`, which is exempted from stopping the load for exactly this reason). Excluding a
namespace that has ever been used would otherwise break every typdoc command on that project the
moment `resolve()` starts dropping it from `config.namespaces` — the opposite of the point of
excluding it to defer adoption.

**Confirmed (2026-09-24 — this is item 6's own answer made real, not a new decision):**
`namespaces::Resolved` gains the set of namespace names matched by at least one entry and then
removed by a later `!` — not "everything on disk," only names the config's own patterns actually
reached and excluded. `state::orphans`'s `known` set becomes `config.namespaces` names ∪ this
excluded set: an excluded namespace's state file reads as "known but excluded," never orphan,
while a state file matching neither a resolved namespace nor an exclusion pattern still fires
`config.state-orphan` exactly as today.

**Edge case, confirmed correct as designed:** a namespace folder that no longer exists at all,
named only by a `!` entry (e.g. `!story-9` where `story-9`'s folder was deleted) — the `!`
pattern matches nothing, so nothing is ever added to the "excluded" set (matching, not text, is
what puts a name there). Its leftover state file is therefore a genuine orphan and still fires
`config.state-orphan`, exactly as today. No special-casing needed; this falls out of "excluded
means matched-then-removed," not "excluded means named by a `!` entry somewhere."

## 2. `.typdoc/config.json` becomes optional (M-24)

### Project discovery — `project.rs::discover`

Changes from `config_file(&dir).is_file()` to `dir.join(TYPDOC_DIR).is_dir()` (directory
existence, not mere existence) in both branches (`TYPDOC_DIR` env var and the ancestor search).
`Error::NoProjectAt`/`Error::NoProject` unchanged in shape, just triggered by folder absence
rather than file absence. **`.is_dir()` specifically, not `.exists()`** (2026-09-24): a
plain file named `.typdoc` (no extension, same name as the folder) must not count as a project —
needs its own test case, since `.is_file()` → `.exists()` would be the easy wrong port.

### Config loading — `config.rs::read_config_json`

Currently `fs::read(&file)` errors on a missing file. Changes to: file missing → return an empty
object (equivalent to `{"version": 1}`, matching every default `Config::load` already produces
for `entries = None`, `validation = Rules::new()`, `imports = {}`, `lock = LockMode::default()`).
File present but unreadable/malformed → unchanged existing error path
(`config.parse`/`config.version`).

### `validate` warning — `collections.empty` (renamed from the original `config.no-collections`
proposal, 2026-09-24)

**Not a `config.*` id, and not added through `Config::load`'s `Report`.** Checked: nothing in
the codebase special-cases the `config.` prefix string itself, but every existing `config.*` id
is fatal purely because it's implemented via `Config::load`'s `Report`/`ConfigError` path, which
`load_inner` always turns into a load failure (the same mechanism that made `config.state-orphan`
load-fatal above). The decision was "warn," not "stop every other command," so this must be
implemented the way `state.retired` is — a `validate`-only `Finding` (`Severity::Warn`), computed
after a successful load, never touching `Config::load`'s `Report`. Naming it outside `config.*`
keeps the prefix's existing meaning ("load-fatal") intact rather than making it the first silent
exception.

- New `collections_empty_finding(path: &str) -> Finding` in `validate.rs`, mirroring
  `state_retired_finding`'s shape (`Severity::Warn`, `path = TYPDOC_DIR` i.e. `.typdoc`,
  `namespace`/`collection`/`key`/`field`/`position` all `None` — a project-level finding, the
  same shape `schema.valid` uses for a finding that isn't about one document).
  Fires when `collections.is_empty()`, project-wide, independent of what else `.typdoc/` holds
  (`state/`, `locks/`, nothing at all).
- Added to both `rules.rs::ALWAYS_ON` (like `state.retired`: always active, not
  configurable — matches the "just warn" decision, no `validation` toggle) and `rules.rs::RULES`
  (needs `fixtures/broken/collections.empty/` per that list's own convention).

### Docs & skill (goal item deliverable)

README, getting-started, how-to/adopt, reference/project-files, and `skills/typdoc/` all
currently state or imply `config.json` is required — each gets a pass in the same PR as the
behavior change.

## 3. `mv a.md a.md` message

`project.rs:4077-4089`: the `same_file` check (device+inode identity, `typdoc-fs/src/lib.rs:59`)
is true both for the literal-same-path case and the case-only-rename case, but only the
case-only message exists today. Fix: branch on `from_path == to_path` (string equality) first —
give it its own message (e.g. `` `{path}` already names this document: nothing to move ``, exact
wording is a build-time call) — and keep today's "the file system does not tell the two names
apart" message only when `same_file` is true but the paths differ as strings (the genuine
case-only scenario). Exit code stays 7 either way (`Error::AlreadyExists`).

## 4. Publish to crates.io

### Crate metadata

`[workspace.package]` in the root `Cargo.toml` gains `license = "MIT"` and `repository =
"https://github.com/thaitype/typdoc"`, both inherited (`license.workspace = true`,
`repository.workspace = true`) by `typdoc-core`, `typdoc-fs`, `typdoc` (and `typdoc-testkit`,
harmless since it's `publish = false`). `description` is **not** inherited — each published
crate sets its own, reflecting its actual role rather than one generic string across a lib/lib/bin
split:
- `typdoc-core`: "Reads a typdoc project and answers questions about it. It changes nothing."
  (already its `lib.rs` doc comment)
- `typdoc-fs`: "The write half of the seam: the file system itself." (already its `lib.rs` doc
  comment)
- `typdoc`: "A CLI that treats a folder of Markdown files as typed, linked documents."

### Workflow shape

**Revised (2026-09-24): `workflow_dispatch` only runs a workflow that already exists on
the default branch** — `gh workflow run --ref` and the UI both refuse a `workflow_dispatch` file
that lives only on a PR branch. A brand-new `publish.yml`, gated entirely behind
`workflow_dispatch`, therefore **cannot produce the "green dry-run on the PR branch" gate
evidence this story requires** — that workflow doesn't exist anywhere runnable until after
merge. Split into two pieces:

1. **The gate, in a new `.github/workflows/publish-check.yml` — its own file, not a job inside
   `ci.yml` (corrected 2026-09-24): `on.push.paths`/`on.pull_request.paths` is a
   workflow-level trigger, not job-level, so putting a path filter on this job inside `ci.yml`
   would stop `test`/`fmt`/`clippy` from running on any PR that doesn't touch a manifest — not
   acceptable. `ci.yml` itself is untouched.** `publish-check.yml`: `on: push, pull_request:
   branches: [main]` (matching `ci.yml`'s own trigger shape) with `paths:` scoped to
   `Cargo.toml`, `Cargo.lock`, `crates/*/Cargo.toml` (see the ci.yml-fix risk note below for why
   the path filter exists at all). One job, `publish-dry-run`, ubuntu-latest + macos-latest
   matrix (matching `ci.yml`'s existing job shape), running `cargo publish --workspace --dry-run`.
   No token needed — a dry run never touches `CARGO_REGISTRY_TOKEN`. This runs automatically on
   this story's own PR (which bumps every manifest to 0.3.0, so the path filter doesn't exclude
   it), giving the gate evidence needed: its CI run id, green, goes in the story report.
   **Ticket acceptance criterion:** this story's own PR must show `publish-check` running and
   green, *and* `ci.yml`'s existing jobs (`test`, `fmt`, `clippy`) still running in full —
   proving the split didn't regress the existing gate.
2. **The real trigger, in a new `.github/workflows/publish.yml`, `workflow_dispatch` only** —
   `version` (required text) + `dry_run` (boolean, default `true`) inputs, modeled on reeve's
   file otherwise. Since `workflow_dispatch` only works once a workflow file is on the default
   branch, this file is unusable until the story's PR merges — which matches how the real
   trigger and merge are authorized anyway (both happen separately, so "only invocable after
   merge" costs nothing here). Its own `gate` job (ubuntu+macos `cargo test` + `cargo clippy --workspace
   --all-targets -- -D warnings`) + version-check step (all three published crates' `Cargo.toml`
   versions equal the input — this also implicitly proves the intra-workspace `version = "0.3.0"`
   dependency bump landed, since a stale `0.2.0` path requirement fails the workspace build
   before this step runs) + `cargo publish --workspace --dry-run` then (real run only)
   `cargo publish --workspace`.
- `cargo publish --workspace` confirmed available on the pinned toolchain (`cargo 1.96.0` has
  `--workspace`/`--exclude` under "Package Selection" in `cargo publish --help`, checked locally
  against this repo's `rust-toolchain.toml` pin) — publishes in dependency order itself and skips
  `typdoc-testkit` automatically (`publish = false`), no manual `needs:` chain needed. **Still not
  run for real anywhere** — `publish-check.yml`'s job is what actually proves the dry run
  succeeds; `publish.yml`'s own dry-run step is a second, standalone check before the real one on
  the same invocation, not the gate evidence itself.
- **Why `publish-check.yml` needs a path filter at all:** after v0.3.0 is really published, an
  unrelated later PR that hasn't bumped the manifests would re-run a dry-run at an
  already-published version on every push otherwise. `cargo help publish`'s documented step split
  (local checks + package build vs. the server-side "additional checks" on upload, which
  `--dry-run` skips) suggests a bare dry-run shouldn't hard-fail on that alone — but this isn't
  verified against a real index (nothing is published yet to test against, and a real publish is
  not this story's automation to trigger). The `paths:` filter sidesteps the question entirely
  rather than depending on that behavior: the job only runs when a manifest actually changed, so
  a PR that doesn't touch versions never re-asks it. (Checked: no `include_str!`/`include_bytes!`
  outside `tests/` in any crate, so no currently-visible way for packaging to break without a
  manifest touch either.)
- No `release-binaries` job — out of scope this story (goal's Out of Scope, item D).
- Both files land through this story's PR, pushed as `mildronize`. Adding
  `CARGO_REGISTRY_TOKEN`, triggering `publish.yml` for real after merge, and merging all happen
  separately, outside this story's own automation.
- Crate names confirmed unclaimed on crates.io as of 2026-09-24: `typdoc`, `typdoc-core`,
  `typdoc-fs`.

## 5. Release mechanics for v0.3.0

- Bump `version` in `typdoc-core/Cargo.toml`, `typdoc-fs/Cargo.toml`, `typdoc/Cargo.toml` (and
  `typdoc-testkit/Cargo.toml`, for workspace version parity — it's unpublished either way) from
  `0.2.0` to `0.3.0`.
- Bump every intra-workspace dependency version requirement that currently pins `"0.2.0"`:
  `typdoc-fs`'s `typdoc-core = { version = "0.2.0", ... }`, and `typdoc`'s own `typdoc-core`/
  `typdoc-fs` requirements — to `"0.3.0"`. This is load-bearing for the version-check step above:
  a stale requirement fails the workspace build, not just a version mismatch.
- CHANGELOG: add a v0.3.0 entry (existing file's format — check `CHANGELOG.md` at build time for
  the established shape).
- Skill's version line (currently states 0.2.0) → 0.3.0.
- README's tag-based install instructions → reference the new tag.
- No git tag is created by this story's automation — tagging happens separately, after merging
  and triggering the real publish.

## Testing Decisions

- **Wildcard exclusion (`namespaces.rs`):** unit tests on `resolve()` directly (existing test
  module pattern in that file) covering: order/last-match-wins (a later plain entry re-including
  after an earlier `!`), a `!` entry matching nothing (exact and glob, both silent — assert no
  report), a plain entry matching nothing (unchanged behavior, still asserted). Fixture-level:
  extend or add to `fixtures/valid`/`fixtures/broken` for the excluded-is-invisible behavior
  (`validate`/`list`/`get`/`refs` golden cases under `fixtures/output/`), following the existing
  `TYPDOC_REGENERATE_GOLDEN` workflow — never hand-edit a golden file.
- **Scope selection (`scope.rs`), item 3/4 + the `!`-not-supported-here test:** one test for
  `--namespace`/`TYPDOC_NAMESPACE` explicitly naming an excluded namespace (asserts the existing
  "not a namespace of this project" error, proving item 3/4 needs no new code), plus one for
  `--namespace '!story-1'` (asserts the existing "not a namespace name or a glob" syntax error,
  locking in that a leading `!` is rejected, not silently ignored or misread — decided
  2026-09-24).
- **State-orphan interaction — the gate for item 6 (2026-09-24): write these red against
  today's code first, before the fix, so the failure this story catches is on record.**
  1. A namespace with an existing state file that has already issued codes (e.g. up to `TK-3`)
     gets excluded via `!` → `validate`/`list`/`get`/`new` (in a *different* namespace) must all
     succeed — not exit on `config.state-orphan`.
  2. Remove the `!` (re-include the namespace) → `new` in that namespace issues `TK-4`, not
     `TK-1`, and the state file's bytes are unchanged from before exclusion to after re-inclusion
     (byte-for-byte compare) — proving exclusion truly never read or wrote it.
  3. A state file whose folder no longer exists and matches no entry (positive or `!`) at all
     still fires `config.state-orphan` — the true-orphan case stays caught.
- **M-24, discovery:** a fixture (or unit test) with a *plain file* named `.typdoc` (not a
  directory) at the project root — `discover()` must report no project there, not treat the file
  as one (2026-09-24 — the case `.is_dir()` vs `.exists()` actually tells apart). Plus a
  companion `fixtures/valid/` case (or reuse an existing minimal project) for "no `config.json`
  at all, still loads as version 1."
- **M-24, `collections.empty`:** `fixtures/broken/collections.empty/` per the `RULES` list's own
  convention. Since this is now a non-fatal `validate`-only `Finding` and not a `Config::load`
  error, the test that actually matters is the one proving it *doesn't* stop other commands
  (2026-09-24): a project with zero collections still returns a normal exit code from
  `list`/`get`, and only `validate` shows the `collections.empty` warning.
- **`mv` message fix:** existing `mv` test module in `project.rs` (search for the case-only-rename
  test) gets a sibling case for the identical-path scenario, asserting the new message text and
  the unchanged exit code 7.
- **crates.io workflow:** `publish-check.yml`'s `publish-dry-run` job is the gate/evidence itself
  (a green run on this story's own PR, id recorded in the report, plus `ci.yml`'s existing jobs
  still running unaffected — the split's own acceptance criterion) — not a job that needs a
  separate test to prove it, since a red run there fails the PR directly. `publish.yml`'s
  `workflow_dispatch` gate/version-check/`cargo publish --workspace` path is not exercisable until
  after merge (see section 4), so it has no pre-merge test of its own beyond `actionlint`/YAML
  validity if this repo has that already — check at build time.
