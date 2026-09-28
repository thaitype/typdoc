# Contract

## Where Windows differs (the whole compile surface, measured with `cargo check --target x86_64-pc-windows-gnu`)

Only two places fail to compile; every other unix-only call is already behind `cfg(unix)` (`process_env.rs`,
`namespace_lock::path_bytes`).

1. `crates/typdoc-fs/src/lib.rs` (`SystemFs`): `mode`, `set_mode`, `file_id` (`dev`, `ino`, `nlink`).
2. `crates/typdoc/src/signals.rs`: `signal_hook::iterator` does not exist on Windows.

## The refusal

Pending the owner's decision (Windows read-only, and the wording of the refusal). TK-2 is not built until
it is made; this section is the proposal.

- On Windows, `SystemFs::create_new` returns `io::ErrorKind::Unsupported` with a message saying typdoc does not
  write files on Windows yet. Every write creates its first file through `create_new` (the lock file first,
  then `new`'s document or a temp file), so every write command, present or future, is refused before any lock
  file or document exists. The refusal sits in the one crate that changes files, not in a list of commands.
  The only side effect before it is `create_dir_all` of `.typdoc/locks/` (an empty folder).
- The command exits 6 (I/O: a file cannot be written), the existing code; no new exit code.
- `mode`, `set_mode`, `identity_at` and the handle's `identity` return `Unsupported` on Windows too. They are
  unreachable behind the refusal, and none of them returns a made-up value: no identity of zeros, no mode
  that always matches.
- `signals::install` is `cfg(unix)`; on Windows it installs nothing. With writes refused no lock file can
  exist to clean up. When writes come to Windows, this is reopened.
- Unix code paths are unchanged byte for byte in behaviour: the Windows branches are additions under
  `cfg(windows)` / `cfg(not(unix))`.

## CI

- New job `windows-build` (blocking) on `windows-latest`:
  `cargo build --locked -p typdoc -p typdoc-core -p typdoc-fs`. Its own check turns red when the build fails.
  (Making it a required check in branch protection is a repository setting, the owner's.)
- `windows-pass-rate` stays as it is (non-blocking measurement).

## Testing Decisions

- Linux/macOS: the existing suite (`scripts/test.sh`, CI ubuntu + macOS) is the proof that nothing changed.
- Windows compile: `windows-build` in CI, and locally `cargo check --target x86_64-pc-windows-gnu -p typdoc
  -p typdoc-core -p typdoc-fs` (the target is installed on this machine).
- The job must be shown able to fail: its first run on PR #26 is before the fix (red, same 6 errors as
  `main`'s pass-rate job), then green after. Locally: plant a unix-only call outside `cfg`, see the Windows
  check red, remove it.
- The refusal's wording and exit 6 on a real Windows machine: Not verified (tests on Windows are out of
  scope). Recorded as such, not claimed.
