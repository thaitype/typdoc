# 1: The write seam, the clock, and the narrowed write ban

Type: implementation
Status: resolved
Blocked by: None (can start immediately)

Throughout these tickets, `decision N` means ticket N of `docs/design-decision-phase-2/`.

## What this delivers

- `Deps` gains the file operations a write is built from, not `write this document` (decision 7): create with `O_EXCL`, write bytes, rename, remove, read a file's mode, set a mode, and ask whether two paths are one file.
- `Clock` in `deps`, giving an instant and the offset to write it in, at one second; the injected clock in tests returns a fixed instant.
- One function above the seam holds the policy of decision 4 — the temp file's reserved shape, the mode carried across, the rename — so no command sees a temp file.
- The write ban narrows rather than lifting: `typdoc-core`'s `clippy.toml` keeps every disallowed method, and the one module implementing the write half of the seam carries `#[expect(clippy::disallowed_methods, reason = ...)]` naming itself as the seam. `SystemTime::now` and the clock functions of `chrono` join the disallowed list.
- The fake file system and the real temporary directory behind one scenario table.

## Done when

- The gates pass, and clippy is shown red once by a write placed in a module that is not the seam, then removed.
- The scenario table runs against the fake and against a real temporary directory, and the fake stages what the real one will not: no space, permission refused, a rename across devices, and a stop between any two operations.
- A test names what neither covers — a power cut, and a `SIGKILL` between two system calls — so the gap is written rather than assumed.
- No command outside the seam's own module calls a function that writes.
