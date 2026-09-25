---
title: Installing typdoc
status: active
---

## Supported platforms

Prebuilt binaries exist for four targets: macOS x86_64 and aarch64, Linux x86_64 and aarch64.
Windows is not supported yet — not by the installer, and not by `cargo install` either, which
has never compiled there. `typdoc-fs`, the crate implementing `typdoc_core::Fs`, is Unix-only by
design, so a Windows binary cannot compile at all today. Supporting Windows is its own later
story, not a gap in this one.

The two Linux targets are musl, not gnu. A gnu-linked binary refuses to start on a distro whose
glibc is older than the one it linked against; musl binaries are statically linked and carry no
glibc dependency at all, which removes that floor entirely instead of managing it by pinning an
old build image. The only requirement left is a kernel new enough for Rust's own baseline for
the target (3.2+ on x86_64, 4.1+ on aarch64) — no glibc version requirement.

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
$ curl -fsSL https://typdoc.thaitype.dev/install | TYPDOC_VERSION=v0.3.1 sh
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

`cargo install typdoc` stays a supported install path alongside the script, on every platform it
already worked on.

## What a release carries

Every release carries, for each of the four targets: the archive, a `.sha256` checksum file next
to it, and a GitHub artifact attestation (`actions/attest-build-provenance`), independently
verifiable with `gh attestation verify`. A checksum only catches a corrupted download; the
attestation shows the binary was actually built by this repository's own workflow.

`x86_64-apple-darwin` is the one target with no matching-arch GitHub-hosted runner available to
this repository, so it is never executed in CI — only cross-compiled. It is named explicitly as
built-but-not-run in the release notes; a passing cross-compile is never presented as a binary
that runs.

## The Pages site

`typdoc.thaitype.dev` serves exactly two files: the installer script itself, and a root
`index.html` that redirects to this GitHub repository. Nothing else lives there.

## Deferred

The release-target list (the four targets above) may become a `catalog/` entry of its own later,
checked against `dist-workspace.toml` and `pages/install` so the three can never silently drift
apart. Not built in this story — no catalog entry exists yet for it.
