# How to install typdoc

Install a prebuilt binary with one command, or build from source with `cargo`.

## With the install script

```console
$ curl -fsSL https://typdoc.thaitype.dev/install | sh
$ typdoc --version
```

This works on macOS and Linux, on amd64 and arm64, with no Rust toolchain. The script picks
the release for your OS and architecture, checks its SHA-256 checksum, and installs `typdoc` to
`~/.local/bin`. If the download is corrupted or your platform isn't supported, it stops and
installs nothing.

On Linux the binary is statically linked against musl, so it doesn't depend on your distro's
glibc version.

### Pin a version or choose the directory

Set `TYPDOC_VERSION` to install a specific release, or `INSTALL_DIR` to install somewhere other
than `~/.local/bin`. The script reads them, not `curl`, so put them after the pipe, right before
`sh`:

```console
$ curl -fsSL https://typdoc.thaitype.dev/install | TYPDOC_VERSION=v0.3.1 sh
$ curl -fsSL https://typdoc.thaitype.dev/install | INSTALL_DIR="$HOME/bin" sh
```

Written before `curl` instead, the variable never reaches the script, and you get the latest
release in the default directory.

To upgrade, run the script again. To uninstall, delete the binary:

```console
$ rm ~/.local/bin/typdoc
```

### Add the directory to PATH

The script never edits `PATH` or your shell's startup files. If the install directory isn't on
`PATH` yet, it prints the lines to add it for your shell (zsh, bash or fish), for example:

```console
  ⚠ $HOME/.local/bin is not on your PATH yet.

  Add it permanently by running:
      echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc
      source ~/.zshrc
```

## Verify a download

Every release binary comes with a SHA-256 checksum, which the script checks for you, and a
[GitHub artifact attestation](https://docs.github.com/en/actions/security-guides/using-artifact-attestations-to-establish-provenance-for-builds)
showing it was built by this repository's release workflow. To check the attestation of an
archive you downloaded yourself:

```console
$ gh attestation verify typdoc-x86_64-unknown-linux-musl.tar.gz --owner thaitype
```

## If macOS blocks the first run

The binary isn't notarized by Apple. Installed with the script, it runs as is. If you downloaded
the archive with a browser and Gatekeeper blocks it, clear the quarantine flag once:

```console
$ xattr -d com.apple.quarantine ~/.local/bin/typdoc
```

## With cargo

If you have a Rust toolchain:

```console
$ cargo install typdoc
```

To try unreleased changes from `main`:

```console
$ cargo install --git https://github.com/thaitype/typdoc typdoc
```

## Windows

Windows is not supported yet, by the script or by `cargo install`. Run typdoc inside WSL
instead.
