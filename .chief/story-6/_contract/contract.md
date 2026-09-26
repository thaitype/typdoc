# Contract

## Scope

Every comment in `crates/`: `//`, `/* */`, `///`, `//!` in `.rs` files (sources, tests, and the
`trybuild` fixtures), and `#` comments in the crates' `Cargo.toml` and `clippy.toml`. Each is
decided with the `comment-review` skill: keep, reduce, remove, or investigate.

- **Doc comments on items that appear on docs.rs** — public items reachable from the root of
  `typdoc-core`, `typdoc-fs` or `typdoc`, not `#[doc(hidden)]`, and the crate-level `//!` of
  those crates: may be shortened; may be removed when they only restate the item's name or its
  `#[error]` message, which docs.rs shows anyway. Decided: what `cargo doc --no-deps` renders
  for a published crate is the test when it is unclear. `typdoc-testkit` is `publish = false`, so
  none of its items count.
- **Everything else:** the skill in full, including removing a doc comment entirely.

## Citations

A comment that cites something through history (`ticket N`, `decision N`, `M-N`, a story path,
`.chief/`, `docs/design.md`, `docs/migrating-design/`, `docs/archived-design/`) is one of three
cases, decided by what the cited text says, not by its form:

1. **Design already in `docs/design/`** — cite the SPC by key (`SPC-7`). A catalog entry is
   cited through the SPC named in its `explained_by`.
