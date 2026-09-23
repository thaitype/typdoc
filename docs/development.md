# Development

This page is for working on typdoc rather than with it. [docs/design/design.md](design/design.md) is the source of
truth for what the tool does; where this page and the design disagree, the design wins.

## The workspace

Four crates, and the split between the first two is load-bearing rather than tidy:

| Crate | What it is |
| --- | --- |
| `typdoc-core` | Reading, parsing, rules, refs, queries, and every decision. It reaches the environment, the clock and the file system only through a seam. |
| `typdoc-fs` | The concrete file system, and the only crate that writes a file. It depends on `typdoc-core`, never the other way round. |
| `typdoc` | The command line: argument parsing, output, exit codes, and the binary that is installed. |
| `typdoc-testkit` | Test support shared between the crates: fixtures, staging, and the harnesses that read the design. |

What may write a file is settled by which crate a piece of code lives in, not by an attribute
somebody can add. Moving a write into `typdoc-core` undoes that, and the lints below are what says
so out loud.

Edition 2024, resolver 3.

## Build

```console
$ cargo build
```

The binary lands in `target/debug/typdoc`. To install it for use instead, [README.md](../README.md) has the
command.

## Tests: run `scripts/test.sh`, not `cargo test`

```console
$ scripts/test.sh                 # the whole suite
$ scripts/test.sh ARGS...         # the same, with these arguments instead
$ scripts/test.sh --self-test     # prove the ceiling stops a runaway
```

The script runs the suite under a memory ceiling of 6144 MB with swap switched off, because a
loop that could not end once took every byte of the machine it ran on and the kernel killed the
session along with the run. The numbers behind the ceiling are measured and are written down in
the script: the suite itself peaks at 229 MB and a cold build followed by a full run peaks at
1451 MB, both at the cgroup rather than per process, and the run that made the kernel step in
reached 16 GB. They belong to a four-core machine; more cores build more crates at once and need
measuring again.

Two things follow that are worth knowing before the first run.

**The script refuses rather than running uncapped.** Without `systemd-run`, or where a user scope
cannot be started, it exits 2 and says so. A safety net that disappears quietly is worse than
none, because everyone goes on believing it is there. On a machine without it, put the run under
another ceiling rather than dropping the ceiling.

**`cargo test` by hand needs a feature.** The shell examples harness puts a stand-in for `typdoc`
on `PATH`, and that stand-in is a binary of the `typdoc` package behind the `test-stand-in`
feature, which is off by default so that `cargo install` does not offer it to a caller. The
workspace run in the script turns it on. A run started by hand does not, and the four tests that
reach for the stand-in fail on a path that is not there:

```console
$ cargo test --workspace --features typdoc/test-stand-in
```

Naming a feature of one package refuses a run that selects another package alone, which is why
the script does not add the flag to its passthrough form.

## Lints

Each crate carries its own `clippy.toml`, and the lists in them are the architecture written as
a check rather than as a comment.

`typdoc-core` may not call the environment, the home directory, the clock, or any function that
writes, creates, renames, removes or changes the permissions of a file. Those are reached through
`deps` or through the `Fs` trait, and the concrete calls live in `typdoc-fs`, which carries no
such list because it is the place the writing is meant to happen.

`typdoc` and `typdoc-testkit` may not start a process except through the CLI tests' spawn helper
and the shell examples harness, each of which the list names.

**The lists cover test code too, and an `allow` is not the fix.** A test that reaches around the
seam is exactly the test that stops proving anything about the code that ships, and a lint turned
off in one place stops being a property of the crate everywhere. A call that seems to need an
exception is a sign the seam is in the wrong place, which is a design question and belongs in a
decision record.

## Where the truth is

- [docs/design/design.md](design/design.md) — what typdoc does and why, in full, and the arbiter when documents
  disagree.
- [docs/design/design-decision-phase-1/](design/design-decision-phase-1/) and `.../design-decision-phase-2/` — the decisions behind
  the design, each with the research or the measurement it rests on, and a map naming what is
  still open.
- [README.md](../README.md), [docs/getting-started.md](getting-started.md), [docs/commands.md](commands.md), [docs/projects.md](projects.md) — written for
  whoever uses the tool. An output printed in any of them is a real one, produced by running it.

## What is not set up

Neither of these is an oversight to work around quietly; both are open.

- **There is no CI.** "Held in place by a test" means the test exists, not that anything runs it
  before a change lands. Running `scripts/test.sh` is on whoever makes the change.
- **The toolchain is not pinned.** There is no `rust-toolchain.toml`; the documents say only that
  a recent toolchain is needed. What is built and run today is 1.96.0.
