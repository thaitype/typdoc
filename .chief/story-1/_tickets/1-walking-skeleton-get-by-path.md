# 1: Walking skeleton: `typdoc get <path> --json`

Type: implementation
Status: resolved
Blocked by: None (can start immediately)

## What this delivers

- A Cargo workspace at the repository root with `crates/typdoc-core` (library) and `crates/typdoc` (binary), Rust 2024.
- `run(args, deps)` with `Env` in `deps`; the project found by walking up from the current directory, or from `TYPDOC_DIR`.
- The smallest config (`{ "version": 1 }`), one collection with a `match` glob, one local schema, an index by path, and the frontmatter of one document read.
- `typdoc get <path> --json` printing `{ "document": ... }` as JSON output describes; exit codes 0, 1 and 5; the error object on standard error.
- The spawn helper (empty environment, constant `PATH`, fresh `HOME`) and the lints of the contract: `--all-targets`, the bans on `Env`, on writing and on `Command::new` in `typdoc-core`.
- `fixtures/valid/minimal`.

## Done when

- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` and `scripts/check-public-text.sh` pass.
- A CLI test runs the built binary through the spawn helper on the fixture and checks the JSON and the exit codes for a found document, a missing one and a bad argument.
- Each ban is shown red once by planting a violation in test code and in library code, then removed.
- The crate writes no file and makes no request.
