# Ticket 1: the write seam, the clock, and the write ban

Resolved. Built at `39d3dab`, then moved into its own crate at the commit this report accompanies.
Four gates green on the committed tree: `cargo fmt --check` clean, `cargo clippy --workspace
--all-targets -- -D warnings` clean, `scripts/test.sh` 767 passed / 0 failed / 1 ignored,
public-text check clean.

## What it delivers

`Deps` carries the file operations a write is built from, and a `Clock` that gives an instant and
the offset to write it in, at one second. One function above the seam holds what decision 4
settled — the reserved temp-file shape, the mode carried across, the rename — so no command sees a
temp file. Reads stay outside the seam except the three a write makes to decide what to do:
whether the destination exists, an existing file's mode, and whether two paths are one file.

The scenario table runs twice, against a fake and against a real temporary directory. The fake
stages what a real directory will not: no space, permission refused, a rename across devices, and
a stop between any two operations. Two tests name what neither covers — a power cut, and a kill
between two system calls.

## Where writing is allowed, and what enforces it

`typdoc-fs` is a crate of its own and is the only crate that may change a file. `typdoc-core`'s
lint list keeps every entry, including the two that forbid reading the time outside `Clock`, and
it carries **no exception anywhere**. The `Fs` trait and the policy function above it stay in
`typdoc-core`; the new crate depends on it to implement the trait, so there is no cycle.

What may write is therefore decided by the dependency graph rather than by an attribute. A crate
that can write has to declare it in a manifest, where it can be searched for and seen; an
attribute can be added beside an existing one without anything noticing. In a repository where
most code is written rather than read, a fence that needs someone to notice it is the weakest
fence available.

Shown by running, each plant restored afterwards:

| Planted | Result |
| --- | --- |
| `std::fs::remove_file` in `typdoc-core`'s `fs.rs`, directly above the seam | `error: use of a disallowed method` |
| `std::fs::write` in `typdoc-core`'s `slug.rs`, a module with nothing to do with files | `error: use of a disallowed method` |
| `std::fs::remove_file` inside `typdoc-fs` | no diagnostic, and no attribute is present to produce one |

The clock ban was shown red the same way, for `SystemTime::now` and for both of chrono's entries.
Those two bite only under a workspace build: `typdoc-core` does not enable chrono's `clock`
feature itself, so a single-crate run reports the entries as naming no function and the ban is
not in force. That is why the clippy gate is run over the workspace, and `.chief/project.md` now
says so where the gate is defined.

## Choices the design does not state

- The shipped clock reports the machine's offset rather than always UTC, because the design's
  `datetime` examples carry real offsets and a fixed `+00:00` would make the offset carry no
  information.
- `sync` is on the trait and is not called: the contract records durability as not decided, and
  having the operation there means that decision can go either way without the trait changing.
- The carried mode is set before the bytes rather than after, so that a document's contents never
  sit under a looser mode for the length of a write and a leftover from a killed run does not
  carry one.
- `same_file` answers `false` when either path is absent rather than failing, since a path that
  leads nowhere names no file. `exists` returns a result rather than a bool, so a permission that
  cannot be read is not read as an absence.

## Left for later

`typdoc`'s own crate has no write ban; it bans only starting a process. Nothing in it writes
today — checked, and every write call in that crate is in test code building scratch projects —
but commands begin writing at ticket 9, and `clippy.toml` cannot tell a test target from the
rest, so a ban there would need an exemption for each of them. Worth revisiting when the write
commands land.
