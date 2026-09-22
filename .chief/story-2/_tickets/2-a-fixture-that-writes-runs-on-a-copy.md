# 2: A fixture whose command writes runs on a copy

Type: implementation
Status: claimed
Blocked by: None (can start immediately)

## What this delivers

- The fixture loader copies a fixture whose declared command is a write to a temporary directory and runs the copy. Today every fixture declares a read and runs in the repository's own tree (`Spawn::args(&spec.command).cwd(dir)`, with `dir` under `fixtures/broken/`), which a write would modify.
- The loader enforces it, so no individual test has to remember.

## Done when

- A fixture that declares a write and is pointed at the repository's tree is a failure of the harness, and that check is shown red once.
- After a write fixture runs, the fixture in the repository is byte-identical to what it was before, asserted rather than assumed.
- Every fixture that declares a read still runs as it did.
