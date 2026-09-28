# Goal (part 2)

typdoc writes on Windows with the same guarantees it keeps on Unix, and the test suite shows it: every lock
and write test runs and passes on `windows-latest`, and the Windows test job blocks a pull request that
breaks one. On the way, CI builds each change once.

In order: CI triggers; tests compile and run on Windows, which measures what fails; then writes on Windows.

## Out of Scope

- A Windows target in dist, `install.ps1`, any doc or README saying Windows is supported; a release.
- The lock left behind when a handle's identity cannot be read after the lock file is created, unless the
  Windows lock work cannot avoid it.
