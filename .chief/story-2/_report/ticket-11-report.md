# Ticket 11: `typdoc mv` within the project

Resolved. Commit `afe18f1`, merged `0d69a02` with twelve conflicting files against tickets 8 and
10 — most add-add unions, plus two real duplicates resolved by decision (below). `cargo fmt --check`
clean, `cargo clippy --workspace --all-targets -- -D warnings` clean, public-text check clean, all
re-run on the merged tree. `scripts/test.sh` with `TMPDIR` on a disk-backed folder: 921 passed / 0
failed / 1 ignored (882 before this ticket).

## Outcome

done

## What it does

`Project::mv` moves an uncoded document within one project and rewrites every ref this project
holds to it — frontmatter and inline body links, in every namespace — before any lock is taken (a
read never locks), then executes under lock. Every content change is prepared as a temp file first;
the renames happen in one run, with the document's own move always last (`mv::commit`, a new pure
module, proved deterministically against a fake staged to stop at exact operation counts). A coded
document, or a move across a coded collection's boundary, is refused outright (exit 1): a coded
collection's `match` fixes the file's path entirely by its key, so a coded source can never reach a
new path under a plain `mv`. Destination-exists and same-file-by-identity are both refused at exit 7
with nothing written. A move onto a schema the document fails is carried out, exits 0, and reports
the rejection in `findings`. `--json` adds `unrewritten` (`refs --reverse`'s own shape, with a
`reason`) and `findings`.

## Checked by running

- The two-phase prepare-then-rename mechanism, against a fake file system stopped at four different
  points (a full run, mid-prepare, between two content renames, at the document's own move) —
  the seam decision 1's promise is actually about.
- A stop left half done: the state it leaves is built by hand, `validate` reports it, and the
  identical `mv` command completes it and leaves `validate` clean.
- Each refusal (destination exists, same-file identity, coded/collection-boundary) leaves every file
  byte-identical, per its own test.
- A schema-rejecting destination exits 0 with the rejection in `findings`, checked by a CLI test and
  a golden.
- Refs held by another project are reported with the project named — not built as reachable in this
  ticket, since it needs `[reverse-scope]`, a known gap this story does not touch (below).
- Seventeen pure functions (relative-path recomputation, link-form preservation, splicing) are
  unit-tested directly; one caught a real bug in its own test before the fix (a confusion between a
  link's written form and its decoded one). A real bug was also caught by the CLI tests: the
  destination's parent folder was not created before the final rename, fixed with `create_dir_all`.
- The existing write ban reaches the new code: `std::fs::write` planted in library code and,
  separately, in test code, both refused; the crate's own report from ticket 1 already established
  the same call inside `typdoc-fs` is not caught, not re-demonstrated here since no `typdoc-fs` code
  was touched.

## Decision

- **Issue:** merging against tickets 8 and 10 surfaced two real duplicates, not textual conflicts.
  First: `Env::hostname()` was independently added by ticket 10 (`io::Result<String>`, reading
  `/proc/sys/kernel/hostname`, failing the whole lock acquisition on any read error) and by this
  ticket (infallible `String`, via a new `hostname` crate dependency, falling back to a placeholder).
  Second: the golden harness's write-staging gap was independently fixed by both, ticket 10 building
  an inline `FixtureSpec` and calling `staging::stage`, this ticket adding a purpose-built
  `staging::stage_command` helper.
- **Options considered (hostname):** a throwaway decision-support agent laid out three: infallible +
  no new crate (a direct `/proc` read with a fallback); infallible + the `hostname` crate (this
  ticket's own diff, unchanged); fallible signature kept, with the caller swallowing the error
  instead of propagating it.
- **Chosen:** infallible, no new crate. A hostname only ever decorates a lock-contention message, the
  same reasoning ticket 3's own `pid_alive` already used to read `/proc/<pid>` directly and answer
  safely on failure — extending that same precedent to the hostname keeps the two consistent, and it
  means a read failure (an unusual container, a non-Linux platform, a permission oddity) no longer
  blocks every write command from acquiring a lock at all, which ticket 10's fallible version did.
  The `hostname` crate dependency and its stack-list line are removed. For the golden helper, ticket
  11's `stage_command` was kept as the more reusable primitive — it takes a directory and a command
  directly, with no fixture-spec construction needed at the call site.

## Notes

- **`imported-project` and `mention`, two of the three `unrewritten` reasons decision 17 names, are
  wired into the shape but cannot be produced by this ticket.** `mention` only ever names a coded
  document, and a plain `mv` only ever moves an uncoded one — the two never meet. `imported-project`
  needs reverse lookup into an imported project, which is `[reverse-scope]`, a known gap
  `out-of-scope.md` says this story does not touch. `links-rule-off`, the third reason, is built and
  tested.
- Reference-style body links (`[text][ref]`) and their definitions are not rewritten, only inline
  links and images — a scope cut, not a mistagged case; a stale reference-style link is still caught
  afterward by `body.links`, the same as any other broken link.
- `mv` rewrites refs held by other documents pointing at the moved one, not the moved document's own
  outgoing refs, matching the design's literal wording.
- `git-common` lock mode is not wired — it needs `git rev-parse --git-common-dir`, spawning a
  process, which no write command has built a path for yet.
- `--lock-timeout` is wired here (default 5 s), since in the branch history this ticket built
  against, no earlier write command's changes had merged yet — this ticket turned out to be the
  first to close that thread in practice.
- A document moved out of every collection prints `document` with `code`, `collection` and `schema`
  left out entirely (not `null`, not empty text), since those fields are non-optional strings and a
  real collection name is never empty.
