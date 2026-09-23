# Contract

This contract does not restate the design. `docs/design/design.md` moves to
`docs/archived-design/design.md` this story (M-7) and is never edited again there — once
archived it is a record of what was decided and when, not something a user or a program follows.
The living spec, for anything this story touches, is `docs/design/spec/` (prose) and
`docs/design/catalog/` (structured); where this story changes a shape on purpose (the write
commands' text output, `list`'s header row, `mv`'s new summary lines), the correct shape is
recorded there, and in the user docs that show it (`docs/commands.md`, `docs/getting-started.md`,
`README.md`) — never as an edit to the archived `design.md`. Wayfinder's
`.chief/story-3/_map.md` is the record of every M-numbered decision this contract draws on; a
ticket named below is `.chief/story-3/_tickets/<N>`. How the story is tested is in
`testing-decisions.md`.

## What story 3 implements

| Part | Where it's specified | For this story |
| --- | --- | --- |
| Spec source | Was `docs/design/design.md`'s five tables, read by `crates/typdoc-testkit/src/design.rs` | `design.rs` removed entirely. Four `docs/design/catalog/*.md` documents (JSON-only body) replace it: `rules.md`, `commands.md`, `exit-codes.md`, `frontmatter-losses.md`. |
| Central helper | New (ticket 5, M-6) | A `pub` function in `typdoc-core` that reads a typdoc document's JSON-only body into typed data, deciding whether/how to validate from a `body-type` frontmatter field, never from the document's path. Internal only (M-9): no `validate`/`get` behavior change. |
| Old docs | `design.md`, `design-decision-phase-1/`, `design-decision-phase-2/` | Moved (`git mv`) whole to `docs/archived-design/`, frozen there. Separately copied to `docs/migrating-design/`, a working copy emptied as content moves to `docs/design/spec/` or `docs/design/catalog/`. Emptying it fully is not required this story (M-7). |
| Stack overflow | `crates/typdoc-core/src/refs.rs`, `cyclic_nodes`'s `visit` | Rewritten as an explicit iterative DFS over a heap-allocated stack. Same output for a two-node cycle, a self-loop, a no-cycle chain, and a cycle-with-tail (ticket 1). |
| Text output | `typdoc new/get/set/toc/refs/validate/mv`, JSON output | See the table below. Every command's error path passes its real `json` flag to `failure`/`failure_text` instead of a hard-coded `true`. |
| `list`'s header row | `typdoc list`, already built | A header row added above the existing table (M-10g); `--ids` unchanged. |
| CI | New | A workflow running `scripts/test.sh`, `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings` on push and PR into `main`. `TMPDIR` set from the runner's own temp directory. Three gates, not four (ticket 4 — `scripts/check-public-text.sh` and its rule are gone, deliberately, since before `v0.1.0`). |
| Toolchain pin | New | `rust-toolchain.toml` pinning `1.96.0`, the version the three gates pass on today. |
| Release mechanics | New | Every crate's `version` to `0.2.0`; a changelog; README install instructions point at the `v0.2.0` tag instead of a clone of `main`. |
| `number`-comparison doc line | User docs | States where exact `number` comparison ends, with the `count>99999999999999999998` / `99999999999999999999` demo on `62e6335` (M-2b). No `date`/`datetime` claim unless this story measures it. |

Not built here: anything under remote schemas (fetch adapter, project lock, `lock.json`/
`vendor/` writes, `pull`, `pull --check`); moving every remaining line out of
`docs/migrating-design/`; the `body.links` invalid-glob-to-config-error change (M-2a, next
story); exact numeric comparison, a `validate` finding for it, or a `--sort` tie-break (M-2b);
`validate`/`get` learning a catalog document's body type (M-9).

## Text-output shapes (M-8/M-10)

| Command | Shape |
| --- | --- |
| `get` | A labeled block, one `name: value` line per field: `path`, `collection`, `schema`, `namespace` (and `key` when coded), then frontmatter fields in file order. A `number` field prints with the document's own digits (the write path's existing `[number-text]` rule). |
| `set` | The same labeled block as `get`, for the document as it stands after the write. |
| `new <path>` | The same labeled block as `get`, for the newly created document. |
| `new <CODE>` | The same labeled block as `get` — **changed** from today's bare key on stdout, accepted knowingly (M-10(f)); a caller that wants just the key reads it out of `--json`. |
| `toc` | A table with a header row: `line`, `end`, `level`, `heading`; one row per heading. |
| `mv` (plain) | The destination's `get`-shaped block, then, always present as their own lines: `rewritten: N refs in M documents` (a count, text only); `unrewritten:` with its own count and one line per entry (project, document, field, written form); `findings:`, listing its entries or the word `none`. |
| `mv --renumber` | **Changed (M-10h), decided:** the same labeled block as the other write commands — no longer a bare key — plus the same `rewritten:`/`unrewritten:`/`findings:` lines as plain `mv` above. |
| `list` | The existing table, plus a header row: `path` (or `key` for a coded collection), `title`, each `--where` field. No header when the result is empty. `--ids` unchanged (one key/path per line, no header). |
| `refs` | Unchanged from the design's worked example — already names each ref's field inline. |
| `validate` (plain, `--schemas`) | One line per finding, from the same finding shape `--json`/`--audit` already expose. |
| `validate --audit` | Unchanged — already built. |

