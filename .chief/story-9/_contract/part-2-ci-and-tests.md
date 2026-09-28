# Contract (part 2): CI triggers and tests on Windows

## CI triggers

- `ci.yml`, `publish-check.yml`: `on: push: branches: [main]` and `pull_request: branches: [main]`
  (`publish-check.yml` keeps its `paths`).
- A branch without a pull request gets no CI; a pull request is opened early, as a draft if need be.

## Tests on Windows

- A test is `cfg(unix)` only when what it tests does not exist on Windows (a signal, a Unix mode, a file name
  that is not UTF-8, a symlink made without the privilege Windows asks for), and the `cfg` says why in one line.
- A test whose behaviour should hold on Windows is not `cfg`'d out to make the count go up. Until writes come
  to Windows, the write tests fail there with the refusal; that failure is the measurement.
- `scripts/windows_test_report.py` says whether the build or the tests did not compile, from the output.

## Testing Decisions

- Windows compile of tests: `cargo check --workspace --tests --target x86_64-pc-windows-gnu` locally.
- Linux and macOS: the test counts stay as they are (Linux 1135 at the start of part 2).
- `scripts/test_windows_test_report.py` covers the new message with fixed sample output, as it does the others.
