# 17: Imports: finding other projects and following refs into them

Type: implementation
Status: open
Blocked by: 7, 10, 14

## What this delivers

- `imports` in config, `imports.json` found by `TYPDOC_CONFIG_DIR`, `XDG_CONFIG_HOME`, then the platform default, `${ENV}` in import paths, one level only.
- `config.config-dir` for a `TYPDOC_CONFIG_DIR` that is not an absolute path to an existing directory.
- `imports.absent`, `project::` in refs and in arguments, `project` in the name of a document, `--namespace 'chief::*'` reaching an imported project, and the reason `import-absent`.

## Done when

- The five cases of finding `imports.json` are tested with a fake `Env`; an unset or empty variable makes the import absent; `imports.absent` at `error` gives exit 2 and an absent import that nothing refers to is not reported.
- `imports.absent` leaves `unimplemented_rules`.