**`mv --json` gains `rewritten` (M-10h), additive to its existing shape:** the full list of what
changed — one entry per rewritten ref, each `{document, field, before, after}`. Text prints only
the count (`rewritten: N refs in M documents`); the detail lives in `--json`, which is also why
there is **no `--verbose` flag** — a caller who wants the detail without a diff reads `--json`,
and `git diff` already shows every changed line for a human at the terminal. This is a design
change (both forms of `mv` gain a JSON field neither has today), recorded in `docs/design/spec/`
and in `docs/commands.md`, not left implicit in this contract alone. `mv`'s existing goldens
(both forms) are rewritten for it — see `testing-decisions.md`.

## Decisions that bind the story

| Ticket | Binds story 3 on |
| --- | --- |
| 1 | The stack-overflow root cause (`refs.rs`'s `visit`) and the iterative-rewrite fix; rejects a depth-limit-plus-error |
| 2 (M-1) | No program reads or compares against markdown for spec; `design.md` may go stale |
| 3 | The catalog documents' body format is JSON |
| 4 | Three CI gates, not four; push and PR triggers; `TMPDIR` from the runner; each gate proven red before green counts |
| 5 | Four catalog documents split by concept, not by caller; the rules document is one list with a `configurable` field, not two lists; the helper is `pub`, not `pub(crate)` |
| 6 (M-4) | `docs/design/spec/` (prose, `SPC`) and `docs/design/catalog/` (JSON-body, `CAT`); a catalog document's `body-type` field, not its path, drives validation |
| 7 (M-9) | The central helper stays internal to the tests; `validate`/`get` do not learn body types this story |
| 8 (M-8/M-10/M-10g/M-10h) | The field-names principle and every text-output shape above, including `mv --renumber`'s change and both `mv` forms' `rewritten:`/`unrewritten:` lines |

## What this contract decides that the design does not

1. **The catalog documents' exact JSON shape.**
   - `rules.md`: `{"rules": [{"id": "schema.valid", "configurable": false}, ...]}`, one entry per
     rule id from both of today's two tables, `configurable` replacing which table an id was in.
   - `commands.md`: `{"commands": ["new", "get", "list", "set", "toc", "refs", "mv", "pull", "validate"]}`.
   - `exit-codes.md`: `{"codes": [0, 1, 2, 3, 4, 5, 6, 7]}`.
   - `frontmatter-losses.md`: `{"losses": ["Comments, anywhere in the block", ...]}`, the same
     short strings `design.rs`'s fixture holds today, kept short per ticket 3's answer.
   Field names are this contract's own choice, not derived from an existing convention (typdoc's
   own field-naming is mixed: `inlineCode`/`fencedCode` in config, `blocked_by` in a fixture
   schema). Whichever the build ticket finds cleaner to consume from `serde_json`, it names
   consistently across all four documents rather than matching each to a different source's
   style.
