# Contract

This contract does not restate the design. `docs/design.md` specifies the behaviour, and where the two differ the design wins (project rule 6). The contract says which parts of the design story 1 implements, which earlier decisions bind it, what it decides that the design does not, and what it leaves open. The decisions are in `docs/design-decision-phase-1/`, and a ticket is named by its number there. How the story is tested is in `testing-decisions.md`.

## What story 1 implements

| Part | Where the design specifies it | For this story |
| --- | --- | --- |
| Finding a project and a scope | Discovery, Arguments that name a document, Choosing a namespace | `TYPDOC_DIR`, `TYPDOC_NAMESPACE`, `--namespace`; a key and a path told apart by form (16) |
| Config | `.typdoc/config.json`, Namespaces, Collections and Match templates, State, Imports, Machine-specific imports, Config errors | Read only. `state/<namespace>.json` is read and never written. `imports.json` is found through `TYPDOC_CONFIG_DIR`, `XDG_CONFIG_HOME`, then the platform default (7) |
| Schemas | Schema format, Target names | typdoc's own JSON format (8). Pinned copies of remote schemas are read; none is fetched (20) |
| Documents | Document files | Frontmatter read with `yaml_serde` (1); body parsed with `pulldown-cmark`, with our own line and column mapping and our own slugger (3, 4, 11) |
| Refs | Refs, Body links, Heading anchors, Across namespaces | The forms that are checked (10) and the slugs (4) |
| Query language | Query | The grammar, the escaping and the sorting (5) |
| Commands | `get`, `list`, `toc`, `refs`, `validate` | Five of the nine |
| Validation | Validation rules, Audit mode | Every rule except `frontmatter.transitions`, which is checked on write |
| Output | JSON output, Exit codes and errors | Shapes and order (15), exit codes (14) |

Not built here: `new`, `set`, `mv`, `pull`, locks, and the rule `frontmatter.transitions`.

## Decisions that bind the story

| Ticket | Binds story 1 on |
| --- | --- |
| 1 | Frontmatter is read with `yaml_serde` into typed values whose types come from the schema; the writer is not part of this story |
| 2 | The state file's `last` per collection is read for `state.missing`; the allocation itself is story 2 |
| 3, 4, 11 | The Markdown parser, the mapping of offsets to line and column, the slugger, and the unit of `col` |
| 5 | The query grammar and its errors |
| 7 | `imports.json`, `${ENV}` in import paths, Linux as the platform that is run; the lock is story 2 |
| 8, 12, 13 | The schema format, collection files, namespaces |
| 10 | Which link forms are checked |
| 14 | The exit codes, and the criterion for adding one |
| 15 | The shape and order of every `--json` output of the five commands and of the error object |
| 16 | How a path and a key are given as arguments, and how the project is found |
| 17, 18, 19 | Schema drift under `schema.valid`, the rule `frontmatter.parse`, `--schemas` and `--audit` with arguments |
| 20 | Story 1 is not released on its own; a remote schema with no pin is `config.schema-unpinned` |
| 21, 22 | No promised figure for the cost of a run; paths compared with their case |
| 9 | The test strategy, and which part each story builds |

## What this contract decides that the design does not

