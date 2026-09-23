## Destination

Story 3 ships v0.2.0: remove `design.rs` and all markdown-reading from code in favor of
typdoc-managed `docs/design/spec/` (prose) and `docs/design/catalog/` (JSON-body) documents,
archive the old design docs, fix the release blockers (stack overflow, CI, toolchain pin,
release mechanics), document the `number`-comparison limit, and **build text output (no
`--json`) for every command, always showing field names (M-10's principle).** M-10(a-h) and
M-10g are fully closed as of 2026-09-23 (ticket 8's Answer). Goal and contract are approved;
tickets 9-22 are written (`.chief/story-3/_tickets/`); build is underway on `story-3-catalog-and-release`.

**2026-09-23, two things surfaced once build was about to start, before any of the design.rs-adjacent
tickets ran:**
- **Ticket 23 (M-11), open with Mild:** ticket 6's `CAT` code for `docs/design/catalog/` conflicts
  with a coded collection's own rule (`match` must take `{key}` exactly once) against the four
  catalog documents needing fixed names. Aria's proposal to Mild: `catalog` uncoded, `spec` keeps
  `SPC`, field renamed `body_type` (snake_case). **Holds tickets 10, 11, 12, 22** until answered.
- **Ticket 24, open, no decider named yet:** a sweep for every reader of `design.md` (prompted by
  Aria catching `fixtures.rs` as a fourth one) found a fifth and much larger one —
  `crates/typdoc-testkit/src/shell_examples.rs` and `crates/typdoc/tests/shell_examples.rs` (831
  lines together) extract and actually run every shell example in `design.md` against a real
  binary. This is the same M-1 violation as `design.rs`, at a larger scale, not yet decided how to
  handle (hand-list examples, a structured fifth catalog document, or something else). **Adds a
  block on ticket 12** (12 can't honestly be "done" — nothing reads `design.md` — while this
  stands) and, transitively, on 13 (13 is now `Blocked by: 12`, added for the same reason: moving
  `design.md` away while any of these still read it turns them red, and a parallel build has no
  other reason to sequence 13 after 12).

Unaffected and can build now: 9, 14, 15 (after 14), 16-21.

**2026-09-23, M-10 (Mild): every text-output shape is decided**, under the principle that text
output always shows field names — no bare, unlabeled value:
- `get`, and (deliberately, replacing today's bare-key behavior) `set`/`new` both forms, print a
  labeled `name: value` block.
- `toc` is a table with a header row.
- Both forms of `mv` — plain, and (M-10h, deliberately replacing today's bare-key behavior)
  `--renumber` — print the destination's `get`-block, plus, always present: `rewritten: N refs in
  M documents` (a count), `unrewritten:` (count + one line per entry), `findings:`. `mv --json`
  gains `rewritten` as the full list, additive; no `--verbose` flag.
- `list` (M-10g, decided earlier the same day) gains a header row; `--ids` unchanged.
- `refs` and plain/`--schemas` `validate` needed no change — their `design.md`-fixed shapes
  already satisfy the principle.

M-10h's flag (was `mv --renumber`'s "same shape as plain `mv`" contested by `design.md`'s own
text for it?) was warranted — that claim was Aria's own addition, not something Mild had
actually seen — but decided anyway once put to him directly. Since `design.md` is archived and
frozen this same story (M-7), the correct current shape for `mv` (and every other changed
command) is recorded in `docs/design/spec/` and the user docs, not as an edit to the archived
copy.

**2026-09-23: Mild's answer to ticket 2 (M-1) reshaped the core** past the original
bare-JSON-file plan — see ticket 2's Answer and ticket 5's re-charted Question/Answer for the
final shape.

## Notes

- Remote schemas are out of scope entirely (Mild) — no fetch, no `lock.json`/`vendor/` writes, no
  project lock, no `pull`/`pull --check`. A remote schema with no pin stays
  `config.schema-unpinned`, as today.
- The four release blockers carried from stories 1-2 are IN scope for v0.2.0 (Mild asked for the
  minimum to release and accepted this list as "roughly this", per Aria) — charting is about HOW
  to fix/ship each, not whether. Toolchain pin and release mechanics turned out mechanical enough
  to be contract requirements directly, not tickets:
  - **Toolchain pin:** pin to `1.96.0`, the version the gate passes on today; CI must run on that
    pinned toolchain. CI's first green run is the proof — a bad pin fails loudly on day one, it
    cannot pass silently.
  - **Release mechanics:** crates to `0.2.0`, a changelog, README install from the tag instead of
    cloning `main`.
- **M-2, resolved 2026-09-23.** Not a wayfinder ticket (Aria's instruction); a contract-time item
  instead of one edited into the archived phase-2 register:
  - **M-2a** — the invalid `body.links` `ignore` glob, silently dropped today → **not in v0.2.0**,
    pushed to the next story. It already fails strict (the link is still checked and reported),
    so the only cost of waiting is a confusing-but-safe error, never a bad link let through.
  - **M-2b** —
    `docs/design/design-decision-phase-2/_tickets/22-comparing-numbers-beyond-a-primitive.md`
    (numeric compare beyond a primitive) → **in v0.2.0, documentation only.** The user docs state
    where exact `number` comparison ends (a value past what `f64` holds exactly compares wrong,
    silently). No exact comparison, no `validate` warning, no `--sort` tie-break — all descoped by
    Mild. Demo to cite: on `62e6335`, `typdoc list --where 'count>99999999999999999998'` returns
    nothing even when a document holds `99999999999999999999`. **`date`/`datetime` — flagged by
    Aria, 2026-09-23: ticket 22 itself left whether they're affected as an open, unmeasured
    question; the doc line covers only `number` unless this story measures `date`/`datetime`
    too.** Ticket 22 itself is not edited (it sits in the phase-2 register, which M-7 archives
    untouched) — this doc line is story 3's own contract item, not a change to that ticket.
- **M-3, closed** — `docs/design/design-decision-phase-1/_tickets/20-story-1-and-remote-schemas.md`'s
  wording ("v1 is the three stories together") no longer held once remote schemas were deferred
  out of this v1. Closed by M-7 (below): ticket 20 lives in an archived, unedited document that
  nothing rewrites; the README states plainly that v0.2.0 has no remote schemas, separately.
- **M-7 — old-docs archival, decided.** `design-decision-phase-1`, `design-decision-phase-2`, and
  `design.md` are **moved** (`git mv`) to `docs/archived-design/` — they no longer exist at their
  old paths — and never edited again there. A separate **copy** goes to `docs/migrating-design/`
  (a working copy from which text is deleted once it's
  been moved to its new place — `docs/design/spec/` or `docs/design/catalog/`, ticket 5/6). Story
  3's scope is removing all markdown-reading from code; moving every piece of content out of
  `docs/migrating-design/` this story is explicitly **not** required.
- Story 2 hit `/tmp` disk pressure on this machine (a shared tmpfs that other crews' scratch
  fills): the fix was `TMPDIR=/home/thw-home/.cache/typdoc-tmp scripts/test.sh`. CI must not
  depend on that machine-specific path — the workflow sets `TMPDIR` from the runner's own temp
  directory (ticket 4).
- **`scripts/check-public-text.sh` does not exist** on `main` or `story-2-write-commands` —
  removed deliberately, by the repo owner's own commit `340db02`, before the `v0.1.0` tag. Not
  restored (ticket 4). CI wires up the three gates that do exist; there is no public-text bullet
  to carry.
- `~/gits/thaitype/*` pushes as `mildronize`; the active `gh` account on this machine is a
  different one. Use the one-shot credential-helper pattern in crew memory
  `thaitype-repos-git.md`, never `gh auth switch`.

## Decisions so far

- [2: Generated-vs-committed comparison against markdown](../story-3/_tickets/2-doc-table-generate-vs-compare-rule.md)
  (M-1, Mild): violates the rule, out entirely — no generator, no compare against markdown in
  either direction. Code stops referencing `design.md` as spec; `design.md` is allowed to go
  stale. Reshapes ticket 5's whole approach — see there for the replacement design.
- [6: Folder names for the two new document kinds](../story-3/_tickets/6-new-doc-folder-names.md)
  (M-4, Mild): prose → `docs/design/spec/` (`SPC`); structured/JSON-body → `docs/design/catalog/`
  (`CAT`). Added requirement: a catalog document declares its body type in a frontmatter field
  (e.g. `json`); the central helper (ticket 5) validates from that field, never from the path.
  **`CAT` and the field's exact name are under revision — see ticket 23 (M-11), open.**
- [1: Root cause and fix for the long-ref-chain stack overflow](../story-3/_tickets/1-stack-overflow-on-long-ref-chain.md):
  `cyclic_nodes`'s inner `visit` (`refs.rs`) is a plain recursive DFS with no depth limit, run
  project-wide on every `acyclic` field by every command that touches `ref_project()` — a long
  **one-way** chain overflows the stack with no cycle needed at all. Reproduced directly (a
  synthetic chain via a temporary unit test, not through the CLI): passes at 5,000 nodes, aborts
  at 10,000+ on this machine's default stack, well inside a realistic project's size. Fix:
  rewrite `visit` as an explicit iterative DFS over a heap-allocated stack, same algorithm and
  output, no depth ceiling; rejected a depth-limit-plus-error since `acyclic` guards against
  cycles, not chain length, and a limit would be arbitrary. Sized for a build ticket.
- [3: Format of the structured spec file](../story-3/_tickets/3-spec-file-format.md): JSON — the
  data is hand-edited (nothing generates it), edited rarely, so zero-new-dependency and matching
  `config.json`/`lock.json` outweigh TOML's editing comfort at that frequency. Long prose (e.g.
  frontmatter-losses entries) stays short. Decided by Aria. *Superseded in shape, not in format,*
  *by M-1/M-4 (tickets 2, 6): this JSON is now the body of a `docs/design/catalog/` document, not*
  *a bare file — see ticket 5.*
- [5: Design.rs replacement — final shape](../story-3/_tickets/5-design-rs-replacement-mechanism.md):
  four `docs/design/catalog/` documents, one per concept (rules, commands, exit codes,
  frontmatter losses), not per current caller. The rules document holds one list of entries each
  carrying a `configurable` field, replacing today's two separate lists (which let an id sit in
  both or neither). A `pub` helper in `typdoc-core` reads a catalog document's JSON body,
  validated per its declared body-type field, never the path. The three current
  `typdoc_testkit::design::*` call sites (all dev-dependency/test-only) are rewired to it; `pub`
  is required regardless of ticket 7's answer, since two of the three call sites are integration
  tests in other crates. Decided by Aria, built on M-1/M-4.
- [7: Central helper's user-facing scope](../story-3/_tickets/7-central-helper-user-facing-scope.md)
  (M-9, Mild): internal only. The helper serves the tests; `typdoc validate`/`get` do not learn
  body types in v0.2.0 — designed later with plain `.json` document support.
- [4: CI gates, triggers, and environment](../story-3/_tickets/4-ci-gates-and-triggers.md):
  settled at **three** gates, not four — `scripts/check-public-text.sh` was found not to exist on
  `main` or `story-2-write-commands` (removed deliberately by the repo owner's own commit
  `340db02`, before `v0.1.0`, not restored). `scripts/test.sh`, `cargo fmt --check`, and `cargo
  clippy --workspace --all-targets -- -D warnings` run in CI on push and PR into `main`, `TMPDIR`
  set from the runner's own temp dir, and each gate is proven red on a throwaway branch before its
  green is trusted.
- [8: Text output shapes, M-10(a-h)](../story-3/_tickets/8-text-output-without-json.md) (Mild):
  `list` gains a header row (M-10g); `get` prints a labeled block; `set`/`new` (both forms) print
  the same labeled block, deliberately replacing `new` coded form's bare-key output; `toc` is a
  header-rowed table; both forms of `mv` (M-10h changes `--renumber` too) print the destination's
  block plus always-present `rewritten:`/`unrewritten:`/`findings:`, with `mv --json` gaining the
  full `rewritten` list additively and no `--verbose` flag. `refs` and `validate` needed no
  change.

## Not yet specified

- [23: Catalog's code, and the body-type field's name (M-11)](../story-3/_tickets/23-catalog-frontmatter-and-code-conflict.md) —
  open with Mild. Every reference in ticket 5, ticket 6, the contract, and the goal to `CAT` or to
  a `body-type` field is written against the *pre-M-11* assumption; once M-11 lands, a pass over
  all of them applies whatever it actually decided rather than assuming Aria's proposal was
  accepted as written.
- [24: `shell_examples.rs` reads `design.md`](../story-3/_tickets/24-shell-examples-read-design-md.md) —
  open, no decider named. Whether this becomes a hand-written list, a fifth structured catalog
  document, or something else is not decided; raised to Aria, not yet routed further.

## Out of scope

- Remote schemas entirely (fetch adapter, `lock.json`/`vendor/` writes, project lock,
  `pull`/`pull --check`) — Mild, carried from the brief.
- The invalid `body.links` `ignore` glob becoming a config error — Mild (M-2), pushed to the next
  story. It fails strict today, so nothing unsafe ships by waiting.
- `typdoc validate`/`get` learning a catalog document's body type — Mild (M-9), designed later
  together with plain `.json` documents.
- Exact comparison for numbers past what `f64` holds, a `validate` warning for them, or a
  `--sort` tie-break — Mild (M-2). v0.2.0 documents the limit only.