2. **The `body-type` frontmatter field.** Named `body-type` (matching typdoc's own kebab-case
   config keys, e.g. `--lock-timeout`), value `"json"` today — the only value the central helper
   recognizes in this story. A schema for catalog documents declares this field (type `enum`,
   `values: ["json"]`, required); the four catalog documents are the schema's only members.
3. **This repository needs a `.typdoc/` project (M-5).** One namespace covers `docs/design/spec/`
   and `docs/design/catalog/` as two collections, each with its own schema (prose documents need
   no fields beyond what typdoc itself requires; catalog documents need `body-type`). Managed with
   the `v0.1.0` binary; if a bug in it blocks a step, that step is done by hand and the gap is
   named in the closing report, per M-5.
4. **Which error call sites move off the hard-coded `true`.** Every `failure(true, ...)` /
   `failure_text(true, ...)` call site inside `new`, `get`, `set`, `refs`, `toc`, and `mv` (plain
   and `--renumber`'s own error path) is audited and changed to the command's real `json`
   variable as part of building that command's text output — not a separate pass, so a command's
   text-mode work isn't done until its errors are text too.
5. **Where a changed shape is written down, now that `design.md` is archived and frozen (M-7).**
   `design.md` is moved, whole and unedited, and never touched again — not even to correct a
   worked example this story makes wrong (Mild, restating M-7: *"Design.md จะถูกเก็บไม่แตะใน
   docs/archived-design"*). Every shape this story changes on purpose (`new` coded form, `list`,
   both `mv` forms, and the rest of the text-output table above) gets its correct, current
   description in `docs/design/spec/`, and wherever the user docs already show that command's
   output (`docs/commands.md`, `docs/getting-started.md`, `README.md`) those examples are updated
   to match. A reader is never left following a worked example in an archived document as if it
   were current — the current one lives in `docs/design/spec/` and the user docs, not in
   `docs/archived-design/`.

## Not decided by this contract

- **Whether `date`/`datetime` are affected by the same `f64`-exactness limit as `number`.**
  Ticket 22 left it unmeasured; this story measures it only if a ticket is built for it, and the
  doc line names only what's measured.
- **The exact wording and placement of the `number`-comparison doc line and the `body-type`
  schema/collection files** — left to the ticket that builds each, consistent with this
  contract's shapes above.

## Constraints

- The workspace and toolchain are otherwise unchanged: Rust 2024, `crates/typdoc-core` the
  library, `crates/typdoc` the binary, `crates/typdoc-fs` and `crates/typdoc-testkit` support
  crates. `rust-toolchain.toml` (new) pins `1.96.0`.
- Before every commit: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D
  warnings`, and `scripts/test.sh` (not bare `cargo test`, which has four red tests by design
  behind the `test-stand-in` feature) pass. `TMPDIR` set off this machine's full `/tmp` locally;
  CI uses the runner's own.
- Non-test code in `typdoc-core` may not `unwrap`, `expect`, `panic!`, `unreachable!`, `todo!` or
  `unimplemented!` without an `#[expect(...)]` whose reason is the evidence — the same lint
  story 1 and 2 held to.
- Every command still has `--json` and uses the exit codes the design's table defines (project
  rule 5) — this story adds no new exit code. `mv --json`'s new `rewritten` field is additive:
  every field `mv --json` prints today keeps printing, unchanged in shape or meaning.
- Decision tickets and build tickets live in the same story register, not two (a standing rule,
  restated because this story's wayfinder phase produced both kinds in `.chief/story-3/_tickets/`
  already).
- `~/gits/thaitype/*` pushes and opens PRs as `mildronize`, via the one-shot credential-helper
  pattern (never `gh auth switch`, which is global, shared state).