2. **Design only in `docs/migrating-design/` or `.chief/`** — move it into an SPC first, then
   cite the SPC:
   - an existing SPC that covers the area, or a new one made with `typdoc new SPC "<title>"`
     (never by hand; the collection's counter lives in `.typdoc/state/default.json`);
   - the moved text describes what the code does today, not what a ticket once asked for;
   - `migrated_from:` names the frozen copy under `docs/archived-design/`, never the working
     copy under `docs/migrating-design/`, which is deleted once empty:
     `docs/archived-design/design.md#<anchor>`, `docs/archived-design/story-3-brief.md#<anchor>`,
     or `docs/archived-design/design-decision-phase-1/_tickets/<file>.md`. Text moved from
     `.chief/` names its `.chief/` path;
   - text moved out of a story brief or a decision ticket is rewritten to hold the public-text
     rule, not copied;
   - the moved text is deleted from `docs/migrating-design/`, as whole lines, leaving the file
     with its remaining sections; a file with nothing left is deleted. Text cited from `.chief/`
     is not deleted there: `.chief/story-N/` is a story's record, not a working copy.
3. **Process only** (who asked, which ticket, when, what a brief said) — drop the citation; keep
   any reasoning in the rest of the comment that still holds on its own.

If the text being moved disagrees with the code, neither side changes: both stay as they are, and
the mismatch goes on the Investigate list. A comment is different: one the code clearly
contradicts is corrected to the code (see "Every PR also shows"). Only the sections a comment cites move.

**Where a citation goes:** a test file that covers an SPC says so once, at its top
(`//! Covers SPC-7.` or `//! Covers SPC-7, SPC-9.`); a test file that covers none says nothing.
Source cites an SPC only where someone would otherwise change the code wrongly. Never on every
item.

## Rules committed to `.chief/_rules/_standard/`

In the pilot PR, as its first commit, before any comment changes:

- a new section, "A comment that cites design moves that design too", appended to
  `design-docs.md`;
- a new file `comments.md`.

The texts are the ones this story was given, tightened only in wording, not in meaning.

## Batches and PRs

One PR per batch, each reviewed before merge. The loop never merges.

| Batch | Scope |
|---|---|
| 1 — pilot | `crates/typdoc-fs`, `crates/typdoc-testkit`, `crates/typdoc-core/src/json_body.rs` |
| 2 | `crates/typdoc-core` (rest of `src/`, and `tests/`), split into PRs by module group; `project.rs` is a PR of its own. Grouping decided in the tickets |
| 3 | `crates/typdoc/src` |
| 4 | `crates/typdoc/tests`, split into PRs by command group. Grouping decided in the tickets |

The pilot must show all three citation cases; `json_body.rs` is in it for that reason. **Nothing
after the pilot starts until the pilot's judgment is accepted**, and a later batch applies what
that review changes. History citations are handled inside each module's batch, not repo-wide.

The pilot PR is opened from `story-6-comment-review` and carries `.chief/story-6/` as well.
Each later batch branches from `origin/main` after the batch before it has merged.

## Commit order in every PR

1. Rules (pilot only).
2. `docs/design/` and `docs/migrating-design/` changes.
3. Comment changes.

`typdoc validate` passes on the repository at every commit.

## Proof that Rust changed only in comments

Rust changes in this story that are not comments, each in its own commit:

- in `crates/typdoc-core/src/json_body.rs`, the test that asserts `MissingContentType` for a
  document with no frontmatter block is renamed to say so (its old name said the case is
  reported as its own); the assertion and all behavior are unchanged (ticket 03 PR);
- in `crates/typdoc-core/src/project.rs`, three `AlreadyExists` messages a user sees no longer
  end in a decision number (the meaning of each is unchanged, and no SPC key replaces it, since
  users do not read SPC keys), and one `#[allow]` reason string that described `body.mentions`
  as not built is brought up to date (ticket 03 PR, with the item above);
- in `crates/typdoc-core/src/namespace_lock.rs`, one test assertion message no longer names a
  decision number; its reason is unchanged (ticket 04 PR);
- in the `typdoc-core` read path, three strings no longer point at history: an assertion message
  in `refs.rs` drops a reference to a design section, a test assertion message in
  `namespaces.rs` drops a note about earlier behavior, and the fixed host name in
  `tests/common/mod.rs` no longer carries a ticket number; each meaning is unchanged (ticket 05
  PR);
- in `crates/typdoc-core/src/refs.rs`, the test that asserts `BadPrefix` for a body link with an
  import prefix and no configured imports is renamed to say so (its old name said the import form
  is not read); the assertion is unchanged (ticket 05 PR);
- in `crates/typdoc/src/registry.rs`, the `[reverse-scope]` entry of `KNOWN_GAPS` states the gap
  without pointing at the design; its meaning is unchanged (ticket 06 PR);
- in `crates/typdoc/tests/mv.rs` and `templates.rs`, an assertion message drops a decision number
  and an `ignore` reason drops the date of a CI run; each meaning is unchanged (ticket 07 PR);
- in `crates/typdoc/tests/state.rs`, a test named after a decision number is renamed to name only
  what it checks; its assertions are unchanged (ticket 07 PR);
- in the read-command tests of `crates/typdoc/tests`, six `ignore` reasons drop the date of a CI
  run and one panic message names the SPC instead of a `design.md` section; four test names that
  said how the behavior changed ("no longer", "now", "as it did") are renamed for what they
  check; no assertion changed (ticket 08 PR).

The proof for each of those PRs reports exactly its own differences and no others.

In every PR body, as a script anyone can rerun on this repository at the base and head commits
(included in the PR body in full, since it is not part of the repository), with its result:

- For every file under `crates/` that differs between base and head, comments are stripped
  from both versions and what is left is compared. Rust: `//` and nested `/* */` comments, doc
  comments included, are removed by a lexer that leaves string, raw-string, byte-string and
  char literals intact, and whitespace outside literals is collapsed. `Cargo.toml` and
  `clippy.toml`: `#` comments outside strings and blank lines are removed. The result must be
  identical for every file.
- It reads source text, not compiler output, so code under any `cfg` is covered.
- Decided: not `rustc -Zunpretty=expanded`. Its output keeps ordinary `//` comments, so a
  comment-only change shows up as a difference; checked with a planted comment.
- The method is shown able to fail before it is trusted: a planted one-token code change makes
  it report a difference, a comment-only change does not, and text after `//` inside a string
  literal is kept.
- Any other changed file under `crates/` (for example a `trybuild` `.stderr` expectation) is
  listed and read by hand: a `.stderr` file may change only in line and column numbers that
  moved with a comment. Any other change to one is a stop.

## Every PR also shows

- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and
  `scripts/test.sh` green, locally and on the ubuntu and macOS CI jobs.
- `git diff <base> -- docs/migrating-design/` is deletions only, apart from cross-references
  repointed to the SPC that now holds their target.
- Every SPC section moved in has `migrated_from:` and describes today's behavior.
- The public-text rule holds for everything added.
- **Corrected comments** in the PR body: each comment the code clearly contradicted, what it
  claimed, what the code does, and `file:line`. It is corrected to the code, or removed if
  nothing non-obvious is left.
- A **citation table** in the PR body, one row per removed citation: where it was at the base, what
  it pointed at, its case (1, 2 or 3), and the result (`SPC-N` cited, moved into `SPC-N` and
  cited, or dropped and why). A match of a citation-like word that is not a citation gets a row
  saying so.
- An **Investigate** list in the PR body: each mismatch where it is unclear which side is right
  (a comment or moved text against the code, or a test name against its assertion), what it
  claims, what the code does, and `file:line`. Nothing on it is fixed.
