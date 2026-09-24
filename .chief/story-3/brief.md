# Story 3 — brief

Assigned by Mild (2026-09-23), directed by Aria. Mina runs it: start with `/chief-wayfinder`,
then plan and build as your own role. Escalations come to Aria; decisions that are Mild's go to Mild.

Mild's words: *"เริ่มจากประมาณนี้คับ story-3 เดี๋ยวให้ mina เรียก chief-wayfinder"* — the scope
below is "roughly this", which is why it goes through wayfinder rather than straight to a goal.

## Release target

Story 3 ships **v0.2.0**. `v0.1.0` is already tagged at the story 2 merge (`62e6335`).

## Out of scope — decided by Mild

**Remote schemas are deferred.** No fetch, no `lock.json`/`vendor/` writes, no project lock,
no `pull`/`pull --check`. A remote schema with no pin stays `config.schema-unpinned`, as today.

## Core of the story — the spec must stop being read from markdown

Mild's rule, absolute: **no program reads markdown to extract spec as machine data — not to
import it, not to check against it.** Comparing is the same as importing. Existing code that
does this is debt to remove, not an exemption because it works.
What to do instead: spec lives in a structured file as the truth → the table in the doc is
generated from that file → tests compare code against the file.

Today `crates/typdoc-testkit/src/design.rs` parses `docs/design/design.md` and extracts five sets:

| extracted | from design.md |
|---|---|
| always-on rule ids | table in §Validation rules |
| configurable rule ids | the other table in §Validation rules |
| command names | `### typdoc <name>` headings in §Commands |
| exit codes | exit code table |
| frontmatter losses on write | table of losses |

Used by `typdoc-core/src/frontmatter.rs`, `typdoc-core/tests/rules.rs`, `typdoc/tests/coverage.rs`.
Done means `design.rs` is gone and nothing reads `design.md`.

## Release blockers carried from stories 1–2

- **Stack overflow on a long ref chain** — a real crash. Story 1 residue; not re-verified today.
- **No CI at all.** The docs say "held in place by a test" everywhere; today that means "if someone runs the gate".
- **Toolchain not pinned** — no `rust-toolchain.toml`; builds today on 1.96.0.
- Release mechanics: crates to 0.2.0, a changelog, README install from the tag instead of cloning `main`.

## Fog for the wayfinder map — not decided by anyone yet

1. Format of the structured spec file (Aria suggested TOML).
2. How the doc table is generated, and whether a test that compares the committed generated
   table with fresh generator output is within Mild's rule. **Mild has not confirmed this line —
   ask, do not assume.**
3. Ticket 20 (`docs/design/design-decision-phase-1/_tickets/20-...`) says v1 is three stories
   together; with remote schemas deferred that is no longer true. How the record and README say so.
4. Whether these go into v0.2.0 (Aria recommends yes, both small):
   - `body.links` `ignore` glob that is invalid is dropped silently → config error.
   - Ticket 22 (numeric compare beyond primitive), still in the old phase-2 register:
     Aria proposed not building exact comparison — state the bound in the design, `validate` warns,
     `--sort` gets a stable tie-break. Not decided.

## Explicitly deferrable

`[reverse-scope]` and `[import-anchor]` known gaps; body-link scan cost; prebuilt binaries / crates.io.

## Rules from Mild still in force

- Decision tickets and build tickets live in the same story — one register, not two.
- User-facing docs don't point into code; docs are linked, and link text is human words, not file names.
- `~/gits/thaitype/*` uses gh account `mildronize`.
- Bare `cargo test` has 4 red tests by design (stand-in behind feature `test-stand-in`); run `scripts/test.sh`.

## Mild's decisions — 2026-09-23 (supersede the fog above where they overlap)

**M-1 — comparing a generated table against design.md violates the rule.** Mild: *"M1 ถือว่าผิดกฏ ผมจะยกเลิกใช้ไฟล์ design.md โดยให้กฏที่ code ใช้ ref ให้อ่านจาก json ครับ · หมายความว่า design.md สามารถ outdate ได้คับ"*
No generator, no compare against any markdown. Code reads its spec from JSON; design.md is allowed to go stale.

