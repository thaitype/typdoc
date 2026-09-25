# How to check a project in CI

Run `typdoc validate` on every push and pull request, so a broken link, a bad value or a
duplicate key fails the build instead of reaching `main`.

## The check

```console
$ typdoc validate
```

It exits 2 if any finding is an error, and 0 otherwise. Warnings are printed but don't fail the
run. That exit code is all CI needs.

If you want warnings to fail too, add `--strict`, which treats every warning as an error:

```console
$ typdoc validate --strict
```

For a fast check that only looks at the schemas and config, not every document, use `--schemas`.
It suits a pre-commit hook.

## GitHub Actions

Pin a version in CI, so a new typdoc version can't change what `validate` reports without you
choosing to upgrade. This workflow checks a project at the root of the repository:

```yaml
name: docs
on:
  push:
    branches: [main]
  pull_request:

jobs:
  typdoc:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Install typdoc
        run: cargo install --locked typdoc --version 0.3.0
      - name: Check documents
        run: typdoc validate
```

If the project is in a subfolder, point typdoc at it with `TYPDOC_DIR`:

```yaml
      - name: Check documents
        run: typdoc validate
        env:
          TYPDOC_DIR: docs/tracker
```

Building typdoc takes a minute or two on each run. Caching `~/.cargo` between runs, for example
with `actions/cache`, saves most of it.

## Projects that import other projects

On a developer's machine, a missing import is only a warning, so that someone without the other
project checked out can still work. In CI you usually want the opposite: check out every project
the refs point into, and make a missing one fail. Set `imports.absent` to `error`:

```json
{ "version": 1, "validation": { "global": { "imports.absent": { "level": "error" } } } }
```

Or keep it a warning in the config and run `typdoc validate --strict` in CI, which raises every
warning, `imports.absent` included.

See [link to another project](link-projects.md) for how imports are found on each machine.

## Reading the output in CI

Each finding is one line:

```
path             level  rule               message
tickets/TK-3.md  error  frontmatter.types  the field `status` is not one of the schema's values: `in-progress`
```

For tooling that wants to annotate files, `typdoc validate --json` prints every finding with its
`path`, `line`, `col`, `rule` and `message`. The [validation rules reference](../reference/validation.md)
explains each rule.
