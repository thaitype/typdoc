# Ticket 8: writing the state file, and the rules that read it

Resolved. Commit `a99165b` (built in an isolated worktree, merged with `265a861`, no conflict).
`cargo fmt --check` clean, `cargo clippy --workspace --all-targets -- -D warnings` clean, public-text
check clean, all re-run on the merged tree. `scripts/test.sh` with `TMPDIR` pointed at a disk-backed
folder (see below): 862 passed / 0 failed / 1 ignored on the merged tree (822 before this ticket,
+10 from ticket 13 merged first, +30 from this ticket).

## Outcome

done

## What it does

`state::write` writes a collection's `last` under the namespace lock, through `write_atomically`.
Updating an existing entry finds the number's byte span with a small structural scanner and replaces
only that, so whatever else the file held keeps its own formatting; a new file or a new entry is
written canonically — JSON, keys sorted at every level, two-space indent, `\n`, a final newline.

`state.malformed` (error), `state.behind` (warn) and `state.retired` (warn) are built and registered
in `rules.rs`, out of `UNIMPLEMENTED_RULES`. `config.state-uncoded` narrows to a collection that
exists whose schema has no code; an entry for a collection the project no longer has at all is
`state.retired` instead, a `validate` finding rather than a config error, so it does not stop a read
the way its predecessor did.

## Checked by running

- Each of the three rules has a fixture that turns it red and a silent counterpart. `state.malformed`
  is one folder holding five namespaces, one malformed shape each (text, absent, negative, a
  fraction, too large), per the ticket's own "per shape" instruction.
- `state.retired` does not stop a `get`, shown against `config.state-uncoded` stopping one, in the
  same test.
- The measurement behind why none of the three repairs is a running test, not a comment: it builds
  decision 13's own example — a document deleted, its number kept versus its number derived — and
  shows keeping the gap gives a missing-target finding where deriving gives nothing at all. An early
  draft got the simulated reissue's own state wrong and this ticket's own `state.behind` rule caught
  it immediately.
- Five things were shown able to fail, one at a time, each restored: the malformed finding
  suppressed, the behind comparison off by one (caught by the exactly-equal case staying silent), the
  retired/uncoded narrowing disabled, `has()` ignoring a malformed entry, and the in-place patch
  forced to always reformat the whole file.
- The existing write ban reaches the new code: `std::fs::write` planted in library code and,
  separately, in test code, both refused; the same call inside `typdoc-fs` was not.

## Decision

- **Issue:** the write half — `state::write` itself — has no command to call it yet; `new` and `mv
  --renumber` are tickets 9 and 12.
- **Chosen:** proved by direct library tests only: pure logic with no filesystem in `typdoc-core`
  (the crate's own lint bans a write even in test code), and the write glue against a real
  filesystem in `crates/typdoc-fs`, mirroring ticket 1's own pattern. The first ticket that holds a
  lock and issues a number is what exercises it through a command.

## Notes

- **A whole, unrelated gate was red on this machine, and it now has a standing fix.** `/tmp` is a
  9.5G tmpfs shared by every crew, sitting at 80%; `shell_examples.rs` hit `QuotaExceeded` copying a
  stand-in binary there, confirmed unrelated to this diff. Pointing `TMPDIR` at a disk-backed folder
  (`/home` has 53G free) makes every test pass; this is recorded as the standing way to run the gates
  on this machine from here on, not a one-off workaround.
- `serde_json::Map`'s default order is not to be trusted inside `typdoc-core`: the binary crate's own
  `preserve_order` feature reaches `typdoc-core` through workspace feature unification, the same
  effect `.chief/project.md` already documents for `chrono`'s `clock` feature. `canonical` sorts
  explicitly rather than relying on the map's own order. No crate or feature was added for this.
- A stale comment in `crates/typdoc-core/clippy.toml`, naming a module from before an earlier
  ticket's refactor and claiming an exemption that no longer exists, was corrected in this commit.
