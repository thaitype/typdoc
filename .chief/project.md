# Project Configuration

## Project

**typdoc** — a CLI that treats a folder of Markdown files as typed, linked documents: it validates frontmatter against JSON schemas, and resolves, queries and checks refs between documents. Every command has `--json` output and meaningful exit codes, because the main users are agents.

Design source of truth: `docs/design.md` (original: `typdoc — Generic Markdown CLI Design.md`, 2026-09-19).

## Development Commands

```bash
cargo build --workspace
cargo test --workspace                     # must pass offline
cargo clippy --workspace -- -D warnings
cargo fmt --check
cargo run -p typdoc -- <args>              # run the CLI from the repo
```

## Architecture Overview

### Tech Stack

- Rust, edition 2024, stable toolchain
- Cargo workspace at the repo root, crates under `crates/`
- clap (derive), serde + serde_json, a YAML frontmatter parser, thiserror (lib) + anyhow (bin), chrono

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

1. Before every commit, `cargo fmt --check`, `cargo clippy --workspace -- -D warnings` and `cargo test --workspace` must pass.
2. Tests must run offline (remote schemas are mocked).
3. Writes touch only the frontmatter block and never re-serialize the body (the exception is `mv`, which rewrites link paths).
4. Always write files via temp file + rename.
5. Every command must support `--json` and use the exit codes 0–4 from the design.
6. The design doc is the source of truth — to deviate from it, change the doc first.
