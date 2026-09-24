# Development

How to build typdoc, run its tests, and get a change through CI.

## Setup

You need Rust. The repository pins the toolchain in `rust-toolchain.toml` (1.96.0, with rustfmt
and clippy), and `rustup` picks it up automatically.

```console
$ git clone https://github.com/thaitype/typdoc
$ cd typdoc
$ cargo build
$ target/debug/typdoc --version
```

## The workspace

| Crate | Contains |
| --- | --- |
| `crates/typdoc-core` | Parsing, schemas, rules, refs, queries: every decision typdoc makes |
| `crates/typdoc-fs` | The real file system. The only crate that writes files |
| `crates/typdoc` | The command-line interface and the installed binary |
| `crates/typdoc-testkit` | Shared test support: fixtures, staging, golden files |

`typdoc-core` never touches the file system, the environment or the clock directly. It goes
through a trait, and `typdoc-fs` provides the real implementation. That split is enforced, not
just a convention: each crate's `clippy.toml` bans the calls it isn't allowed to make, test code
included. If a change seems to need an exception, the code is probably in the wrong crate.

## Running the tests

Use the script rather than `cargo test`:

```console
$ scripts/test.sh                 # the whole suite
$ scripts/test.sh -p typdoc-core  # any cargo test arguments
$ scripts/test.sh --self-test     # check that the memory ceiling works
```

The script runs the tests under a memory ceiling, so a test stuck in a loop can't take down the
machine. On Linux it uses `systemd-run`; on macOS it uses its own watchdog. If it can't apply the
ceiling it stops with an error rather than running without one.

Plain `cargo test` also works, but a few tests need a stand-in binary that's behind a feature
flag, so pass it:

```console
$ cargo test --workspace --features typdoc/test-stand-in
```

If `/tmp` is small or full on your machine, point `TMPDIR` somewhere else before running the tests.

## Before you commit

The same three checks CI runs:

```console
$ cargo fmt --check
$ cargo clippy --workspace --all-targets -- -D warnings
$ scripts/test.sh
```

## CI

GitHub Actions runs those three checks on every push to `main` and every pull request, on both
`ubuntu-latest` and `macos-latest`. The workflow is `.github/workflows/ci.yml`.

The Linux and macOS test counts differ by a small fixed number: a handful of tests create files
whose names aren't valid UTF-8, which macOS file systems don't allow, so those are skipped there.

## Where things are documented

| For | Where |
| --- | --- |
| People using typdoc | `README.md` and `docs/` |
| Coding agents using typdoc | `skills/typdoc/`, the agent skill |
| What typdoc should do | `docs/design/spec/` (prose) and `docs/design/catalog/` (the data tests read) |
| Past decisions | `docs/archived-design/`, frozen |

When a change alters what a command prints or accepts, update the user docs and the agent skill
in the same pull request. The skill states the typdoc version it describes on its first line.

The tests never read Markdown to learn what typdoc should do. Rule ids, commands, exit codes and
similar lists live as JSON in `docs/design/catalog/`, and the tests compare the code against
those.

## Releasing

1. Bump `version` in every crate's `Cargo.toml`.
2. Add the release to `CHANGELOG.md`.
3. Update the version on the first line of `skills/typdoc/SKILL.md`, and the install tag in the
   README and docs.
4. Merge to `main`, then tag `vX.Y.Z`.
