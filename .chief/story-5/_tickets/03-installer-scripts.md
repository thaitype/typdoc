Type: implementation
Status: claimed
Blocked by: 01

# Ticket 03 — installer scripts (`pages/install`, `pages/install.ps1`)

Produce the two scripts at their canonical location, `pages/install` and `pages/install.ps1`.
For each script independently: use dist's generated shell/powershell installer if it can be
configured to satisfy every point below; otherwise hand-write that one script. State in the PR
description which way each script went — do not leave it for the diff to reveal.

Contract each script must satisfy (see `_contract/contract.md`):
- OS/arch detection → one of the five targets; unsupported platform prints an error and exits
  non-zero, no closest-match fallback.
- Version resolution: `TYPDOC_VERSION` if set, else latest via the `releases/latest` redirect
  (never the REST API).
- `TYPDOC_INSTALL_BASE_URL` test-only override for the download base (undocumented publicly —
  comment in the script only).
- Download archive + `.sha256`, verify before installing anything outside a temp dir; loud
  failure and non-zero exit on mismatch.
- Install path: `${INSTALL_DIR:-$HOME/.local/bin}` on macOS/Linux; always
  `%LOCALAPPDATA%\typdoc\bin` on Windows (one fixed path regardless of generator). `INSTALL_DIR`
  overrides both.
- No `PATH`/shell-rc edits anywhere, ever — including turning off dist's own path modification if
  its generated installer is used. Print the one line to run instead.
- Exit 0 on success, non-zero on any failure.

Add the PR-time test: start a local HTTP server serving the archives + checksums produced by
ticket 01's reusable-workflow PR-triggered caller (plus
a second fixture "version" for the `TYPDOC_VERSION` case), point the scripts at it via
`TYPDOC_INSTALL_BASE_URL`, and exercise on all three OS families: default install, `INSTALL_DIR`
override, `TYPDOC_VERSION` pin, and an intentionally-corrupted checksum (must fail loudly,
install nothing).

Docs (same PR): README + `docs/getting-started.md` install one-liners, `INSTALL_DIR` mention,
`gh attestation verify` usage, the macOS `xattr -d com.apple.quarantine` note and the Windows
SmartScreen note for browser-downloaded files, and the relevant `skills/typdoc/` update.

Demoable on its own: the local-server CI test passes on all three OS families, without Pages or
DNS existing.
