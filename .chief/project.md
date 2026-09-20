# Project Configuration

## Project

**typdoc** — a CLI that treats a folder of Markdown files as typed, linked documents: it validates frontmatter against JSON schemas, and resolves, queries and checks refs between documents. Every command has `--json` output and meaningful exit codes, because the main users are agents.

Design source of truth: `docs/design.md` (original: `typdoc — Generic Markdown CLI Design.md`, 2026-09-19).

## Development Commands

```bash
cargo build --workspace
cargo test --workspace                     # must pass offline
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo run -p typdoc -- <args>              # run the CLI from the repo
scripts/check-public-text.sh               # public-text gate: a floor, not a ceiling (see rule 7)
scripts/check-public-text.sh --self-test   # proves the gate can fail; run whenever the gate changes
```

## Architecture Overview

### Tech Stack

- Rust, edition 2024, stable toolchain
- Cargo workspace at the repo root, crates under `crates/`
- clap (derive), serde + serde_json, thiserror (lib) + anyhow (bin), chrono
- YAML frontmatter: read with `yaml_serde` into typed `String` fields (types come from the schema); write with `yaml-edit`, exact-pinned, behind a typdoc-owned trait of three operations (set a scalar, append or remove a list item, add a key), and every write is re-read with `yaml_serde` and compared with the intent before the temp file is renamed. A mismatch rejects the write with an error, with no automatic fallback (`yaml-edit` is young; the trait makes a hand-written editor a later swap)
- Markdown body: `pulldown-cmark` 0.13.4 with `default-features = false`; the frontmatter is cut first and the body slice parsed, with its offset added back so lines count from the top of the file
- Remote schemas: `ureq` 3.x (blocking, rustls) behind a `Fetch` trait and a typdoc-owned `FetchError`, so `typdoc-core` has no async runtime: no restriction on http or https (the connection is the user's choice), an explicit timeout, a maximum response size, at most ten redirects, and the proxy variables `HTTPS_PROXY` and `HTTP_PROXY` honoured

### Key Architectural Patterns

- **lib + bin split:** `typdoc-core` (config, schema, index, query, validate, lock) knows nothing about the CLI; `typdoc` (bin) owns clap, output formatting, `--json` and exit codes.
- **Lens, not format:** typdoc adds no syntax to Markdown — every document stays a plain Markdown file.
- **Index-first:** before any query or validation, build one index mapping every key and path to its file.
- **Write under lock:** one lock per namespace; files are written via temp file + rename.

### Directory Structure

- `crates/typdoc-core/` — lib
- `crates/typdoc/` — bin (clap)
- `docs/` — design doc
- `examples/` — sample namespaces (`.typdoc/config.json` + schemas)
- `.chief/` — planning artifacts

### Important Development Rules

1. Before every commit, `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings` and `cargo test --workspace` must pass, and so must `scripts/check-public-text.sh` (rule 7).
2. Tests must run offline (remote schemas are mocked). Fixtures live in the repo: it is public, so a real document is copied in only after review, and a test never skips silently when a file is missing.
3. Writes touch only the frontmatter block and never re-serialize the body (the exception is `mv`, which rewrites link paths).
4. Always write files via temp file + rename.
5. Every command must support `--json` and use the exit codes the design defines. The table in the design is the only place they are listed, so adding a code never changes this rule.
6. The design doc is the source of truth — to deviate from it, change the doc first.
7. This repository is public, and every line in it reads as the repository owner's own work: no names of people, teams, tools or assistants who worked on it, no record of who approved or decided something or how a decision arrived, and no wording that refers to someone else deciding. The reasoning is kept in full; only who decided and how it arrived is left out. A decision is written as `Decided ...`, a chosen fallback as `Default ...`, and something not checked as `Not verified ...`. This holds for every file, `.chief/` included, and for later edits, not only for a first cleanup. `scripts/check-public-text.sh` catches phrasing already known to fail. It is a floor, not a ceiling: a clean run proves nothing beyond its patterns, and the check that decides is reading each paragraph and asking whether it reads as the owner's own work.
