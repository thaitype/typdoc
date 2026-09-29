---
title: Installing typdoc
status: active
---

## Supported platforms

Prebuilt binaries exist for six targets: macOS x86_64 and aarch64, Linux x86_64 and aarch64,
Windows x86_64 and aarch64. Windows builds start with 0.6.0; a release before it has none.
`cargo install typdoc` builds on every one of these platforms.

The two Linux targets are musl, not gnu. A gnu-linked binary refuses to start on a distro whose
glibc is older than the one it linked against; musl binaries are statically linked and carry no
glibc dependency at all, which removes that floor entirely instead of managing it by pinning an
old build image — no glibc version requirement, on any distro whose kernel is new enough to run
a modern Linux binary at all. Proven in CI, not just asserted: each musl binary also runs inside
an intentionally old, unrelated distro's container image, on both arches.

## The installer script

`https://typdoc.thaitype.dev/install` is a POSIX `sh` script, piped into a shell:

```console
$ curl -fsSL https://typdoc.thaitype.dev/install | sh
```

It detects the OS and architecture, downloads the matching release archive, and verifies its
SHA-256 checksum **before** installing anything — a checksum mismatch or an unsupported platform
fails loudly and installs nothing. There is no partial-success outcome: the script exits `0` on
success, non-zero on any failure.

Two environment variables change its behavior — `TYPDOC_VERSION` (install this exact release tag
instead of the latest one) and `INSTALL_DIR` (install into this directory instead of the default,
`~/.local/bin`). Both are read by the script, not by `curl`, so they belong after the pipe, right
before `sh`, not before `curl`:

```console
$ curl -fsSL https://typdoc.thaitype.dev/install | TYPDOC_VERSION=v0.7.0 sh
```

The script never edits `PATH` or any shell startup file. If the install directory isn't already
on `PATH`, it prints how to add it instead, detected from the user's login shell
(`basename "$SHELL"` — never the shell running the script itself, which under the pipe is always
`sh`, whatever the user actually uses interactively):

- `zsh` — the two lines to append the export to `~/.zshrc` and source it.
- `bash` — the same shape, to `~/.bashrc` on Linux or `~/.bash_profile` on macOS: a login-shell
  Terminal window on macOS reads the latter, not the former, so the two are genuinely different
  files, not a detail worth blurring into one.
- `fish` — `fish_add_path <dir>`, which persists on its own and takes effect immediately; `fish`
  has no `export` syntax, so the script never shows one for it.
- anything else, or an unset `$SHELL` — a shell-agnostic pointer to add the directory in
  whichever startup file applies, plus the one `export PATH=...` line that works for the current
  session regardless of shell.

The directory is shown as `$HOME/...` when it's actually under `$HOME`, and as a plain absolute
path otherwise. Where it's under `$HOME`, the persisted command keeps `$HOME` as literal,
unexpanded text (the script builds it inside single quotes) so the line stays correct regardless
of which `$HOME` happened to be set when the installer ran.

## The Windows installer

`https://typdoc.thaitype.dev/install.ps1` is the same installer for Windows, piped into PowerShell:

```powershell
irm https://typdoc.thaitype.dev/install.ps1 | iex
```

It runs under Windows PowerShell 5.1 and PowerShell 7 and behaves as the script does: it takes
the tag from `TYPDOC_VERSION` or from the latest release, downloads the archive for the machine's
architecture, verifies its SHA-256 checksum before installing anything, and installs `typdoc.exe`
to `INSTALL_DIR`, by default `%USERPROFILE%\.local\bin`. A release that has no Windows archive, a
tag that does not exist, or a checksum mismatch fails with one `error:` line and installs nothing.
The variables are set in the same session before the pipe:

```powershell
$env:TYPDOC_VERSION = 'v0.7.0'; irm https://typdoc.thaitype.dev/install.ps1 | iex
```

It never edits `PATH`, for the user or the machine. If the directory is not on `PATH`, it prints
the command that adds it for the user permanently and the one that adds it to the current session.

`cargo install typdoc` stays a supported install path alongside the scripts, on every platform.

## What a release carries

Every release carries, for each of the six targets: the archive (`.tar.gz`, or `.zip` for
Windows), a `.sha256` checksum file next
to it, and a GitHub artifact attestation (`actions/attest-build-provenance`), independently
verifiable with `gh attestation verify`. A checksum only catches a corrupted download; the
attestation shows the binary was actually built by this repository's own workflow.

`x86_64-apple-darwin` is the one target with no matching-arch GitHub-hosted runner available to
this repository, so it is never executed in CI — only cross-compiled. It is named explicitly as
built-but-not-run in the release notes; a passing cross-compile is never presented as a binary
that runs.

## The Pages site

`typdoc.thaitype.dev` serves exactly three files: the two installer scripts, `install` and
`install.ps1`, and a root `index.html` that redirects to this GitHub repository. Nothing else lives
there.

## Deferred

The release-target list (the six targets above) may become a `catalog/` entry of its own later,
checked against `dist-workspace.toml`, `pages/install` and `pages/install.ps1` so the four can
never silently drift apart. No catalog entry exists yet for it.
