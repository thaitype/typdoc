# Ticket 25 Report

## Ticket
`resolve_path`'s fallback in `crates/typdoc-core/src/refs.rs` trusted `Path::is_file()` (a raw
`stat()`) to decide whether a ref's written path matched a real file, but that syscall's case
sensitivity depends on the filesystem — correct on Linux's ext4, wrong on macOS's default APFS,
where it resolves a wrongly-cased ref to the real file and reports it found.

## Outcome
done

## The fix
`resolve_path`'s fallback (around line 406 of `refs.rs`) no longer calls `root.join(path).is_file()`
directly. It now calls a new `case_exact_file(root, path)`, which:

1. Normalizes `path` with the crate's existing `normalize` (needed because one caller,
   `resolve_into_project`'s own trailing `resolve_path(rest, ...)` call, can reach this function
   with an un-joined, un-normalized `rest` straight from an import ref, unlike every other call
   site which already ran the path through `join`).
2. Splits the normalized path on `/` and walks it component by component, starting from `root`.
3. At each level, calls a helper `exact_entry(dir, name)` that does `std::fs::read_dir(dir)` and
   compares each entry's `file_name()` against `name` as raw `OsStr` — never a case-folded or
   Unicode-normalized comparison — returning the real entry's path only on an exact byte match.
4. Every component is checked this way, not just the last one — a case-insensitive filesystem
   folds case at every directory level of a path lookup, not only the final filename, so an
   intermediate folder written with the wrong case (`story/Target.md` when the real folder is
   `Story`) needed the same treatment as the leaf name.
5. The final component additionally calls `.is_file()` on the exact-matched entry (once its name
   is already confirmed correct) to keep rejecting directories, exactly as `is_file()` did before.

Symlinks: `read_dir` on an intermediate symlink-to-directory follows it transparently (normal
`opendir` behavior), same as the old `is_file()`'s path resolution did for intermediate
components. `.is_file()` on the final entry also follows a symlink to a real file, so a symlink
target still resolves — the fix only ever changes whether the *name* has to match exactly, not
how the entry got there.

Nonexistent paths: `std::fs::read_dir(dir).ok()?` turns any read failure (missing directory, not
a directory, permission error) into a clean `None`/`false`, never a panic or a propagated `Err`
— matching the previous behavior where `is_file()` on a nonexistent path simply returned `false`.
An unreadable individual directory entry (`Result::Err` from the `ReadDir` iterator) is skipped
via `.flatten()` rather than aborting the whole scan.

Performance: this fallback is only reached when `index.get(path)` already missed — i.e. it is
never on the hot path for the common case (an indexed document), only for files outside every
collection reached via `target: "*"` (e.g. a README).

I verified the multi-component/case/nonexistent-path logic directly with a throwaway unit test
(created a nested `Story/Target.md` fixture, asserted exact match found, leaf-case-mismatch not
found, dir-case-mismatch not found, missing-parent not found, missing-file not found — all
passed), then removed that scratch test before committing since it wasn't part of the ticket's
required deliverables.

## Verification
- `cargo fmt --check` — clean, no diff.
- `cargo clippy --workspace --all-targets -- -D warnings` — clean, zero warnings.
- `TMPDIR=/home/thw-home/.cache/typdoc-tmp scripts/test.sh` — **1000 test(s) passed across 56
  suite(s) on linux, 0 failed** (baseline ~999 + the 1 new test this ticket adds).
- `cargo test -p typdoc-core --lib refs::` — all 27 ref-resolution unit tests pass.
- `cargo test -p typdoc --test validate a_ref_whose_case` — both the pre-existing
  `a_ref_whose_case_differs_from_the_files_is_not_found` and the new
  `a_ref_whose_case_exactly_matches_a_file_outside_every_collection_is_found` pass.

Explicit note on the mismatch test: `a_ref_whose_case_differs_from_the_files_is_not_found` was
**already green on this Linux/ext4 machine before this change** (ext4 is case-sensitive, so
`root.join(path).is_file()` already correctly answered `false` for a mismatched-case path here).
It is still green after the change — no red-then-green transition happened or was expected
locally. The real red-before-green proof for this bug (the fallback wrongly answering "found" on
a case-insensitive filesystem) can only happen on a real macOS CI runner, which is explicitly out
of scope for this ticket's build and will be run separately.

## Notes
For whoever runs the macOS CI proof next: the fix's correctness on APFS rests on
`std::fs::read_dir` + exact `OsStr` comparison being unaffected by APFS's case-folding at the
`stat`/`open` layer — `read_dir` always returns filenames with their real on-disk casing
regardless of how the directory is case-folding, so the exact-byte comparison should behave
identically on APFS to how it behaves on ext4. If the macOS run still goes red, the first thing
to check is whether `OsStr` comparison on macOS's filesystem API normalizes Unicode differently
(HFS+/APFS historically NFD-normalizes filenames) — that's a different, narrower issue than case
folding and wasn't in this ticket's scope, but worth ruling out if the proof doesn't come back
clean.