**New doc layout, two kinds** (converted from the old design docs):
1. Prose rules code must not read → `docs/design/<xxxx>/*.md`, typdoc documents, managed through the typdoc CLI.
2. Structured rules code reads → `docs/design/<yyyy>/*.md`, typdoc documents whose body is JSON only.
Folder names and codes are **pending M-4** (Aria proposed `spec`/`SPC` and `catalog`/`CAT`). **Do not create either folder until Mild picks.**

**M-5 — typdoc version for managing these docs:** the latest tag, `v0.1.0`. If a typdoc bug gets in the way, skip that step and defer validating with typdoc until the bug is fixed. (The repo has no `.typdoc/` yet — it has to be set up.)

**M-6 — reading the JSON body:** a central helper in typdoc handles "md document whose body is JSON". Supporting plain `.json` documents instead of md is future work — **not designed now**.

**M-7 — the old documents and story-3 scope:**
- Copy ALL of `design-decision-phase-1`, `design-decision-phase-2`, `design.md` into `docs/migrating-design/`. In story 3, text is deleted from `docs/migrating-design/` once it has been moved to its new place.
- The original documents are not edited; they move to `docs/archived-design/`.
- Story 3 focuses on removing **all** markdown-reading from code. Moving every piece of content out is **not** required.

**M-3 is closed by M-7** — ticket 20 lives in an archived, unedited document; nothing rewrites it. README still says plainly that v0.2.0 has no remote schemas.
**Still open with Mild:** M-2 (`body.links` ignore glob, ticket 22 — in v0.2.0 or not), M-4 (folder names).

**M-4 — folder names (2026-09-23):** Mild: *"ชื่อ folder เห็นด้วย ทั้ง spec, catalog ครับ ส่วน ใน catalog ให้มี field สำหรับบอก body type เช่น json เพื่อให้ helper รู้ว่าต้อง validate JSON ไม่ใช่ ดูจาก path ว่าต้อง validate หรือไม่"*
- Prose → `docs/design/spec/`, structured → `docs/design/catalog/` (codes as proposed: `SPC`, `CAT`).
- Catalog documents carry a frontmatter field declaring the body type (e.g. `json`). The helper decides whether and how to validate the body **from that field, never from the path**.
**Still open with Mild:** M-2 only.

**M-9 — central helper visibility (2026-09-23):** Mild agreed with (A): internal, used by the tests to read catalog documents (strict JSON parse of the body, driven by the body-type field). `typdoc validate`/`get` do **not** learn body types in v0.2.0 — that is designed later together with plain `.json` documents.

**M-2 — (2026-09-23):** Mild agreed with Aria's proposal:
- `body.links` `ignore` glob that fails to parse and is dropped silently → **not in v0.2.0**, next story. (It fails strict — the link is still checked and reported — so it confuses but never lets a bad link pass.)
- Ticket 22 (numbers past what f64 holds exactly) → **in v0.2.0: document the limit only**, in the user docs, stating where exact comparison ends. No exact comparison, no `validate` warning, no `--sort` tie-break in this story. Demo on `62e6335`: `list --where 'count>99999999999999999998'` returns nothing when a document holds `99999999999999999999`.

**M-8 — text output (2026-09-23):** Mild: *"M-8: output แบบไม่ใส่ --json ยังไม่ได้ทำ จะเอาเข้า v0.2.0"*
Today every command without `--json` answers `the output without --json is not built yet` with exit 1 (`crates/typdoc/src/cli.rs:248`, `:263`, `:211`). **In v0.2.0, commands work without `--json`.**

