# Validation

typdoc 0.2.0. `typdoc validate` checks the project, or the documents named, against its schemas
and rules. Every finding carries a `rule` id; this page lists every id 0.2.0 can report, what it
means, and the usual fix.

## Running it

```console
$ typdoc validate                          # whole project
$ typdoc validate WF-2 notes/x.md          # only these documents
$ typdoc validate --schemas                # schemas only (fast; good for pre-commit and CI)
$ typdoc validate --strict                 # every remaining warn becomes error
$ typdoc validate --audit                  # adoption report; exits 0 unless the config is unreadable
```

Exit 2 if any finding is at level `error`; warnings alone exit 0. The report is on stdout either
way.

### Text

```
path                        level  rule                 message
.typdoc/state/default.json  warn   state.behind         the collection `tickets`'s `last` in its state file is 5, lower than …
notes/anchors.md:4:5        error  body.anchors         the heading `#no-such-heading` does not exist in `notes/getting-started.md`
notes/notitle.md            error  frontmatter.types    the field `title` is required and is missing
```

`path:line:col` when a position is known. Lines count from the top of the file, frontmatter
included (what an editor shows); columns count characters. No findings prints nothing.

### `--json`

```json
{"summary":{"scope":"paths","strict":false,
            "checked":{"namespaces":["default"],"documents":1,"paths":["notes/broken.md"]},
            "findings":{"error":1,"warn":1,"info":0}},
 "findings":[{"path":"notes/broken.md","namespace":"default","collection":"notes","field":"bad_field",
              "rule":"frontmatter.unknown","level":"warn","message":"…"},
             {"path":"notes/broken.md","namespace":"default","collection":"notes","line":5,"col":5,
              "rule":"body.links","level":"error","message":"link target missing: nowhere.md"}]}
```

`summary.scope` is `all`, `paths` or `schemas`; `summary.checked` says what was covered, so an
empty `findings` is never read as more than was checked. A finding always has `rule`, `level`,
`message`, `path`; it has `namespace`, `collection` and `key` when the file is a document,
`field` when it is about one field, and `line`/`col` when a position is known. Findings are
ordered by path, then line, col, rule, message.

### `--audit`

For deciding whether and how to adopt typdoc on a folder that already exists. Rules switched off
are reported as `info`, files in no collection and files with no frontmatter are listed instead of
failing, and the exit is 0 unless the config itself cannot be read:

```
typdoc audit: 2 collections, 10 files (2 in no collection)

notes    4 files   body.anchors 1 error · frontmatter.parse 1 error · frontmatter.types 1 error · frontmatter.unknown 1 warn
tickets  4 files   state.behind 1 warn · refs.codedByPath 1 warn

in no collection: tickets/README.md, tickets/WF-2-copy.md (2)
```

## Levels and configuration

Levels are `off`, `warn`, `error` (plus `info` in an audit). **Always-on** rules cannot be
configured. **Configurable** rules take their level from typdoc's default, then
`validation.global` in `.typdoc/config.json`, then the collection file's `validation`, each
merging key by key:

```json
"validation": { "global": { "body.links": { "level": "error", "ignore": ["assets/**"] },
                            "frontmatter.unknown": { "level": "off" } } }
