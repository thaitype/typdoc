# 25: `resolve_path`'s fallback trusts the filesystem's own case-folding

Type: implementation
Status: open
Blocked by: None (can start immediately)

**Found 2026-09-24, running the real macOS CI proof for ticket 14/M-15** — `validate.rs`'s
`a_ref_whose_case_differs_from_the_files_is_not_found` fails on a real `macos-latest` runner.
Confirmed a real product bug, not a CI or test-environment artifact (Aria, verifying the same
run): the design is explicit that paths compare "exactly as they are written, case included, on
every platform... even where the file system would open it" — so this is in scope for v0.2.0
outright, no decision needed on whether to fix it, only how.

## The bug

`crates/typdoc-core/src/refs.rs`, `resolve_path` (around line 406):

```rust
pub(crate) fn resolve_path(path: &str, index: &Index, root: &Path) -> Outcome {
    if let Some(entry) = index.get(path) { ... }   // exact-string lookup — case-exact, fine
    if root.join(path).is_file() { ... }            // <-- a real stat() syscall, OS-dependent
    Err(Reason::NotFound)
}
```

The index lookup is case-exact by construction (a map keyed by the real on-disk path strings).
The fallback is not: `Path::is_file()` asks the OS, and on a case-insensitive-but-preserving
filesystem (macOS's default APFS; also Windows, though Windows is out of scope per M-12) the OS
resolves a wrongly-cased path to the real file and answers `true`. A ref written `target.md`
against a real file `Target.md` is then treated as resolving, contradicting the design's own
guarantee and silently letting through exactly the kind of mismatch `refs.resolve` exists to
catch.

## The work

Make the fallback branch verify the resolved entry's actual on-disk name matches `path` exactly,
byte for byte, before treating it as found — not trust `is_file()`'s yes/no alone. This likely
means reading the parent directory's entries and comparing the last path component's bytes
directly (e.g. via `std::fs::read_dir` and an exact `OsStr` comparison), rather than relying on
any single syscall whose case sensitivity varies by filesystem. The fix needs to hold on both
case-sensitive and case-insensitive filesystems: on ext4 it should behave exactly as it does
today (nothing currently passing may start failing); on APFS it should now correctly report
`NotFound` for a case-mismatched path instead of resolving it.

## Tests

- `a_ref_whose_case_differs_from_the_files_is_not_found` (already exists, `validate.rs:1386`)
  passes on a real macOS run, not just on Linux — this is the test that surfaced the bug and the
  one that proves the fix; confirm via the same CI red-before-green discipline this story already
  uses, on a throwaway branch, before trusting a green result.
- A new test confirming case-EXACT matches still resolve correctly on both platforms (the fix
  must not turn a correct match into a false negative).
- Re-run the existing ref-resolution test suite in full on Linux to confirm no regression from
  changing the fallback's mechanism.

## Done

- `resolve_path`'s fallback verifies exact case before reporting a path as resolved, on every
  supported platform.
- The existing case-mismatch test passes on a real macOS CI run (not just locally, not just on
  Linux).
- No regression in `typdoc-core`'s existing ref-resolution test suite.
