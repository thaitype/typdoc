## Destination

typdoc writes on Windows with the guarantees it keeps on Unix: every lock and write test runs and passes on
`windows-latest`, and the Windows test job is a blocking gate. Tests that cannot apply on Windows are `cfg`'d
out with a stated reason, never skipped to go green. The fog is clear when the Windows forms of file identity,
mode, rename-over and Ctrl+C cleanup are each decided.

## Notes

- Measured before B (TK-4): 982 / 1116 tests pass on Windows; 122 of the failures are the read-only refusal.
- `windows-sys` 0.61 is already in `Cargo.lock` (through clap); as a direct dependency it is expected.
- `windows_by_handle` (std's Windows file id and link count) is unstable on 1.96.
- Decisions that touch a guarantee, or what a user sees, go to the owner through the director.
- Test-only Windows failures (a `:` in a file name, path separators in messages) must be fixed before the job
  becomes blocking.

## Decisions so far

- [TK-11](_tickets/TK-11-absolute-path-before-prefix.md): an absolute path, and on Windows `\`, `.\`, `..\`, is on disk before any prefix; `C:note.md` stays a namespace (SPC-2)
- [TK-5](_tickets/TK-5-windows-file-identity.md): identity from a handle, 64-bit volume + 128-bit id; ReFS 64-bit index not unique; after delete `DeletePending`, links 0 by spec
- [TK-6](_tickets/TK-6-windows-rename-over.md): rename keeps the temp's inherited ACL, fails over read-only; `ReplaceFileW` keeps the DACL but is not atomic and has gaps
- [TK-7](_tickets/TK-7-windows-ctrl-c.md): `SetConsoleCtrlHandler` via `windows-sys`; tests aim Ctrl+Break at a child's process group; exit with `STATUS_CONTROL_C_EXIT`
## Not yet specified

- How the Windows form of the signal tests delivers Ctrl+C to a child (after TK-7).
- Undiagnosed Windows failures: the config collection-name test, the `frontmatter.transitions` fixture in the
  coverage test, the lock-contention holder's lock not appearing (likely the refusal).
- Whether the lock left behind when a handle's identity cannot be read must be fixed for the Windows lock work.

## Out of scope

- A Windows target in dist, `install.ps1`, documents saying Windows is supported; a release.