```

## Always-on rules

| Rule | Reported when | Usual fix |
| --- | --- | --- |
| `schema.valid` | A schema is broken: duplicate name or code, `extends` cycle, redefining an inherited field without `"override": true`, an invalid option, a field name outside `[A-Za-z_][A-Za-z0-9_-]*` or using a reserved name, an import alias that is a URL scheme, a qualified `target` naming a schema the imported project does not have | Fix the schema file named in the finding |
| `frontmatter.parse` | The frontmatter block is not valid YAML or never closes. No other rule runs on that file | Fix the YAML at the position given |
| `frontmatter.types` | A value has the wrong type, a required field is missing, or an enum value is not allowed | `typdoc set` the field to a valid value (the write is checked), or fix it by hand |
| `frontmatter.transitions` | An `enum` field moved between values its schema's `transitions` does not allow (checked on write) | Move through an allowed value |
| `refs.resolve` | A frontmatter ref points at nothing | Correct the ref, or create the target |
| `refs.target` | A ref points at a document whose schema the field's `target` does not allow | Point it at an allowed kind of document |
| `refs.acyclic` | A cycle runs through an `acyclic` field; reported on every document in the cycle. `set`/`new` do not refuse a cycle in 0.2.0, so this is where it shows up | Remove one ref of the cycle (`typdoc set <doc> field=` or set it to the others) |
| `keys.unique` | Two files share a key | Decide which document keeps the key, then give the other a different key (in a multi-namespace project, `typdoc mv <doc> --renumber <namespace>` issues a fresh one) and re-run `validate` |
| `collections.overlap` | A file is matched by two collections; it is checked against neither | Change the `match` templates so each file belongs to one |
| `state.missing` | A collection has coded documents in a namespace but no `last` recorded in `.typdoc/state/<namespace>.json`. `new` and `mv --renumber` refuse to issue a number there | Restore the file from version control, or write `{"<collection>": {"last": N}}` with N the highest number ever used |
| `state.malformed` | A recorded `last` is not a usable whole number (text, null, negative, fraction, too large) | Set it to the highest number ever issued |
| `state.behind` | (warn) `last` is lower than the highest existing key. Allocation is still right; the record is not | Raise `last` to the highest existing number, or just run the next `typdoc new`, which records it |
| `state.retired` | (warn) A state entry names a collection the project no longer has. Kept on purpose: it is the only record those numbers were issued | Nothing to do; never delete it |
| `files.unreadable` | A file or folder a `match` or `namespaces` glob reaches is a symbolic link or has a name that is not valid UTF-8; it is skipped and the run continues | Replace the link with the real file, or rename it |

Never lower `last` to match the files: a number whose document was deleted would be issued again,
and old refs to it would silently resolve to the new document. On a merge conflict in a state
file, keep the higher `last`.

## Configurable rules

| Rule | Default | Options | Reported when | Usual fix |
| --- | --- | --- | --- | --- |
| `body.links` | `error` | `ignore`: globs of relative targets to skip | A Markdown link in the body (inline, image, reference-style) points at a missing file; also text that looks like a link but is not (usually a space in the target — write `<my file.md>` or `my%20file.md`), and a reference label defined twice | Fix the path. `typdoc mv` keeps links right when you move files |
| `body.anchors` | `error` | — | A `#heading` in a link does not exist in the target | Use the slug `typdoc toc <target>` prints |
| `body.mentions` | `off` | `inlineCode` (true), `fencedCode` (false) | A key mentioned in plain body text (`see WF-3`) does not exist | Correct the key |
| `refs.codedByPath` | `warn` | — | A coded document is referenced by path (`WF-2.md`) instead of its key | Write the key (`WF-2`); a key survives moves |
| `refs.moved` | `error` | — | A ref points at an old name recorded in some document's `auto: moves` field; the message names the new key | Update the ref to the new name |
| `names.shadowed` | `warn` | — | One name is both a sibling namespace and an import alias | Rename one |
| `frontmatter.unknown` | `warn` | — | A frontmatter field the schema does not declare. `set` writes such a field anyway and this reports it | Remove it (`typdoc set <doc> field=`), fix its name, or add it to the schema |
| `filename.pattern` | `error` | — | A file in a coded collection's folder fits no `match` template (`tickets/README.md`) | Rename or move it out, or add a collection for it |
| `imports.absent` | `warn` | — | A ref reaches into an imported project that is not on this machine (including an import path whose `${VAR}` is unset) | Set up the import (see [project-layout.md](project-layout.md)); CI may set this to `error` |

A move is recorded only if the schema has a `list` field with `auto: moves`; without one, a
moved ref is an ordinary `refs.resolve` finding.

## Config errors

Reported when the config loads, with the id in `rule` and the config file in `path`. One that
makes checking impossible stops the command with exit 2 and an error object listing every config
error found (`"complete": false` if parsing stopped early); one that leaves checking possible is a
finding in `validate`'s report.

| Id | Reported when |
| --- | --- |
| `config.parse` | `config.json` cannot be parsed |
| `config.version` | `version` is missing or unknown (0.2.0 knows `1`) |
| `config.unknown-key` | `config.json` or a collection file has an unknown key |
| `config.collection-parse` | a collection file cannot be parsed |
| `config.collection-name` | a collection file's name uses anything but ASCII letters, digits, `-`, `_` |
| `config.collection-schema` | a collection names a schema that does not exist |
| `config.rule-unknown` | a rule name or option is unknown |
| `config.rule-always-on` | an always-on rule is configured |
| `config.match-template` | a `match` template breaks the placeholder rules (`{key}` exactly once and no globs for a coded schema; no placeholders for an uncoded one) |
| `config.coded-schema-shared` | two collections name the same coded schema |
| `config.state-uncoded` | a state entry names an existing collection whose schema has no code |
| `config.state-orphan` | a file in `.typdoc/state/` matches no current namespace — delete or rename it |
| `config.namespaces-entry` | a `namespaces` entry contains `/` or `**`, names a missing folder, or names a symbolic link |
| `config.namespace-name` | a namespace folder's name uses anything but ASCII letters, digits, `-`, `_`, or is `default`, `http`, `https`, `mailto`, `file` |
| `config.namespace-nested` | a namespace folder holds its own `.typdoc` |
| `config.schema-url` | a schema URL uses a scheme other than `http://` or `https://` |
| `config.schema-unpinned` | a remote schema has no pin. **0.2.0 cannot fetch remote schemas**, so any unpinned `http(s)://` schema is this error |
| `config.vendor-missing` | a pinned copy of a remote schema is missing |
| `config.vendor-edited` | a pinned copy was edited by hand |
| `config.config-dir` | `TYPDOC_CONFIG_DIR` is set but is not an absolute path to an existing directory |

The messages of `config.vendor-*` mention `typdoc pull`; that command does not exist in 0.2.0.
Restore the pinned copy from version control instead.

## Known gaps in 0.2.0

- `refs --reverse` scans this project's namespaces only, not the projects it imports, so a ref
  held in an imported project that points back here is missing from its result.
- A body link that crosses into an imported project has its file checked (`body.links`) but not
  its `#anchor` (`body.anchors`).
- `set`/`new` do not refuse a cycle on an `acyclic` field (see `refs.acyclic`).
