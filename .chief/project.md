# Project Configuration

## Project

**typdoc** — a CLI that treats a folder of Markdown files as typed, linked documents: it validates frontmatter against JSON schemas, and resolves, queries and checks refs between documents. Every command has `--json` output and meaningful exit codes, because the main users are agents.

Design source of truth: `docs/design/` (`spec/` for prose, `catalog/` for the JSON-body reference documents typdoc itself validates). Content not yet migrated out still lives at `docs/migrating-design/design.md`, progressively emptied as each piece moves; `docs/archived-design/design.md` (original: `typdoc — Generic Markdown CLI Design.md`, 2026-09-19) is the frozen original, archived 2026-09-24 per story 3, ticket 13, and is never edited again.

## Development Commands

```bash
cargo build --workspace
scripts/test.sh                            # the test suite, under a memory ceiling; must pass offline
scripts/test.sh <args>                     # the same, with arguments passed to cargo test
scripts/test.sh --self-test                # proves the ceiling stops a runaway; run whenever the script changes
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo run -p typdoc -- <args>              # run the CLI from the repo
TYPDOC_REGENERATE_GOLDEN=<command>/<case> scripts/test.sh -p typdoc --test golden regenerate -- --ignored   # regenerate one golden, never all
```

## Architecture Overview

### Tech Stack

