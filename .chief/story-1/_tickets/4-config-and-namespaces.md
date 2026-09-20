# 4: Config, namespaces and collections, and the config errors

Type: implementation
Status: open
Blocked by: 2

## What this delivers

- `config.json` in full for a read (`version`, `namespaces`, `validation`, `imports` is left to ticket 17), one namespace or several, and collection files with `match` templates (coded and glob), `schema`, `refBase` and `validation`.
- The config errors that need no schema, import, state file or pin, each with its id: `config.parse`, `config.version`, `config.unknown-key`, `config.legacy-file`, `config.collection-parse`, `config.collection-name`, `config.rule-unknown`, `config.rule-always-on`, `config.namespaces-entry`, `config.namespace-name` and `config.namespace-nested`. The error object on standard error carries `details` and `complete`, exit 2.
- The scope of a command: prefix, `--namespace`, `TYPDOC_NAMESPACE`, the current directory.

## Done when

- Every config error built here has a fixture in `broken/` and an exact expected id set, including `config.parse` and the case where `complete` is false.
- A fixture with several namespaces, and one with the namespace `default` that has no folder of its own.
