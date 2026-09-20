# Ticket 1 Report

## Ticket
The walking skeleton: a Cargo workspace and `typdoc get <path> --json` over a project with one collection, one local schema and the smallest config.

## Outcome
done

## Decision
Nothing was ambiguous enough to stop the build. The ticket left these open, and each is settled below as a placeholder that a later ticket replaces or confirms.

- **Symbolic links and non-UTF-8 names.** Which files a run reads is not decided. A directory entry that a collection's pattern matches and that is a symbolic link stops the run with exit 6, and so does a matching name that is not valid UTF-8. Entries that no pattern matches are left alone. Exit 6 is arguably the wrong code, since a retry never fixes it. Both the code and the policy are settled by the ticket that builds the walk.
- **`*` in a `match`.** It stands for any run of characters, including a leading `.`, so `.md` and `.hidden.md` match `*.md`. Default. The design does not say, and nothing in this ticket pins it.
- **No project found** exits 5. The design does not say. An empty `TYPDOC_DIR` counts as not set.
- **A `match` that holds `/`, `**`, `{` or `}`** exits 2 as not read yet, until ticket 4.
- **A local `schema` path** is read relative to the project folder.
- **A file with no frontmatter block** is a document with no fields; an unclosed block exits 2.
- **Frontmatter values** other than text and lists of text exit 2. Reading them as text would change the file's text; coercion by schema is ticket 5.
- **`details` is `[]`** and `code` and `key` are left out of the error object until config errors carry ids (ticket 4).
- **`get` without `--json`** exits 1 as not built yet: the design gives no text form.

## Notes
- Run: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (39 tests: 3 in the binary's `run`, 33 in `tests/get.rs` through the built binary, 3 in the core's glob matcher) and `scripts/check-public-text.sh` with the names list, all green.
- Guards shown red and removed: in library code and in test code of `typdoc-core` (`std::fs::write`, `std::env::var`, `std::env::var_os`, `std::process::Command::new`), and a `Command::new` next to the spawn helper and in `tests/get.rs`.
- Not built: the compile-time failure on Windows (contract item 7, not part of this ticket).
- Not covered by a test: a non-UTF-8 `TYPDOC_DIR`; the empty environment of the spawn helper is set with `env_clear()` and no test reads the child's environment back.
- For ticket 4: config errors have no `config.*` id yet, and every one exits 2. For ticket 5: the value reader is replaced by coercion by schema. For the ticket that builds the walk: the two symbolic-link tests and the leading-dot default above.
