## Destination

`docs/design.md` plus this map's Decisions so far is enough for `/chief-plan` to write typdoc v1's goal and contract without re-grilling: every implementation-level gap in the design is decided, and `docs/design.md` is amended wherever a decision changes what it says.

## Notes

- **Story scope: the whole of v1 as written in `docs/design.md`** (all nine commands, remote schemas, imports, refs, query language, validation rules, locking). Chosen by the human at charting time. The design's own "Out of scope for v1" list carries over as this story's out of scope.
- **`docs/design.md` is the source of truth** (project rule 6): to deviate from it, amend the doc first. Its original at `~/tmp/typdoc-design/` is a snapshot; this repo's copy is canonical from now on.
- Stack is fixed by `.chief/project.md`: Rust 2024, workspace with `typdoc-core` (lib) and `typdoc` (bin). Decisions here refine it; they do not reopen it.
- The design moves wayfinder-ticket fields (`Type:`, `Status:`, `Blocked by:`) into frontmatter, but the installed `chief-wayfinder` (`v5.canary-2.exp`) still uses inline lines. Tickets in this story use the inline form; typdoc does not manage its own tickets yet.
- Motivating failure: querying prose-formatted records with grep answers wrongly and silently (a status in a heading counted 43 "unanswered" when 21 were). Prefer decisions that make the wrong answer loud.
- Cargo gotchas for later stories: never run two cargo commands in one `target/` at once, and check test counts, not just exit codes.

## Decisions so far

<!-- index: one line per resolved ticket, enough to judge relevance, zoom the link for detail -->

- [YAML frontmatter approach](../_tickets/1-yaml-frontmatter-approach.md): read with `yaml_serde` into typed `String` fields; write with `yaml-edit` (exact-pinned, three operations) behind a mandatory reparse guard; `yaml-edit` is two days old, so the guard is load-bearing (research recommendation, human may override).
- [Markdown body parsing](../_tickets/3-markdown-body-parsing.md): build on `pulldown-cmark` 0.13.4; cut frontmatter first and parse the body slice; hand-write the line/col mapping and a GitHub-style slugger (research recommendation, human may override).
- [HTTP client for remote schemas](../_tickets/6-remote-schema-fetching.md): `ureq` 3.x blocking behind a `Fetch` trait; `typdoc-core` stays runtime-free (research recommendation, human may override).
- [Counter allocation](../_tickets/2-counter-allocation.md): next number = max(highest existing in the collection, the collection's `last`) + 1, written back under the lock; never reused after a delete; the shared `counter` option is removed (one ticket schema, one collection, told apart by `kind`); a coded schema serves exactly one collection; no new file. Where `last` lives is ticket 12.

## Not yet specified

- Exact `--json` output shape for every command (belongs in the contract `/chief-plan` writes, once the decisions above settle).
- Distribution and versioning of the tool itself (`cargo install`, release binaries, config `version` upgrade path).
- Scale: the index is rebuilt on every run with no cache; at what document count does that stop being acceptable?

## Out of scope

- Everything on the design's own "Out of scope for v1" list: editing body sections, SQL queries, saved query aliases, multi-hop ref traversal, imports of imports.
- Changing the `chief-wayfinder` or typmem skills to use typdoc: that work belongs to their repos, after typdoc exists.
- Migrating any existing registry (e.g. K-Care's `.jsonl` registers) onto typdoc.
