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
- [TK-7](_tickets/TK-7-windows-ctrl-c.md): decided: Ctrl+C cleanup through `SetConsoleCtrlHandler` (`windows-sys`), the process then exits `STATUS_CONTROL_C_EXIT`, and SPC-3 says so for Windows; tests send Ctrl+Break to a child in its own process group
- [TK-12](_tickets/TK-12-measure-on-windows.md): measured on the runner (NTFS): delete is POSIX, a held handle then shows links 0 and delete-pending; rename fails over read-only and over a target open without share-delete; Ctrl+Break to a child's group works, exit 0xC000013A
- TK-12, second probe: a POSIX-semantics rename with ignore-read-only replaces a read-only target and keeps a read-only temp's flag; a target open without share-delete fails with sharing violation (32); no reader saw the target missing
- [TK-10](_tickets/TK-10-windows-lock-release.md): decided: identity from `FileIdInfo`; the lock is ours while the handle has links and is not delete-pending and the path's id is the held one; access denied at the path is not ours, so nothing is removed (blocked by TK-9 for the id width)
- A replace that fails with sharing violation (os error 32) exits 6 with: `typdoc: notes/a.md: another program has this file open and does not let it be replaced; close it there and run the command again`; any other I/O error keeps today's message
- The shell-examples tests are Unix only, with the reason that SPC-13's shells are POSIX shells, and SPC-13 says that on Windows its examples are not run: a stated scope limit. The config collection-name test's Windows form leaves out `a:b` (an NTFS stream, not a name)
- The other Windows failures at 812b562 (the `frontmatter.transitions` fixture, the coverage exit-code test, the lock-contention holder, two lock-timeout tests) fail on the refusal, since a write takes its lock before it checks anything, and go with it
- The lock left behind when a handle's identity cannot be read stays out of this story: the Windows lock reads its identity from the handle it has just created, as on Unix
- [TK-8](_tickets/TK-8-windows-permissions.md): decided: the DACL and the read-only flag are carried to the temp before the rename; a DACL that cannot be read or set stops the write
- [TK-9](_tickets/TK-9-refs-file-id-width.md): decided: `FileId` holds a 128-bit id; `typdoc-core` API change in the CHANGELOG
- Built: TK-13 (writes), TK-14 (Ctrl+C), TK-15 (`Windows tests` gate). At c8c60a1 the Windows suite passes 1122 / 1122; three faults planted on a throwaway pull request turned the six new Windows tests red
- Part C built (TK-16 to TK-20): Windows archives for x86_64 and aarch64 (aarch64 builds and runs `typdoc.exe --version` on a `windows-11-arm` runner); `pages/install.ps1`, whose eleven scenarios pass on `windows-latest` under Windows PowerShell 5.1 and 7, the one-liner included with the script served as `application/octet-stream`; the live check proves `/install.ps1` is this repository's, and the Windows install from it is checked only after a release; the Windows docs wait in `windows-docs-for-release.md`. Two faults planted in the installer on a throwaway pull request turned its checksum and missing-archive scenarios red
- `ReplaceFileW` is not used: it is not atomic, so it would weaken the replace guarantee (TK-6)

## Not yet specified

## Out of scope

- A Windows target in dist, `install.ps1`, documents saying Windows is supported; a release.
