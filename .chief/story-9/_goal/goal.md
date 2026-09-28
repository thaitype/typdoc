# Goal

`cargo build` of the shipped crates (`typdoc`, `typdoc-core`, `typdoc-fs`) succeeds on Windows, and a CI job on
`windows-latest` fails whenever it does not. Linux and macOS behave exactly as before.

On Windows the built binary reads (`get`, `list`, `refs`, `toc`, `validate`) and refuses every write with a
clear error before it touches a file, because the guarantees a write depends on (the identity check that keeps
a lock from being released by the wrong process, the mode carried to a replaced file) have no Windows
implementation yet. A Windows build that quietly loses a lock guarantee is worse than no Windows build.

## Out of Scope

- Tests compiling or passing on Windows; the pass-rate job stays a non-blocking measurement.
- Writing on Windows: file identity (no stable std API on 1.96: `file_index`, `volume_serial_number`,
  `number_of_links` are behind `windows_by_handle`), modes/ACLs, Ctrl+C cleanup.
- `install.ps1`, a Windows target in dist, any doc or README saying Windows is supported, a release.
