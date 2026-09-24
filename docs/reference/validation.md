# Validation rules

`typdoc validate` reports findings. Each finding has a rule id, a level and a message. This page
lists every rule in typdoc 0.2.0.

## Levels

| Level | Effect |
| --- | --- |
| `error` | `validate` exits 2 |
| `warn` | printed; exit 0 unless `--strict` |
| `info` | only in `--audit`, for rules that are switched off |
| `off` | not checked |

Always-on rules can't be configured. Configurable rules take their level from typdoc's default,
then `validation.global` in `.typdoc/config.json`, then the collection file's `validation`. Each
layer only needs the keys it changes:

```json
{ "validation": { "global": { "body.links": { "level": "warn" } } } }
```

## Always-on rules

| Rule | Reports |
| --- | --- |
| `schema.valid` | A broken schema: duplicate name or code, an `extends` loop, a redefined field without `override`, an unknown option, a field name that isn't allowed, a `target` naming a schema that doesn't exist |
| `frontmatter.parse` | Frontmatter that isn't valid YAML. No other rule checks that file |
| `frontmatter.types` | A value of the wrong type, a missing required field, or an enum value that isn't allowed |
| `frontmatter.transitions` | A status change that `transitions` doesn't allow |
| `refs.resolve` | A frontmatter ref to a document that doesn't exist |
| `refs.target` | A ref to a kind of document the field's `target` doesn't allow |
| `refs.acyclic` | A loop through an `acyclic` field; reported on every document in the loop |
| `keys.unique` | Two files with the same key |
| `collections.overlap` | A file matched by two collections |
| `state.missing` | A numbered collection with documents but no `last` in its state file. `new` refuses until it's recorded |
| `state.malformed` | A `last` that isn't a usable whole number |
| `state.behind` | (warn) A `last` lower than the highest existing key. The next number is still correct; the file isn't |
| `state.retired` | (warn) A state entry for a collection that no longer exists. Kept on purpose, since it's the only record those numbers were used |
| `files.unreadable` | A symbolic link, or a name that isn't valid UTF-8, where a collection or namespace would look. The file is skipped |

## Configurable rules

| Rule | Default | Options | Reports |
| --- | --- | --- | --- |
| `body.links` | error | `ignore`: globs of link targets to skip | A body link to a file that doesn't exist. Also text that looks like a link but isn't one, usually because of a space in the path (write `<my file.md>` or `my%20file.md`), and a reference-style label defined twice |
| `body.anchors` | error | | A `#heading` in a link that doesn't exist in the target. `typdoc toc` shows the right slugs |
| `body.mentions` | off | `inlineCode` (true), `fencedCode` (false) | A key mentioned in plain text, like "see WF-3", that doesn't exist |
| `refs.codedByPath` | warn | | A numbered document referred to by path instead of by key |
| `refs.moved` | error | | A ref to a document's old name, when its schema records moves with an `auto: moves` field. The message gives the new name |
| `names.shadowed` | warn | | A name that is both a namespace and an import alias |
| `frontmatter.unknown` | warn | | A frontmatter field the schema doesn't declare |
| `filename.pattern` | error | | A file in a numbered collection's folder that fits no template, such as `tickets/README.md` |
| `imports.absent` | warn | | A ref into an imported project that isn't on this machine |

## Config errors

Problems with the configuration itself. If one makes checking impossible, the command stops with
exit 2 and lists every config error it found. Otherwise it's reported as a finding like any other.

| Id | Reports |
| --- | --- |
| `config.parse` | `config.json` isn't valid JSON |
| `config.version` | `version` is missing or not `1` |
| `config.unknown-key` | An unknown key in `config.json` or a collection file |
| `config.collection-parse` | A collection file isn't valid JSON |
| `config.collection-name` | A collection file name with characters other than ASCII letters, digits, `-`, `_` |
| `config.collection-schema` | A collection naming a schema that doesn't exist |
| `config.rule-unknown` | An unknown rule or rule option |
| `config.rule-always-on` | A level set on an always-on rule |
| `config.match-template` | A `match` that breaks the template rules (`{key}` once and no wildcards for numbered schemas; no `{key}` otherwise) |
| `config.coded-schema-shared` | Two collections using the same numbered schema |
| `config.state-uncoded` | A state entry for a collection whose schema has no code |
| `config.state-orphan` | A state file for a namespace that no longer exists; delete or rename it |
| `config.namespaces-entry` | A `namespaces` entry with `/` or `**`, or naming a missing folder or a symbolic link |
| `config.namespace-name` | A namespace folder name that isn't allowed |
| `config.namespace-nested` | A namespace folder with its own `.typdoc/` |
| `config.schema-url` | A schema URL that isn't `http://` or `https://` |
| `config.schema-unpinned` | A remote schema; 0.2.0 can't fetch these |
| `config.vendor-missing` | A saved copy of a remote schema is missing |
| `config.vendor-edited` | A saved copy of a remote schema was edited by hand |
| `config.config-dir` | `TYPDOC_CONFIG_DIR` isn't an absolute path to an existing folder |

## Known gaps in 0.2.0

- A body link into an imported project is checked for its file, not for its `#heading`.
- `refs --reverse` doesn't look inside projects that import this one.