- Rust, edition 2024, stable toolchain
- Cargo workspace at the repo root, crates under `crates/`
- clap (derive), serde + serde_json, thiserror (lib) + anyhow (bin), chrono; the binary crate enables chrono's `clock` feature, which is what reads the machine's time and the offset it is in for the clock in `deps`. `typdoc-core` does not ask for that feature, but a workspace build unifies features across the members, so `Utc::now` and `Local::now` are reachable from `typdoc-core` all the same; what keeps a time there coming only through `deps` is the disallowed list in its `clippy.toml`, not the feature. The feature brings `iana-time-zone`, the only one of its tail this platform builds; the rest are for Windows, wasm and Haiku and enter the lock file without being compiled. Decided: the shipped clock reports the machine's offset rather than UTC, so that a stamped time says where it was written, which is what carrying an offset is for
- `serde_json`'s `raw_value` feature, in the binary crate: `--json` prints a `number` with the digits written in the document, and a `serde_json::Number` holds a `u64`, an `i64` or an `f64` and nothing else, so `1e3` cannot survive one. A document object is therefore assembled as JSON text and handed on as a `RawValue`. What follows from it is that reading `--json` back into a `serde_json::Value` converts the number again, so a golden file, whose comparison is of parsed JSON, cannot tell `1e3` from `1000.0`; the digits are pinned against the bytes on standard output instead, and `crates/typdoc-testkit/src/golden.rs` says so where a reader of the harness meets it. Decided: `arbitrary_precision` is not used, although it would let a golden see the digits, because it also changes how a number is classified and compared, and how a number is compared is settled elsewhere
- tempfile (dev-dependency of the binary crate and of `typdoc-core`; an ordinary dependency of `typdoc-testkit`, whose fixture loader copies a write fixture to one ahead of running it, so the crate needs it outside `#[cfg(test)]` too): a fresh `HOME` for each spawned process, scratch projects for the cases no fixture holds, and the real temporary directory the write seam's scenarios run against; it is the smallest crate that makes a temporary folder and removes it when the test ends
- YAML frontmatter: read with `yaml_serde` into typed `String` fields (types come from the schema); written back with the same crate, from the text the read path already kept, behind a typdoc-owned trait so the writer behind it can be revisited without touching a call site. A write rewrites the whole block rather than editing the line that changed, quoting a value only where YAML would otherwise read it as something else; `yaml-edit` is not a dependency, and there is no re-read guard, because rewriting from the reader's own text leaves nothing for a guard to catch that would not already be a defect in the reader (decision 20)
- Markdown body: `pulldown-cmark` 0.13.4 with `default-features = false`; the frontmatter is cut first and the body slice parsed, with its offset added back so lines count from the top of the file
- `unicode-general-category`: the slugger deletes characters by Unicode general category (punctuation, symbols, controls and so on), which the standard library does not expose; it is a table with no dependencies of its own, and `char::is_alphabetic` from the standard library covers the letters that are kept
- Remote schemas: `ureq` 3.x (blocking, rustls) behind a `Fetch` trait and a typdoc-owned `FetchError`, so `typdoc-core` has no async runtime: no restriction on http or https (the connection is the user's choice), an explicit timeout, a maximum response size, at most ten redirects, and the proxy variables `HTTPS_PROXY` and `HTTP_PROXY` honoured
- Pinned schemas: `sha2` 0.11, `default-features = false`, to hash a vendored copy's bytes against its file name (`config.vendor-edited`); it is the standard, dependency-light SHA-256 implementation in the Rust ecosystem, and the default features add only `std`, `alloc` conveniences this crate has no use for. The same dependency hashes the project's relative path for a `git-common` lock (decision 14): one hash function, two callers, no second crate
- `trybuild` 1.0 (dev-dependency of `typdoc-core`): the compile-fail proof that a lock cannot be built outside its own module and that a writing function cannot be called without one (decision 6) — the smallest widely-used crate that runs both the failing and the compiling half of that proof and reports a fixture that fails to compile for the wrong reason as a failure of the harness itself, which a hand-rolled check of `rustc`'s exit code would not
- `signal-hook` 0.4.4 (ordinary dependency of the binary crate, at an ordinary version requirement rather than an exact pin): catches `SIGINT` and `SIGTERM`, moves the identity check and the unlink off the handler onto an ordinary thread, and re-raises through the default disposition so the process ends by the signal, all in one dependency (decisions 5 and 6)
- `libc` 0.2.189 (dev-dependency of the binary crate, already reached indirectly through `signal-hook`): `kill(2)` for the CLI spawn helper's own way to send a spawned child a real `SIGINT` or `SIGTERM`, which `std::process::Child::kill` cannot reach past `SIGKILL`

### Key Architectural Patterns

- **lib + bin split:** `typdoc-core` (config, schema, index, query, validate, lock) knows nothing about the CLI; `typdoc` (bin) owns clap, output formatting, `--json` and exit codes.
- **Lens, not format:** typdoc adds no syntax to Markdown — every document stays a plain Markdown file.
- **Index-first:** before any query or validation, build one index mapping every key and path to its file.
- **Write under lock:** one lock per namespace; files are written via temp file + rename.
- **One write seam:** `deps` carries the file operations a write is built from and a clock, and `typdoc-core`'s `clippy.toml` disallows every function that writes and every way of reading the time, so only the module that implements the seam can reach either. The policy above the seam — the temp file's reserved shape, the mode carried across, the rename — is in one function, so no command sees a temp file.

### Directory Structure

- `crates/typdoc-core/` — lib
- `crates/typdoc/` — bin (clap); the CLI itself is in its `src/lib.rs`, so that its tests reach the registry of commands
- `crates/typdoc-testkit/` — dev-only, not published: the loader of `fixtures/` (the design extraction mechanism that once also read `docs/archived-design/design.md` as data was removed in story 3, ticket 12), the comparison of golden files with the guard of the generator that writes them, and the fake file system and fixed clock the write seam is tested against; both crates use it in tests
- `fixtures/` — projects the tests read (`valid/`, `broken/`), and the golden cases of `--json` output in `output/<command>/<case>/`
- `docs/` — design doc
- `examples/` — sample namespaces (`.typdoc/config.json` + schemas)
- `.chief/` — planning artifacts

### Important Development Rules

1. Before every commit, `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings` and `scripts/test.sh` must pass. `--workspace` on the clippy line is load-bearing and is not there for speed: `typdoc-core` does not enable chrono's `clock` feature itself, so in a single-crate run `chrono::Utc::now` and `chrono::Local::now` are not reachable, their entries in `crates/typdoc-core/clippy.toml` are reported as naming no function, and the ban on reading the time outside `Clock` is silently not in force. A workspace run unifies the feature the binary asks for, the two functions become reachable from `typdoc-core`, and the entries bite — checked by planting `chrono::Utc::now()` in a module of `typdoc-core` and reading the error text, which is `use of a disallowed method` under `--workspace` and a plain `no associated function named now` under `-p typdoc-core`. A guard that holds only because of how a command happened to be written is a guard nobody is keeping. Run the tests through `scripts/test.sh` and not through `cargo test` directly: it puts the run under a memory ceiling of 6144 MB, which this machine needs because a test that could not end once took all of it and the kernel killed the session driving the run as well. The ceiling is measured rather than guessed: the suite peaks at 229 MB and a cold build followed by a full run peaks at 1451 MB, both at the cgroup, on this four-core machine, so the ceiling leaves about four times the worse case. Those numbers belong to this machine and this core count; a machine that builds more crates at once needs them measured again. A test that hangs or grows without stopping is a fault to report, never a reason to raise the ceiling.
2. Tests must run offline (remote schemas are mocked). Fixtures live in the repo: it is public, so a real document is copied in only after review, and a test never skips silently when a file is missing.
3. Writes touch only the frontmatter block and never re-serialize the body (the exception is `mv`, which rewrites link paths).
4. Always write files via temp file + rename (the exception is `new`'s own document creation, which uses `O_EXCL` with no temp file, since the file it creates adds no state a temp file would have to protect — decisions 7 and 15).
5. Every command must support `--json` and use the exit codes the design defines. The table in the design is the only place they are listed, so adding a code never changes this rule.
6. The design doc is the source of truth — to deviate from it, change the doc first.
