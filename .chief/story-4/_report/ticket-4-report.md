# Ticket 4 Report

## Ticket
`mv a.md a.md` (identical source/destination path) gets its own message instead of reusing the
case-only-rename wording.

## Outcome
done

## Decision
- **Issue:** the ticket/contract said to find "the case-only-rename test" in `project.rs`'s mv
  test module and add a sibling case for identical-path. In reality `project.rs`'s own test
  module doesn't cover `mv` at all — the real mv tests live in `crates/typdoc/tests/mv.rs` — and
  the one existing test matching that description was already the identical-path test; there
  never was a separate case-only-rename test (not producible on this repo's case-sensitive Linux
  CI through the normal harness).
- **Options considered:** none needed — this was a documentation-vs-reality mismatch, not a
  design ambiguity. Verified the reading against the actual test file; the Spec-axis code review
  confirmed it independently.
- **Chosen:** extended the existing identical-path test in place (renamed to
  `the_same_path_given_twice_is_refused_at_exit_7_with_its_own_message`) rather than fabricate a
  redundant case-only-rename test that can't be exercised here.

## Notes
`project.rs`'s decision-12 identity check now branches on `from_path == to_path` (string
equality) before the `same_file` (device+inode) check, giving the identical-path case its own
`Error::AlreadyExists` message. Exit code 7 unchanged either way. Full workspace tests green
except the pre-existing, unrelated `shell_examples.rs` environmental failure (confirmed present
on unmodified `HEAD` too, via `git stash`). Commit `36d75bb` on `story-4-namespace-ignore`.