1. **The commands the binary has.** The registry holds `get`, `list`, `toc`, `refs` and `validate`. `new`, `set`, `mv` and `pull` are in `unimplemented_commands`, and exit codes 3 and 4 in `unproduced_exit_codes` (ticket 9; the lists are described in `testing-decisions.md`).
2. **The rules the binary has.** The registry holds every rule in the design's tables except `frontmatter.transitions`, which is in `unimplemented_rules`. Default: the same two checks as for commands, extended to rules, because the coverage checks of ticket 9 compare the design with the registry for commands only, so a rule that the design lists and nobody builds would not be noticed. How the list works with the fixtures is in `testing-decisions.md`.
3. **Reading pinned copies.** A pinned copy is read from `vendor/schemas/<sha256>` and its bytes are hashed with SHA-256 to check the name (`config.vendor-edited`); an absent copy is `config.vendor-missing`; a URL with no pin is `config.schema-unpinned`. The crate for SHA-256 is chosen by the ticket that builds the check and recorded in `.chief/project.md`.
4. **Nothing is written.** `typdoc-core` calls no function that changes the file system, and story 1 has no exception to that. A `clippy.toml` in its directory lists these as disallowed methods: `std::fs::write`, `rename`, `copy`, `remove_file`, `remove_dir`, `remove_dir_all`, `create_dir`, `create_dir_all`, `hard_link` and `set_permissions`; `std::fs::File::create`, `File::create_new` and `File::set_len`; `std::fs::OpenOptions::open`; `std::os::unix::fs::symlink`; and `std::process::Command::new`, so that a command cannot be spawned to do the writing. Reading is left alone: `File::open`, `read_to_string`, `read_dir`, `metadata` and `canonicalize` were run and pass. Each listed function was run on 2026-09-20 with clippy 1.96.0 and turns the check red, also when it is reached through `use std::fs`. `OpenOptions::open` is on the list because a write made by opening a file for appending and writing to it is not a call to any other listed function: a run without it passed. This makes "the read core changes nothing" a property of the code and not of care, for what the list names. It does not cover unsafe code or a direct system call, a write made inside a dependency, or a function of the standard library that is added later and is not on the list; `File::set_permissions` and `File::set_modified` are on no list and were not run. A test that needs a file uses a fixture in the repository.
5. **No network.** `ureq` is a dependency of neither crate. The story makes no request.
6. **`run(args, deps)`.** `deps` holds `Env` (the location of `imports.json`, and the `TYPDOC_` variables). `std::env::var`, `std::env::var_os` and `std::env::home_dir` are disallowed methods in `typdoc-core`. `Clock` and `Fetch` are added by the stories that need them.
7. **A `frontmatter.parse` finding never carries a position the reader did not give, and never one known to be imprecise.** Checked on 2026-09-20 with `yaml_serde` 0.10.7: it reports a line and a column for errors of syntax, the column in scalar values and the line counted inside the block, so the line of the opening `---` is added; it reports none for a block that holds a second YAML document; and it reports the start of the mapping, not the second key, for a duplicate key. Which errors keep their position is decided by the ticket that builds the rule, after it checks whether the reader tells the kind of error. Default: if it does not, the finding has no position for any error.
8. **The accounting invariant.** In `--audit --json`, for every project in the fixtures and for the two acceptance runs named in the goal, `summary.checked.documents` plus every count of what was not checked equals the number of `.md` files the run reads, counted independently of typdoc. Those counts are `summary.unreported.uncollected`, `summary.unreported.no_frontmatter` and `summary.overlapping`. A document whose block cannot be parsed is checked, since it has a finding, and is counted in none of them. A file matched by more than one collection is counted in `summary.overlapping` and not in `unreported`, which holds what is outside `findings`, while such a file is reported under `collections.overlap`.

## Not decided by this contract

- **Which files a run reads.** The design does not say whether a run enters folders whose names start with `.`, follows symbolic links, or leaves out ignored files. The count in the invariant above depends on it, so it is fixed in the design before the ticket that builds the walk. It changes what the audit lists as in no collection, so it is a decision about what users see.
- **How exit code 6 is produced by a read command in a test.** A failure caused by permissions does not work when the tests run as root. If no deterministic way is found, code 6 goes in `unproduced_exit_codes` (ticket 9).
- **Crates that `.chief/project.md` does not list.** The ticket that needs one chooses it and adds it there.
- **The `--json` shapes of `new`, `set` and `mv`** (story 2) **and of `pull`** (story 3), and the other items of those stories on the map.

## Constraints

- A Cargo workspace at the repository root, Rust 2024 on the stable toolchain. `crates/typdoc-core` is the library (config, schemas, index, query, validation) and knows nothing about the CLI; `crates/typdoc` is the binary (clap, output formatting, `--json`, exit codes).
- Linux is the platform that is run and claimed. The code is written for macOS as well, and a build on Windows fails with a message (7).
- Before every commit, `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` and `scripts/check-public-text.sh` pass. Tests run offline and never skip silently when a file is missing (project rules 1 and 2).
- Every command has `--json` and uses the exit codes that the design's table defines (project rule 5).
