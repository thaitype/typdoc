## Destination

`docs/design.md` plus this map's Decisions so far is enough for `/chief-plan` to write typdoc v1's goal and contract without re-grilling: every implementation-level gap in the design is decided, and `docs/design.md` is amended wherever a decision changes what it says.

## Notes

- **Story scope: the whole of v1 as written in `docs/design.md`** (all nine commands, remote schemas, imports, refs, query language, validation rules, locking). Chosen at charting time. The design's own "Out of scope for v1" list carries over as this story's out of scope.
- **`docs/design.md` is the source of truth** (project rule 6): to deviate from it, amend the doc first. Its original at `~/tmp/typdoc-design/` is a snapshot; this repo's copy is canonical from now on.
- Stack is fixed by `.chief/project.md`: Rust 2024, workspace with `typdoc-core` (lib) and `typdoc` (bin). Decisions here refine it; they do not reopen it.
- The design moves wayfinder-ticket fields (`Type:`, `Status:`, `Blocked by:`) into frontmatter, but the installed `chief-wayfinder` (`v5.canary-2.exp`) still uses inline lines. Tickets in this story use the inline form; typdoc does not manage its own tickets yet.
- Motivating failure: querying prose-formatted records with grep answers wrongly and silently (a status in a heading counted 43 "unanswered" when 21 were). Prefer decisions that make the wrong answer loud.
- Cargo gotchas for later stories: never run two cargo commands in one `target/` at once, and check test counts, not just exit codes.

## Decisions so far

<!-- index: one line per resolved ticket, enough to judge relevance, zoom the link for detail -->

- [YAML frontmatter approach](../_tickets/1-yaml-frontmatter-approach.md): read with `yaml_serde` into typed `String` fields; write with `yaml-edit` (exact-pinned, three operations) behind a mandatory reparse guard; `yaml-edit` is two days old, so the guard is load-bearing (decided 2026-09-19).
- [Markdown body parsing](../_tickets/3-markdown-body-parsing.md): build on `pulldown-cmark` 0.13.4; cut frontmatter first and parse the body slice; hand-write the line/col mapping and a GitHub-style slugger (decided 2026-09-19).
- [HTTP client for remote schemas](../_tickets/6-remote-schema-fetching.md): `ureq` 3.x blocking behind a `Fetch` trait; `typdoc-core` stays runtime-free (decided 2026-09-19).
- [Query grammar](../_tickets/5-query-grammar.md): `!=` is exactly NOT `=` and absent things satisfy it (ordering comparisons stay false); escapes are `\` for `,` `*` `\` only, glob is `*` only; names, scope, `$body` and reserved pseudo-fields fixed; `ref.all` and `refby.all` need `.EXPR`; dangling refs found with `ref.any(f).path!=*`; a Grammar block is in the design.
- [Schema format identity](../_tickets/8-schema-format-identity.md): typdoc's own JSON format, not JSON Schema; no interop in v1; design wording fixed in three places.
- [Collection definition files](../_tickets/12-collection-definition-files.md): one file per collection at `.typdoc/collections/<name>.json` (`match`, `schema`, `refBase`, `validation`, `last`); one `version` in `config.json` covers all typdoc-owned formats; no collection order; overlapping matches are an error; a broken collection file stops the namespace.
- [Counter allocation](../_tickets/2-counter-allocation.md): next number = max(highest existing in the collection, the collection's `last`) + 1, written back under the lock; never reused after a delete; the shared `counter` option is removed (one ticket schema, one collection, told apart by `kind`); a coded schema serves exactly one collection; no new file. Where `last` lives is ticket 12.
- [Slug rules](../_tickets/4-slug-rules.md): heading slugs follow GitHub's algorithm (Thai kept, `dup-1` collision-aware, empty slugs deduped like any other); link fragments are percent-decoded and compared case-insensitively; `{slug}` is removed from file names, which are never derived from a title.
- [Multiple namespaces in one `.typdoc`](../_tickets/13-multiple-namespaces-in-one-typdoc.md): a project (`.typdoc`) holds one namespace `default` or several named by child folders (`namespaces`, one level, `[A-Za-z0-9_-]`); `name` removed from config; `name:` sibling, `name::` import; whole-project import in v1; scope by prefix > `--namespace` > `TYPDOC_NAMESPACE` > cwd, `TYPDOC_DIR` replaces `--dir`; state in `state/<namespace>.json`, locks per namespace; coded docs cannot move across namespaces except `mv --renumber`, with `auto: moves` and `refs.moved`. Amends 2, 12, 7, 5, 4.
- [Column unit](../_tickets/11-column-unit.md): `col` counts Unicode scalar values, 1-based (Thai and emoji each 1, tab 1), `line` from the top of the file frontmatter included; no byte offset in `--json` in v1; tests with a Thai line and an emoji line.
- [Link forms checked](../_tickets/10-link-forms-checked.md): `body.links` checks inline, image and reference-style links (definitions reported once, with use count; unused ones checked, not in `$body`; duplicate label reported); text that looks like a link but is not (space in a destination ending in a file extension) is reported with a `<…>`/`%20` hint; `[t][ref]` with no definition is not; `mv` keeps each link's written form.
- [Platforms and lock](../_tickets/7-platform-and-lock.md): Linux and macOS only (Windows build fails clearly); no automatic lock takeover, exit 4 explains, one remover at a time; project lock for pins, `lock.json` without `path`, `vendor/` never pruned in v1; machine file `imports.json` found by `TYPDOC_CONFIG_DIR`, `XDG_CONFIG_HOME`, then the platform default; unset `${ENV}` makes the import absent (`imports.absent`), never an empty string; quoting advice only for shells that were run.

## Not yet specified

- Exact `--json` output shape for every command (belongs in the contract `/chief-plan` writes, once the decisions above settle).
- Distribution and versioning of the tool itself (`cargo install`, release binaries, config `version` upgrade path). Also open there: whether the test suite must run from a published package. Fixtures and examples sit at the repository root, outside every crate's package root, so a package's `tests/` cannot find them (checked with `cargo package --list`); choosing between excluding `tests` from packages and moving fixtures inside a crate belongs to this decision.
- A published meta-schema (JSON Schema describing typdoc's own schema files) so editors can complete `schemas/*.json`; no one has asked for it yet.
- Exact argument shape of `mv --renumber` (how the destination namespace is named) and whether it is atomic across two namespaces when a write fails midway; belongs in the contract.
- From ticket 7, for the contract: the crate for catching interrupt signals; running zsh to earn its row in the quoting list; whether `imports.absent` should fire for an unresolved import that nothing refers to.
- Whether the public-text gate (`scripts/check-public-text.sh`) runs automatically, in CI or a hook: today it runs only when someone remembers, so it is a habit, not a gate. Adding CI to this public repository is not yet decided.
- For the contract: exit 1 covers three unrelated things (not found, bad arguments, I/O), which says little to a program that has to branch on it, and a test asserting exit 1 proves little.
- Scale: the index is rebuilt on every run with no cache; at what document count does that stop being acceptable?

## Out of scope

- Everything on the design's own "Out of scope for v1" list: editing body sections, SQL queries, saved query aliases, multi-hop ref traversal, imports of imports.
- Changing the `chief-wayfinder` or typmem skills to use typdoc: that work belongs to their repos, after typdoc exists.
- Migrating any existing registry (e.g. K-Care's `.jsonl` registers) onto typdoc.
