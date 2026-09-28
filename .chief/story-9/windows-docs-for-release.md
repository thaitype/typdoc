# Windows docs, for the release branch

These changes say Windows is supported. They are applied on the branch of the first release whose
assets include `typdoc-x86_64-pc-windows-msvc.zip`, and not before: until that release is
published, the installer answers that the latest release has no Windows build, and `cargo install`
on Windows builds a version that is not released. Nothing here is on `main` until then.

The default install directory below is `%USERPROFILE%\.local\bin`; if the owner chooses another
before that release, change it here, in `pages/install.ps1` (`$DefaultInstallDir`), in scenario 1
of `scripts/test_installer_windows.ps1` and in `publish.yml`'s Windows live install step.

## README.md, section "Install"

Replace

```
Supported platforms:

- macOS — amd64 and arm64
- Linux — amd64 and arm64

Windows is not supported yet.
```

with

````
On Windows, in PowerShell:

```powershell
irm https://typdoc.thaitype.dev/install.ps1 | iex
```

Supported platforms:

- macOS — amd64 and arm64
- Linux — amd64 and arm64
- Windows — amd64 and arm64
````

In the paragraph after it, "puts `typdoc` in `~/.local/bin`" becomes "puts `typdoc` in `~/.local/bin`
(`%USERPROFILE%\.local\bin` on Windows)".

## docs/how-to/install.md

In "With the install script", after the `curl` block, add:

````
On Windows, in PowerShell:

```powershell
irm https://typdoc.thaitype.dev/install.ps1 | iex
typdoc --version
```
````

and "This works on macOS and Linux, on amd64 and arm64" becomes "This works on macOS and Linux, on
amd64 and arm64, and on Windows on amd64 and arm64".

Replace the section "## Windows" with:

````
## Windows

The PowerShell installer works as the script does: it picks the build for your machine, checks its
SHA-256 checksum, installs `typdoc.exe` to `%USERPROFILE%\.local\bin`, and never changes PATH. If
that directory is not on PATH, it prints the command that adds it.

To pin a version or choose the directory, set the variable in the same PowerShell session, before
the pipe:

```powershell
$env:TYPDOC_VERSION = 'vX.Y.Z'; irm https://typdoc.thaitype.dev/install.ps1 | iex
$env:INSTALL_DIR = "$env:USERPROFILE\bin"; irm https://typdoc.thaitype.dev/install.ps1 | iex
```

A release from before Windows builds has none: pinned to one, the installer says so and installs
nothing.

`cargo install typdoc` works on Windows too, from that release on.

On Windows typdoc reads and writes as it does elsewhere. A document it replaces keeps its access
control list and its read-only flag; a document another program holds open without letting it be
replaced is left as it is, and typdoc says so. Ctrl+C ends it with exit code `0xC000013A`, after
the locks it holds are removed.
````

(`vX.Y.Z` is the release that first ships Windows builds.)

## Not verified

The binaries are not signed. A binary downloaded by `irm` and extracted by `Expand-Archive` is not
marked as coming from the internet, so SmartScreen is not expected to prompt when it is run from a
terminal; Microsoft Defender may still scan it. None of this was checked on a Windows machine.