**M-10 — text output shapes (2026-09-23), in progress:**
- Principle from Mild: *"อยากให้ เวลาที่ไม่ใช้ JSON แล้วยังเห็นชื่อ field อยู่นะ ไม่งั้น งงเลย"* — text output shows field names; no bare unlabeled values.
- **M-10g decided** (Mild: *"ดีคับ เห็นด้วย"*): `list` text output gets a header row (`path`, or `key` for coded collections, then `title`, then each `--where` field). No header when the result is empty (exit 0 as today). `list --ids` is unchanged — one key/path per line, no header, for pipes.
- M-10a–f (get, set, toc, new path-form, mv plain, new coded form) still with Mild. Aria's proposal: write commands print the same labeled block as `get`; `toc` is a table with a header row.
- **M-10a–f decided** (Mild: *"ตามข้อเสนอได้เลย"*), all under the field-names principle:
  - a `get`: labeled block, one `name: value` per line — `path`, `collection`, `schema`, `namespace` (and `key` when coded), then frontmatter fields in file order; numbers printed with the document's digits.
  - b `set`, d `new <path>`, f `new` coded form: print the same labeled block as `get` for the resulting document. (f changes today's bare-key output — accepted; callers wanting just the key use `--json`.)
  - c `toc`: table with a header row `line  end  level  heading`, one row per heading.
  - e plain `mv`: the destination's `get` block, then `unrewritten:` and `findings:` always present, each listing its entries or `none`. `mv --renumber` follows the same shape (it is a write command too) — flag to Aria if the design's "prints the new key, and nothing else" makes this contested.
**M-10 fully closed.** No questions open with Mild.
- **M-10h decided** (Mild: *"ดีคับ"* / *"เห็นด้วยคับ"*, 2026-09-23): `mv --renumber` uses the same labeled block as the other write commands (not the bare key). Both `mv` forms also print a move summary: `rewritten: N refs in M documents` (count only in text), and `unrewritten:` with count and one line per entry (project, document, field, written form). `mv --json` gains `rewritten` as the **full list** (document, field, before, after) — text prints the count, JSON carries the detail. **No `--verbose` flag** (detail is in `--json`; `git diff` shows every changed line).
- Field-names principle **confirmed by Mild** (2026-09-23): the inbox copy of that message reads "ไม่อยาก …", the chat reads "อยากให้ …" — Mild: *"อยากให้ เวลาที่ไม่ใช้ JSON แล้วยังเห็นชื่อ field — อันนี้ถูกครับ"*. The principle stands as built into the contract.

**M-11 — frontmatter of spec/catalog (2026-09-23), decided.** Mild: *"งั้นเอา content_type ละกัน"* · *"เห็นด้วยคับ"*
- a: `docs/design/spec/` is a **coded** collection, code `SPC`, files `spec/SPC-<n>.md`. `docs/design/catalog/` has **no code**; path-identified: `catalog/rules.md`, `commands.md`, `exit-codes.md`, `frontmatter-losses.md`. (A coded collection's `match` must be `{key}`, so `CAT` + named files was impossible — the contract's `CAT` is withdrawn.)
- b: spec fields — `title` (string, required), `status` (enum `draft|active|superseded`), `superseded_by` (ref → SPC, only when superseded), `migrated_from` (string, source location in the archived docs). Catalog fields — `title` (required), `content_type` (enum `["json"]`, required), `explained_by` (ref → SPC, written as a key, e.g. `SPC-4` — not a path).
- c: the body-type field is named **`content_type`** — replaces `body-type` everywhere in the contract and tickets.

**M-12 — pushing (2026-09-23).** Mild: *"push ขึ้น branch ได้คับ"* — approved for **throwaway branches used to prove CI red-before-green**, deleted after. Not a release of the story branch, not a PR, not main.

**CI runners (2026-09-23).** Mild: *"ให้ทำ github actions ที่ ubuntu กับ mac นะครับ"* — CI runs every gate on **both** `ubuntu-latest` and `macos-latest`. Windows not asked.

**M-14 — `toc --depth` with nothing at that depth (2026-09-24).** Mild: *"M-14 เห็นด้วยคับ"* — stdout stays empty and exit 0 (the list precedent), but when the document HAS headings and none survive `--depth`, text mode prints one line to **stderr**, e.g. `no headings at depth ≤ 1 (3 headings are deeper)`. A document with no headings at all prints nothing. `--json` unchanged.

**M-15 — token (2026-09-24).** Mild: *"M-15 ทำให้แล้วคับ"* — the `mildronize` fine-grained PAT now has Workflows: Read and write for `thaitype/typdoc`. The CI red-before-green proof on throwaway branches (M-12) can run.

**M-13 — `shell_examples` (2026-09-24).** Mild: *"เห็นด้วยกับข้อ 1 ก็คือ ไม่มี shell example .md เพิ่มใช่มั้ย"* — confirmed by Aria: yes. Option 1: the hand-declared example list already in `crates/typdoc/tests/shell_examples.rs` becomes the list; the sh/bash runs against the stand-in stay. **Removed:** `typdoc-testkit/src/shell_examples.rs` (the markdown extractor), the test that cross-checks declared examples against `design.md`, and the quoting-paragraph span test. **No `catalog/shell-examples.md`.** Examples that were "safe" (no unsafe char) and only reached the harness via extraction: hand-list the ones worth keeping or drop — the harness must not read markdown to find them.

**`explained_by` is `ref[]` (2026-09-24).** Mild: *"explained_by เราไม่ทำเป็น ref[] ไม่ดีกว่าเหรอ"* — agreed: `explained_by` is `ref[]` → SPC, written as keys (e.g. `[SPC-4, SPC-7]`), **optional** (spec documents may not exist yet during migration).

**M-16 — header rows for `refs` and `validate` (2026-09-24).** Mild: *"M-16: เพิ่มหัวตารางแบบเดียวกับ list และ toc — เพิ่มคับ"*. Text `refs` prints a header row (`target`, `field`, …the columns it already prints); text `validate` prints a header row with `rule` moved before the long `message` (e.g. `path  level  rule  message`). Same conventions as `list`/`toc`: no header on an empty result.

**M-17 — push and PR (2026-09-24).** Mild: *"เสร็จแล้วบ push branch แล้ว เปิด PR เข้า main เลยคับ"*. After M-16 is merged and the gates are green: push `story-3-catalog-and-release` and open a PR into `main`. Merging stays with the reviewer (ruleset requires another account's approval).

**M-18..M-20 — fix in PR #2 (2026-09-24).** Mild: *"ผมคิดว่าแก้ทั้งหมดไปเลยก็ได้คับ M-18-M-20"*. Found by Aria while writing the typdoc skill (all reproduced on 1a8b4ee, release build); all three predate story 3, the design already says the right behavior, so these are fixes to the design, not decisions:
- **M-18** — `set`/`new` must refuse a write that forms a cycle on an `acyclic` field (design §Refs "Write-time checks (`new`, `set`): … no cycle forms on `acyclic` fields"). Today `set WF-5 blocked_by=WF-1` with WF-1 already blocked by WF-5 exits 0; only `validate` reports `refs.acyclic`. Exit 2, nothing written, finding `refs.acyclic` in `details`.
- **M-19** — `--set` / `set field=value` escaping per design §Query: `\*` is a literal star, an unescaped `*` is an error, `\,` a literal comma (commas split only array fields), `\\` a backslash, `\` before anything else is an error. Today values are stored literally (`a\*b` keeps its backslash, `x*y` accepted).
- **M-20** — in a multi-namespace scope, `list` (text) and `list --ids` print a name another command accepts (design §Arguments: `namespace:key` when the project has several namespaces). Today both print bare `WF-1` twice across `story-1`/`story-2`, which then exits 1 as ambiguous.

**M-21 / M-22 / M-23 (2026-09-24).** Mild: *"เห็นด้วยทั้งหมดคับ"*.
- **M-21** — narrow M-18 (ticket 30): refuse only a write that **forms** a new cycle (design: "no cycle forms"). A write to a document that already sits on a cycle, which does not create a new one (e.g. `set WF-1 title=…`), must succeed; `validate` keeps reporting the old cycle. Removing a ref that breaks a cycle stays allowed.
- **M-22** — `mv` must list plain-text mentions of the moved key in `unrewritten` with reason `mention` (design §JSON output, `mv`). Repro on 09c7b7b: `body.mentions` at warn, a body saying "WF-3", `mv story-2:WF-3 --renumber story-1` → `unrewritten: []`, while `validate` then reports `body.mentions WF-3 not found`.
- **M-23** — the docs rewrite and the agent skill (ticket 33, built by Aria) are in this story/PR; landed as 4dee53c.

**M-24 — deferred (2026-09-24).** Mild: *"typdoc/.typdoc/config.json มีแค่ { "version": 1 } ถ้ามีแค่นี้ผมคิดว่าไม่ต้องมี config ก็ได้นะ"* → *"งั้นค่อยก่อนก็ได้คับ"*. Making `config.json` optional means finding a project by its `.typdoc/` folder and treating a missing file as `{ "version": 1 }`. Not in v0.2.0; first candidate for the next story.
