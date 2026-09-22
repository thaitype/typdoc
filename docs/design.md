# typdoc — Schema-Driven Markdown Documents: Design

2026-09-19 · @Someone

## Overview

`typdoc` is a CLI that treats a folder of Markdown files as typed, linked documents: frontmatter is validated against schema files (a JSON format of typdoc's own, not JSON Schema), and references between documents, in frontmatter or body links, are resolved, queried and checked. Tickets are one kind of document; notes, learnings and precedents are others. Every command has `--json` output and meaningful exit codes, because the main users are agents.

**Philosophy**

typdoc adds no syntax. Everything it understands is already Markdown: YAML frontmatter, standard links, headings. What typdoc adds is the ability to check that those things are correct: that a field has the right type, that a link points at a file that exists, that a reference lands on the kind of document it should.

A typdoc document is therefore just a Markdown file. Someone who has never heard of typdoc can open it in GitHub, an editor or a terminal, read it, follow its links, and edit it without breaking anything they can see. Nothing in the file needs typdoc to make sense.

typdoc is a lens over the files, not a format for them. Adopting it means adding a config and schemas beside existing documents, not rewriting them; `validate --audit` shows what to fix first. Removing it leaves every file exactly as usable as before.

**Principles**

- **Files stay plain Markdown.** Frontmatter is ordinary YAML whose values are ordinary strings, numbers and lists. A link in the body is a standard Markdown link to a real path. Text that merely mentions something ("see WF-3") is information by default. A namespace can opt in to checking that mentioned keys exist (the body.mentions rule); mentions still never become refs.
- **Generic.** `typdoc` knows documents, fields, headings and refs. Workflow meaning ("claim", "frontier", "precedent") lives in the skill or workflow that calls it.
- **Schema-driven.** Types, enums, state transitions and ref targets come from schema files, so one tool serves many document structures. The format is typdoc's own JSON, not JSON Schema, which has no notion of refs between documents, state transitions or auto fields.
- **Frontmatter only.** Writes touch the frontmatter block and never re-serialize the body. The one exception is `mv`, which rewrites link paths.
- **Safe under concurrency.** Number allocation and conditional updates happen under a lock.

**Out of scope for v1:** editing body sections, SQL queries, saved query aliases, multi-hop ref traversal, and following imports of imports.

## Model

A document is identified by a key if its schema has a `code`, and by its path otherwise; both kinds can reference each other.

| Concept | Definition | Example |
| --- | --- | --- |
| Project | A folder containing `.typdoc/config.json`; what `imports` points at. Collections and schemas belong to the project. A file belongs to the nearest project above it. | `.chief/`, `typmem/memory/` |
| Namespace | A folder that owns documents and numbers its own keys. A project is one namespace named `default`, or several when `namespaces` is set: each matching child folder is one, named by its folder. Keys are unique within a namespace. | `default`, or `story-3` at `.chief/story-3/` |
| Collection | Files matching one pattern in each namespace, sharing one schema; defined by one file in `.typdoc/collections/`, so its name is unique within a project | `learnings/*.md` |
| Schema | JSON definition of fields and rules, optionally with a `code` | `wayfinder.json` (`code: WF`) |
| Document | One `.md` file | `learnings/never-send-secrets-over-ship.md` |
| Key | `{code}-{number}`, the identity of a document whose schema has a code | `WF-3` |
| Path | The identity of a document whose schema has no code | `precedents/secret-handling.md` |
| Ref | A pointer to another document, from a frontmatter field or a body link | `blocked_by: [WF-1]` |

Before any query or validation, `typdoc` builds one index mapping every key, within its namespace, and every path to its file. Refs of either kind resolve through it, so a keyed ticket can point at a path-identified note and the reverse. Paths are compared exactly as they are written, case included, on every platform: a ref is resolved through the index of names as they are on disk, so a path that differs from the file's name in case does not resolve, even where the file system would open it. Two keys cannot differ only in case, because a code is capital letters and digits (see `code`), so `keys.unique` has nothing to do about case.

**Collection vs schema.** A collection selects files; a schema describes their shape. One schema without a `code` may serve several collections (`notes` and `drafts` both using `note.json`), so anything about choosing documents uses the collection: `list --collection`, the `collection` pseudo-field, a collection's `validation`. Anything about data shape uses the schema: types, fields, `extends`, and a ref field's `target`. `target` names schemas rather than collections because a schema, possibly published remotely, cannot know what a given namespace calls its collections.

## Config: .typdoc/config.json

Each project has one `.typdoc/config.json` that sets project-wide options, optionally declares several namespaces and optionally imports other projects, plus one file per collection in `.typdoc/collections/` that maps files to a schema. The smallest valid config is `{ "version": 1 }`.

```json
{
  "version": 1,
  "namespaces": "story-*",
  "imports": { "memory": "${TYPMEM_DIR}/memory" },
  "validation": {
    "global": { "body.links": { "level": "error", "ignore": ["assets/**"] } }
  },
  "lock": "local"
}
```

| Key | Required | Meaning |
| --- | --- | --- |
| `version` | yes | Integer version of every file format typdoc owns: this file, the collection files, `lock.json` and the schema format. Currently `1`. An unknown version is a config error; `typdoc` never guesses. Pinned copies of remote schemas follow the publisher's version in their URL. |
| `namespaces` | no | The child folders that are namespaces: a name, a glob or an array of them. Absent: the whole project is one namespace, `default`. See Namespaces. |
| `imports` | no | Alias → folder of another project. The alias is chosen here; the imported project has no name of its own. Paths may use environment variables. See Refs → Across namespaces. |
| `validation` | no | Rule levels and options that apply project-wide, under `global`. A collection tunes them in its own file. See Validation rules. |
| `lock` | no | `local` (default) or `git-common`. See Concurrency. |

**Namespaces.** Without `namespaces`, the project is one namespace named `default`, and `match` counts from the folder that holds `.typdoc`. With it, each entry is a name or a glob (`*` only) matched against the folders directly inside that folder, and every match is one namespace, named by its folder, with `match` counting from that folder. A glob that reaches a folder that is a symbolic link skips it, and the run reports it under `files.unreadable`, the same as a `match` that reaches a link. An entry that names a symbolic link in plain text is a config error (`config.namespaces-entry`) that says to name the folder it points to. A link is a second name for a folder that is already there, so following it would count one set of documents twice under two paths, which breaks the accounting and the rule that a path names one document; a folder whose name begins with `.` is a folder of its own that no other name reaches, which is why it is entered when it is named and a link is not.

- An entry is one path segment. `/` and `**` are config errors, because a namespace name has to show where it ends in a path reference such as `chief::story-3/_tickets/WF-5.md`. Anything deeper is grouped by placing `.typdoc` deeper.
- A name uses only ASCII letters, digits, `-` and `_`. `*` never matches a folder starting with `.`. A matched folder with any other name is a config error that names the folder; it is never skipped.
- `default` is reserved: a folder may not use it.
- An entry without a glob must exist. A matched folder that holds its own `.typdoc` is a config error: namespaces do not nest.
- Collections, schemas and rule levels are shared by every namespace of a project. A code may be used in every namespace; each namespace numbers its own keys, and a key is unique within its namespace, not across them.
- A ref or mention with no prefix means the namespace of the document that holds it, whatever the working directory. Prefixes are described under Refs.
- Files outside every namespace folder belong to no namespace; a relative path can still point at them. A ref or a body link that reaches such a file resolves like any other: the file exists, so the answer is never `not-found`, and the command goes on. The file is named by its path alone: `refs` and a `ref.*` or `refby.*` condition treat it as a document with no namespace, and `--json` leaves the `namespace` field out, the way it leaves out `key` for a document that has none.
- Any other key in `config.json`, `name` included, is an unknown key and a config error.

**Collection files.** Each collection is one file, `.typdoc/collections/<name>.json`. The file name without `.json` is the collection's name: ASCII letters, digits, `-` and `_`, so it is unique by construction. The file maps files to a schema. It is configuration only: numbering state lives in `.typdoc/state/`.

```json
// .typdoc/collections/wayfinder.json
{
  "match": "tickets/{key}.md",
  "schema": "schemas/wayfinder.json",
  "validation": { "body.mentions": { "level": "error" } }
}
```

| Key | Required | Meaning |
| --- | --- | --- |
| `match` | yes | Which files belong to the collection. See Match templates. |
| `schema` | yes | Relative path or `http://` or `https://` URL of the schema. See Remote schemas. |
| `refBase` | no | How frontmatter paths resolve: `file` (default, relative to the document) or `namespace` (relative to the namespace folder) |
| `validation` | no | Rule levels and options for this collection only, merged over `validation.global`. See Validation rules. |

**Loading.** Every `*.json` file in `.typdoc/collections/` is a collection; other files are ignored. A file that cannot be parsed, has an unknown key or names a schema that does not exist is a config error that names the file, and `typdoc` stops rather than skip it, because a skipped collection would silently shrink every result. Collections have no order; anything that lists them sorts by name. A document matched by two collections is an error (`collections.overlap`), never settled by precedence; the overlap is checked in each namespace. This is intended and not a limit waiting to be lifted. A rule that named a winner, the more specific match or the first one, would let someone who reads two collection files fail to say which one wins until they knew a rule that neither file states, and it would decide for them without saying so. `--audit` shows every collection, one that ends with no document included, and names the collections of each overlap, so the overlap is seen from both sides. Numbering state is not kept here: a `last` key in a collection file is an unknown key and a config error.

**State.** `.typdoc/state/<namespace>.json` holds, per collection, the highest number ever issued in that namespace — by `typdoc new`, and by `mv --renumber` in the namespace it moves a document into. It is the highest number issued, not the highest number that exists. The two differ whenever a document is deleted, or moved out by `mv --renumber`, and the difference is ordinary and is left alone. `last` is never lowered to match what is on disk, not even when the document that held the highest number has gone: lowering it makes `typdoc new` issue that number a second time, and a ref written `story-1:WF-9` before the move would then resolve to a different document, with nothing to report because both the ref and the document are well formed. The gap is what keeps a retired key retired.

```json
// .typdoc/state/story-3.json
{ "wayfinder": { "last": 7 } }
```

When typdoc creates the file, or adds an entry to one, it writes JSON with the keys in alphabetical order, two spaces of indentation, `\n` line endings and a final newline. The file is committed, so it is read as a diff and merged, and a form that does not move means a conflict says that two people disagree about a number rather than that the formatting changed under them. Updating an entry that is already there replaces only the number, in place, as described below, and so leaves the rest of the file exactly as it was found. It is written by `typdoc new` and by `mv --renumber`, which are the commands that issue numbers, and not by hand except as described next; it is what stops a number being reused after its document is deleted. `state/` is committed (see the layout above). A collection that has coded documents in a namespace and no `last` recorded there, because the file or the entry is missing, is the reverse of an orphan: either the record was lost, or the documents were made before typdoc was used. The always-on rule `state.missing` reports it, and `typdoc new` and `mv --renumber` refuse to issue a number for that collection in that namespace, because the highest existing number may be lower than a number that was issued and later deleted. To go on, restore the file from version control, or create it with `last` set to the highest existing number, or to a higher number if one was ever used and deleted. A collection with no coded documents in the namespace and no record is new, and nothing is reported: the first `typdoc new` creates the file or the entry. A record can be lost without a trace only when every coded document of the collection in that namespace has also been deleted, since nothing is left to compare it with. The namespace `default` uses `state/default.json`, so nothing in the config can change which file holds the numbers. `typdoc new` replaces only the number, in place, then re-parses the result and compares it with what it meant to write before renaming the temp file over the original, so it cannot corrupt the file. A file in `state/` that matches no current namespace is reported as the finding `config.state-orphan`, which names it and stops nothing: delete it after removing a namespace, or rename it after renaming a folder or after moving from one namespace to several (`default.json` becomes `<folder>.json`). Worktrees that work on different namespaces never change the same state file; Concurrency covers two on the same one.

**Match templates.** For a coded schema, `match` is a template with placeholders; for a schema without a code, it is a glob.

| Placeholder | Stands for | Allowed in |
| --- | --- | --- |
| `{key}` | `{code}-{number}`, with the code taken from the schema, e.g. `WF-3` | Coded schemas; exactly once, no globs |
| `*`, `**` | Glob | Schemas without a code; no placeholders |

One template serves both directions: it decides which files belong to the collection, and `typdoc new` uses it to name new files. Because `{key}` includes the schema's code, several coded collections can share one template in one folder: `tickets/{key}.md` matches `WF-3.md` for one schema and `RFC-4.md` for another. Each code counts on one number sequence of its own in each namespace, so a coded schema serves exactly one collection; two collections naming the same coded schema is a config error, as is a state entry for a collection whose schema has no code.

**Which files a run reads.** A glob does not enter a folder whose name begins with `.`, which is the rule namespaces already follow, so a collection and a namespace answer the question the same way. A literal segment does enter one: a project that keeps its documents under `.agents/` names that folder in `match` and gets them, because naming a folder is saying it is wanted, while a glob is saying "whatever is here". A `*` does match a leading dot in a file name, since a file is named by the template that reaches it rather than found by walking into it. A symbolic link to a folder is not followed, so a run cannot leave the project or read one file twice under two names. `.gitignore` is not read: what a version control system hides is a different question from what a project declares, and a file that no `match` reaches is already outside every collection. A directory entry a template reaches that is a symbolic link, or whose name is not valid UTF-8, is skipped and reported under `files.unreadable` rather than stopping the run: one name that cannot be read should not deny an answer about every other file beside it.

**Discovery.** `typdoc` finds its project by walking up to the nearest folder containing `.typdoc/config.json`, starting from the first of these that applies: a document path given as an argument that is absolute or begins with `./` or `../`, read from the disk as the file it names; the folder `TYPDOC_DIR` names, which skips the walk, for an agent that runs from a repository or worktree root above the project; the current directory. Any other path argument is relative to the project folder. It cannot say which project it is in, so it takes no part in this choice and is read after the project is found. There is no `--dir` flag. Config, collection files and local schemas are read on every run with no cache; remote schemas are read from their pinned copies (see Remote schemas).

**Arguments that name a document.** An argument that names a document is a path or a key, told apart by its form and never guessed. After any `project::` prefix, an argument that ends in `.md` is a path, and one that has the form of a key is a key. A key never ends in `.md` and a document is always a `.md` file, so the two cannot be confused. Anything else is bad arguments (exit 1). A path that begins with `/`, `./` or `../` is a path on disk, absolute or relative to the current directory. Any other path is relative to the project folder, the folder that holds `.typdoc`, which is what `path` is in `--json`. The path of a document of an imported project is written `project::path`, relative to that project's folder. `mv` reads both its arguments in this way, except that neither may carry a `project::` prefix: `mv` writes only in the project it is run in, so an argument naming a document of another project is bad arguments (exit 1). Its second names a file that does not exist yet: a `mv` whose destination is already there writes nothing and exits 7, and so does a `--renumber` whose destination name is taken. When a path relative to the project names nothing in it but a file of that name exists relative to the current directory, the error is exit 5 and says that `./name` exists. That is a suggestion; nothing is done in its place.

The string that names a document in an argument follows from the name it is printed with (see JSON output):

| The document | As a path | As a key (a coded document only) |
| --- | --- | --- |
| In this project | `path` | `key` when the project has one namespace, `namespace:key` when it has several |
| In an imported project | `project::path` | `project::key` when that project has one namespace, `project::namespace:key` when it has several |

The path form works for every document and needs to know nothing about how many namespaces a project has, so it is the form for a program to pass on. A name that a command prints is accepted by every command that takes a key or a path, and a test walks every document of every project in the fixtures to check it.

**Choosing a namespace.** In a project with more than one namespace, a command takes its scope from the first of these that applies:

1. A prefix on a key or path argument (`story-2:WF-5`).
2. `--namespace <list>`: names separated by `,`, or globs (`*` only). `'*'` means every namespace of this project; imported projects are not included and are named explicitly, as in `'chief::*'`.
3. `TYPDOC_NAMESPACE`, with the same syntax as `--namespace`.
4. The current directory, when it is inside a namespace folder or below one.
5. Otherwise there is no scope: reads span every namespace of the project and writes are an error.

A key that exists in more than one namespace in scope, and a write that could land in more than one, exit 1 with every choice listed and, with `--json`, a `candidates` array; typdoc never picks. From inside a namespace, reading another needs a prefix or `--namespace`. A ref written in a file always means the namespace of that file, whatever the working directory. A namespace of an imported project can be named only for reading. Examples always quote `'*'`, because an unquoted `*` is expanded by the shell.

**Nested projects.** Namespaces of one project never nest (see Namespaces). A folder with its own `.typdoc/config.json` deeper in the tree is a separate project: a collection's `match` never crosses into it, and a file there always belongs to the nearer project.

**The .typdoc folder.** Everything typdoc reads as configuration or writes for itself lives in one folder at the top of the project; the folder is also what marks a project.

```
.typdoc/
  config.json    config (this section)            commit
  collections/   one file per collection          commit
  state/         one file per namespace: numbers  commit
  lock.json      pins for remote content          commit
  vendor/        pinned copies, e.g. schemas/     commit
  locks/         one mutex per namespace, one per project  .gitignore
```

`lock.json` has no version of its own (the `version` in `config.json` covers every file typdoc owns) and is split into sections so pins other than schemas can join later without a rename:

```json
{
  "schemas": {
    "https://schemas.example.dev/chief/wayfinder/v1.json": {
      "sha256": "9f2c…", "fetchedAt": "2026-09-19T14:30:00+07:00"
    }
  }
}
```

A pinned copy lives at `vendor/<section>/<sha256>`, with no extension: `vendor/schemas/9f2c…` for the entry above. The path is never stored; it follows from the section name and the hash, so a second kind of pin needs no change to the rule. A copy counts as edited by hand when the hash of its contents differs from its file name. That lets a file in `vendor/` check itself without opening `lock.json`, and leaves one value where two could disagree. Files here are written like every other file (temp file, then rename), so a command that reads `lock.json` or a copy while `pull` writes it sees the old file or the new one whole. v1 never deletes anything in `vendor/`: it only grows, on purpose. Clearing copies nothing refers to is a separate job, and a large `vendor/` is not a bug to fix. A pinned copy that is missing is a config error that says to run `typdoc pull`, which fetches again and compares with the pinned SHA-256: equal restores the copy; different is reported as a changed URL, an update of the pin, as `pull` always reports.

There is one config location, `.typdoc/config.json`. The error for a folder with no project names that file as the one looked for, so a person who looked for the config under any other name is told where it is, whatever the name was; no other name is checked for.

**Machine-specific imports.** When an imported project's location differs per machine, put it in an environment variable or in a machine file, `imports.json`, which is merged under the project's own `imports`, so no machine-specific path is committed. The file is found in this order, stopping at the first step that applies:

1. `TYPDOC_CONFIG_DIR`, if set: the file is `$TYPDOC_CONFIG_DIR/imports.json`. The value must be an absolute path to a directory that exists; anything else is an error, since it was set on purpose.
2. `XDG_CONFIG_HOME`, if set to an absolute path: the file is `$XDG_CONFIG_HOME/typdoc/imports.json`. Unset, empty or relative counts as not set, as the XDG specification says.
3. The platform default. In v1, on Linux and macOS: `~/.config/typdoc/imports.json`.

Support for another platform is one new line at step 3; steps 1 and 2 do not change. `TYPDOC_CONFIG_DIR` also lets tests and containers with an odd `HOME` move the file without borrowing the system's variable. If the file does not exist there are no machine-specific imports, and a ref into an import that this leaves absent is reported by `imports.absent` (a warning by default); there is no second mechanism. An error about the file names the path that was searched and which of the three steps it came from. No test reads the real home directory of whoever runs it, and each step is exercised with a fake `HOME` or environment.

**Environment variables in import paths.** `${NAME}` in an import path is replaced by the variable's value; one rule serves the `imports` in `config.json` and the ones in `imports.json`. A variable that is unset, or set to an empty value, is never replaced by an empty string: that would turn `${HOME}/projects` into `/projects`, a path that may exist and be the wrong project. The import is instead treated as absent on this machine and reported by `imports.absent`, with a message that names the variable (`TYPMEM_DIR is not set`), which is different from a path that does not exist. Other imports and namespaces load normally. Imports that differ per machine exist so that no machine-specific path is committed; if one missing import stopped the whole project from loading, the easy way out would be to commit the path, which is what this is meant to prevent. So a project that needs its imports to be there should set `imports.absent` to `error` in CI: a mistyped variable name is otherwise only a warning.

## Schema format

A schema is a JSON file with a name, an optional code, an optional parent, and its fields. The format is typdoc's own; it is not JSON Schema, and no JSON Schema tool reads it.

**Top-level keys**

| Key | Required | Meaning |
| --- | --- | --- |
| `name` | yes | Schema name, unique within its project |
| `code` | no | `[A-Z][A-Z0-9]*`. Present: documents get keys and `typdoc new` allocates numbers. Absent: documents are identified by path. |
| `extends` | no | Parent schema: a relative path or an `http://` or `https://` URL. Chains allowed; cycles rejected. A parent is usually abstract (no `code`, not used by any collection). |
| `fields` | yes | Map of field name to definition |

**Field types:** `string`, `number`, `bool`, `date` (ISO `YYYY-MM-DD`), `datetime` (ISO 8601 with offset, e.g. `2026-09-19T14:30:00+07:00`), `enum`, `list` (array of strings), `ref`, `ref[]`.

**Field options**

| Option | Applies to | Meaning |
| --- | --- | --- |
| `required` | all | Must have a value |
| `default` | all | Filled in by `typdoc new` |
| `values` | `enum` | Allowed values, in order; the order is also the sort order |
| `transitions` | `enum` | Map of value → allowed next values. Omitted: any change allowed. |
| `target` | `ref`, `ref[]` | `"*"` (default: any file) or a list of schema names |
| `acyclic` | `ref`, `ref[]` | Reject any cycle formed through this field |
| `auto` | `date`, `datetime`, `list` | `create`: set by `new`, never changed after. `update`: set by `new` and by every `set` that changes a value. `moves` (`list` only): `mv` appends the document's previous key or path on every move, always with its prefix (`story-2:WF-5`, `notes/old-name.md`), `--renumber` included. |
| `override` | all | Required to redefine an inherited field |

**Auto fields.** Field names carry no meaning; only `auto` does. A field such as `created_at` without `auto` is an ordinary field that `typdoc` never fills. `set` may not write an `auto` field directly. Because edits made outside `typdoc` (by hand, or a file tool changing the body) are invisible to it, an `auto: update` field means "frontmatter last changed through `typdoc`", not "file last modified". A `list` field with `auto: moves` is opt-in per schema and holds plain strings, never refs, since they name documents that no longer exist under that name; the `refs.moved` rule reads it. Without such a field nothing is recorded.

**Target names.** A bare name (`"learning"`) means a schema in this project. A qualified name (`"memory::learning"`) means a schema in the imported project `memory`. Qualified names live only in schema JSON, never in Markdown files. `"*"` also accepts files outside any collection, such as a README.

**Extends rules.** New fields merge in. Redefining an inherited field without `"override": true` is a schema error, so a child cannot silently change what a shared field means. Sibling schemas may define same-named fields independently.

**Remote schemas.** Anywhere a schema is referenced, in a collection's `schema` or in `extends`, an `https://` or `http://` URL to a JSON file works as well as a path. A workflow can publish its schemas so users need not write them:

```json
// .typdoc/collections/wayfinder.json
{ "match": "tickets/{key}.md",
  "schema": "https://schemas.example.dev/chief/wayfinder/v1.json" }
```

```json
// schemas/wayfinder.json: adopt the published schema, add one local field
{
  "name": "wayfinder",
  "code": "WF",
  "extends": "https://schemas.example.dev/chief/wayfinder/v1.json",
  "fields": { "team": { "type": "string" } }
}
```

- **Pinned, not live.** The first time a URL is needed, `typdoc` fetches it, stores a copy under `.typdoc/vendor/schemas/<sha256>`, and records its SHA-256 in `.typdoc/lock.json`. Every later run reads the stored copy, so results never change because a server did, and runs work offline. Commit both so CI and teammates use the same bytes. `typdoc pull` re-fetches on purpose.
- **Relative references** inside a remote schema, such as its own `extends`, resolve against its URL.
- **The connection is the user's choice.** typdoc does not restrict it: `https://` and `http://` both work, and whether to use https is for the user to decide. A URL with any other scheme is a config error. A schema is JSON data; nothing in it is executed. The pin records the SHA-256 of the bytes received, so a later change is reported, but it cannot say that the first fetch was the document the publisher meant.
- **Proxies.** The proxy named by `HTTPS_PROXY` or `HTTP_PROXY`, in either case, is used for every request whichever scheme it has, and an exact host in `NO_PROXY` skips it. Nothing more is claimed: precedence when several are set, `ALL_PROXY`, patterns in `NO_PROXY` and proxy authentication were not tried.
- **Names.** Only schemas used directly by a collection share the project's names, since those are the names `target` refers to. A parent reached only through `extends` does not, so a local schema may reuse its parent's name, as above. Two collection schemas with one name are reported by `schema.valid`.
- **Versioning** is the publisher's job: put the version in the URL (`.../v1.json`), so a breaking change is a new URL that users adopt deliberately.

**Targets in published schemas.** An import alias belongs to the user's project, so a published schema should not name one in `target` (`"memory::precedent"`). Publish `"*"` or bare schema names, and let users narrow `target` in a local schema that extends the published one, using `"override": true`.

**Example**

```json
// base-ticket.json (abstract)
{
  "name": "base-ticket",
  "fields": {
    "title":  { "type": "string", "required": true },
    "status": {
      "type": "enum",
      "values": ["open", "claimed", "resolved", "closed"],
      "default": "open",
      "transitions": { "open": ["claimed", "closed"], "claimed": ["resolved", "open", "closed"] }
    },
    "blocked_by": { "type": "ref[]", "target": "*", "acyclic": true, "default": [] },
    "created_at": { "type": "datetime", "auto": "create" },
    "updated_at": { "type": "datetime", "auto": "update" }
  }
}
```

```json
// wayfinder.json
{
  "name": "wayfinder",
  "code": "WF",
  "extends": "./base-ticket.json",
  "fields": {
    "kind":    { "type": "enum", "values": ["research", "prototype", "grilling", "task", "feature"], "required": true },
    "owner":   { "type": "string" },
    "context": { "type": "ref[]", "target": ["memory::precedent", "memory::learning"] }
  }
}
```

```json
// precedent.json (in the memory namespace; no code, identified by path)
{
  "name": "precedent",
  "fields": {
    "author":   { "type": "string", "required": true },
    "created":  { "type": "date",   "required": true },
    "sources":  { "type": "ref[]",  "target": ["learning"], "required": true },
    "proposal": { "type": "ref",    "target": ["proposal"] }
  }
}
```

## Document files

A document is YAML frontmatter plus a free Markdown body; `typdoc` owns only the frontmatter. A file has frontmatter when it begins with a `---` line that opens a block. A block that is present but empty counts: it is a document that declares itself and has no fields yet, and it is checked like any other, so every required field it lacks is a finding. A file with no block at all is an ordinary Markdown file. The two are not merged, because a document missing every required field would otherwise be filed with the files that are not typdoc's, with no signal. A block that is present but cannot be parsed is not an absent block: it is the finding `frontmatter.parse`, so a damaged document is never taken for an ordinary Markdown file. The finding has a position when the YAML reader gives one, and the reader does not give one for every error.

```markdown
---
title: Cosmos or SQL?
kind: grilling
status: open
blocked_by: [WF-1]
context: [memory::precedents/secret-handling.md]
created_at: 2026-09-19T14:30:00+07:00
updated_at: 2026-09-19T16:05:12+07:00
---

## Question

See [the secret-handling precedent](../../typmem/memory/precedents/secret-handling.md) first.

## Answer
```

**Filenames**

| Schema | `match` | Example file |
| --- | --- | --- |
| With `code` | `tickets/{key}.md` | `tickets/WF-3.md` |
| Without `code` | `learnings/*.md` | `learnings/never-send-secrets-over-ship.md` |

A coded document's file name is its key and nothing else, so it never changes when the title does and a body link to it never breaks. Keys resolve to files, and two files with the same key are a validation error. A document without a code is named by whoever creates it (`typdoc new <path>`); typdoc only checks the path against `match`. typdoc never derives a file name from a title: a title can slug to nothing (`😀`), and a long Thai title can exceed the file-name limit.

**Rules**

- The key is not stored in frontmatter; the filename is the single source of truth.
- `title` is an ordinary frontmatter field, not the H1. The body has no required structure.
- Writes never move an existing key and never reformat the body. A key that is added goes at the end of the frontmatter block.
- **A write rewrites the whole frontmatter block, not the line it changed.** No value changes when it does: every value is carried across as the text it was written with, and quoted where YAML would otherwise read it as something else. `1e3`, `1.10`, `0755`, an integer longer than 64 bits, `no`, `~`, a date, an empty string, and text holding a newline, a tab or a leading space all come back exactly as they were. What the block was written *with*, rather than what it says, is kept as a best effort and is not a promise. These are lost on any write, including one that changes a single field:

| Written in the file | After any write |
| --- | --- |
| Comments, anywhere in the block | Gone |
| Blank lines between fields | Gone |
| `tags: [a, b]` | A block list, one item per line. An empty list is written `[]` |
| `title: 'Ship it'`, `status: "no"` | Unquoted, unless the text needs quotes to survive |
| `id:   WF-3` | One space after the colon |
| `&anchor` with `*alias` | The value written out in full at every place that used it |
| `!!str`, `!Ref`, any other tag | Gone; the value stays |

  The last two change what the file means, not only how it looks. An alias is one value used in several places; after a write it is several values that no longer follow each other, and nothing afterwards reports it. A tag may be what another tool in the user's chain reads. A document that depends on either should be edited by hand, not by typdoc.

- Frontmatter fields not in the schema are kept on write and reported by `validate`.

## Refs

Refs come from two places, frontmatter fields and body links, and both resolve through the same index.

**Frontmatter values.** A `ref` or `ref[]` value is a plain string, read in this order:

| Value | Read as | Condition |
| --- | --- | --- |
| `WF-3` | Key in the document's own namespace | Matches `^[A-Z][A-Z0-9]*-\d+$` and the code exists in this project |
| `story-2:WF-5` | Key in the sibling namespace `story-2` | `story-2` is a namespace of this project |
| `story-2:notes/x.md` | Path inside the sibling namespace `story-2`, from its folder | Same |
| `memory::precedents/x.md` | Path inside the imported project `memory`, which has one namespace | `memory` is an alias in `imports` |
| `chief::story-3:WF-5` | Key in namespace `story-3` of the imported project `chief` | `chief` is an alias in `imports` |
| anything else | Relative path | Resolved from the document (`refBase: file`) or the namespace folder (`refBase: namespace`) |

`name:` reaches a sibling namespace and `name::` an import, and the two never fall back to each other: a name that does not exist on the side the syntax names is an error, never a relative path. A path that really contains a colon is written with a leading `./`. A ref into a project with several namespaces must name one (`chief::story-3:WF-5`); `chief::WF-5` is an error there, since only a one-namespace project has a `default`. A name may be both a sibling and an import; the `names.shadowed` rule warns. Sibling names and import aliases may not collide with URL schemes (`http`, `https`, `mailto`, `file`); `validate` enforces this, as `config.namespace-name` for a namespace and as `schema.valid` for an import alias.

**Body links.** Standard Markdown links count, in every form the parser reads as a link: inline `[text](path)` and `[text](path#heading)`, images `![alt](path)`, and reference-style `[text][ref]`, `[ref][]` and `[ref]` with a definition `[ref]: path`. Autolinks (`<https://…>`) are URL-scheme links and are skipped. Paths are relative to the document, percent-decoded, and may be written `<my file.md>`. Links starting with a URL scheme (`https:`, `mailto:` and so on) that is not a namespace name or an import alias are always skipped; no configuration is needed. A prefixed link is written `name:path` or `name::path`; after the prefix comes a path, never a key. Links inside fenced code blocks and inline code are not links. Relative paths that should not be checked (images, generated files) go in the `ignore` option of the `body.links` rule. Plain-text mentions are never refs; the `body.mentions` rule can check that they exist (see Validation rules). Body links are exposed to queries as the virtual ref field `$body`.

**Reference definitions.** Every definition line is checked once, at the definition, whether or not anything uses it, with the number of places that use it (`link target missing: notes/x.md (used 3 times)`); the uses are not reported separately. Labels are compared as CommonMark does: case-folded, whitespace collapsed. When a label is defined twice, CommonMark ignores the later definition; `body.links` reports it (`already defined at line N; this definition is ignored`) even when both point at the same file, and does not check its target. A definition inside a code block is not a definition. A definition that nothing uses is checked but does not appear in `$body`, so `refby` never counts a document that does not actually link to another. `mv` rewrites the active definition and leaves an ignored one alone.

**Text that looks like a link but is not.** Under `body.links`, a `[text](inner)` or `![text](inner)` outside code that the parser does not read as a link, and a line `[label]: inner` that it does not read as a definition, is reported when `inner` has no URL scheme (a namespace name or import alias does not count as one) and, after removing a trailing title (`"…"`, `'…'` or `(…)`), ends with a file extension, optionally followed by `#anchor`. An extension is a `.` followed by one to eight ASCII letters or digits, at least one of them a letter. The usual cause is an unescaped space, which CommonMark does not allow in a bare destination; the message says to write `<my file.md>` or `my%20file.md`. Without this check such a link would be invisible: the reader sees a link and the checker sees text. The finding is at the `[` (or the `!`), or at the definition line; for a rejected definition it cannot say how many places use it. A `[text][ref]` with no definition is not reported: CommonMark reads it as plain text, and the shape is too common in ordinary prose (`a[0][1]`).

**Heading anchors.** The `slug` of a heading follows GitHub's algorithm, so a link that passes `validate` also works on GitHub. Take the heading's plain text (text and code spans; image alt text, line breaks and inline HTML contribute nothing), lowercase it, delete punctuation other than `-` and `_`, symbols and other characters that are not letters or digits, and turn each space into `-`. Marks count as part of a letter, so Thai vowels and tone marks stay; non-ASCII text is kept as written, never transliterated. A slug that repeats an earlier one in the same document gets `-1`, `-2` and so on, skipping any result already taken (`Dup`, `Dup`, `Dup 1` give `dup`, `dup-1`, `dup-1-1`). Every heading counts, including those inside block quotes and list items but not those inside fenced code, so the numbering matches GitHub's. A heading whose slug is empty (`## !!!`, `## 😀`) is not special: the first gets `""` and cannot be linked to, the next `-1`, then `-2`. In a link, the fragment is percent-decoded (a `%` not followed by two hex digits is kept as written) and then compared with the slug without regard to case. Slugs are used only for anchors, never for file names. The exact character classes are pinned by the contract's fixtures, generated from GitHub's renderer.

**Canonical form.** A coded document should be referenced by key in frontmatter; referencing it by path works but `validate` warns, since a path changes when the file is moved and a key does not. Body links always use paths.

**Write-time checks** (`new`, `set`): the target exists; its schema is allowed by `target`; no cycle forms on `acyclic` fields.

**Across namespaces.** A relative path that leaves the namespace, a sibling prefix or an import prefix lands in another namespace or project. `typdoc` finds that file's nearest `.typdoc/config.json` to learn its schema; namespaces of one project share collections and schemas, so a sibling is checked exactly like the document's own namespace. Forward traversal needs no configuration. Reverse lookup (`refby`, `refs --reverse`, `mv`) scans every namespace of this project and the projects it imports. Imports are one-way: `chief` importing `memory` does not let `memory` see `chief`. An imported project is read-only here, with no exception: no command writes a file in it, `mv` included. A `mv` reads the refs of an imported project so that it can report the ones that will be left pointing at the old path, and it changes none of them. Because it writes nothing there, it takes no lock there either: a lock belongs to the project that owns the file, and typdoc never takes one in a project it does not write to.

|  | Without imports | With imports |
| --- | --- | --- |
| Forward refs and validation across namespaces and projects | yes | yes |
| `refby` sees refs from | every namespace of this project | also the imported projects |
| `mv` rewrites refs in | every namespace of this project, preserving each ref's written form | nothing: an imported project is read-only, and its refs are reported instead |

**Ownership rules** (so two namespaces never conflict):

1. A file is validated only by the namespace that owns it. A ref from another namespace or project checks only that the target exists, matches `target`, and has the linked heading.
2. `match` stops at a nested project's boundary; namespaces of one project never nest.
3. A bare schema name in `target` always means this project.
4. Imports are followed one level; imports of imports are ignored.
5. Writes stay inside the namespace the scope names, except `mv`, which also rewrites refs in the other namespaces of the same project. No write ever leaves the project.

**Schema drift.** A schema names a schema of an imported project only through a qualified name in `target` (`"memory::learning"`). If the imported project renames or removes that schema, the `target` no longer names anything, and `validate` reports it under `schema.valid`, at the schema file of this project and naming the ref field. An import that is absent on this machine is reported by `imports.absent` instead and is not an error here. A field that an imported project renames is not checked ahead of time, because nothing in a project's files records which fields of another schema it relies on; a query that names a field no schema in scope defines is an error and not an empty result (see Names and scope), so such a change is loud when the query runs. Run `validate --schemas` in CI when projects live in different repos.

## Query

Each `--where` is evaluated while standing on one candidate document ("me"): a plain condition reads my own fields, a ref condition follows arrows to other documents and tests them.

```
ref.all(blocked_by).status=resolved
 │   │      │          └─ condition tested on each document reached (optional)
 │   │      └─ ref field holding the arrows
 │   └─ quantifier: all | any | none
 └─ direction: ref (arrows leaving me) | refby (arrows pointing at me)
```

Arrows are stored only on the document holding the field. `refby` finds incoming arrows through a reverse index built from the frontmatter already loaded, the same index `refs --reverse` uses; nothing is stored twice.

**Grammar**

```
expr     = ref-expr | plain
ref-expr = dir "." quant "(" f ")" [ "." plain ]   ; "all" requires the "." plain part
dir      = "ref" | "refby"
quant    = "all" | "any" | "none"
f        = field | "$body"                         ; a ref or ref[] field, or $body
plain    = field op value
field    = [A-Za-z_][A-Za-z0-9_-]*                 ; or a pseudo-field
op       = "!=" | "<=" | ">=" | "=" | "<" | ">"    ; longest match at the first operator after the field
value    = item { "," item }                       ; one comparison value only for < <= > >=
item     = { char | "*" | "\" ( "," | "*" | "\" ) }  ; "\" before anything else is an error
```

An unescaped `*` is a glob; alone, in `k=*`, it means "present". In `--set` the same rules hold except that `*` must be escaped and `,` splits only array fields.

**Expressions**

| Expression | Meaning | Example |
| --- | --- | --- |
| `k=v` | Equals | `status=open` |
| `k!=v` | Not equals | `status!=resolved` |
| `k=a,b` | Equals any listed value | `status=open,claimed` |
| `k=pre*` | Glob match | `title=Cosmos*` |
| `k=*` / `k!=*` | Field present / absent or empty | `owner!=*` |
| `k=v` on an array | Array contains `v` | `blocked_by=WF-3` |
| `k<v`, `k<=v` | Less than, less than or equal | `updated_at<2026-09-01` |
| `k>v`, `k>=v` | Greater than, greater than or equal | `estimate>=3` |
| `ref.all(f).EXPR` | Every document in my `f` matches; true when empty | `ref.all(blocked_by).status=resolved` |
| `ref.any(f)[.EXPR]` | At least one document in my `f` matches; false when empty | `ref.any(blocked_by).updated_at<2026-08-01` |
| `ref.none(f)[.EXPR]` | No document in my `f` matches; true when empty | `ref.none(blocked_by).kind=research` |
| `refby.all(f).EXPR` | Every document whose `f` points at me matches; true when none | `refby.all(blocked_by).status=resolved` |
| `refby.any(f)[.EXPR]` | At least one document whose `f` points at me matches; false when none | `refby.any(blocked_by).status=open` |
| `refby.none(f)[.EXPR]` | No document whose `f` points at me matches; true when none | `refby.none(sources)` |

- **Omitting `.EXPR`** tests only whether arrows exist: `ref.any(blocked_by)` = "I have a blocker"; `refby.none(sources)` = "nothing cites me". `ref.all(f)` and `refby.all(f)` need a `.EXPR`; without one they are errors, with the hint `use ref.any(f) or ref.none(f)`, since "every arrow passes a condition that is not there" is always true. For `ref.*` an arrow is counted from the value written in the field, dangling refs included, so a ticket whose only blocker points at nothing does not look unblocked. To find dangling refs, use `ref.any(f).path!=*`: a dangling ref has no document and so no `path`, which satisfies `!=` under the rule below. (`refby` arrows always come from a document that exists, so `refby` has no dangling case.)
- **Pseudo-fields** on every document: `path`, `key` (coded only), `code`, `collection`, `schema`, `namespace`. `$body` is a virtual ref field holding body links. A reached document in another namespace reports that namespace's collection name. `namespace` is the namespace's folder name, `default` in a one-namespace project; for a document reached through an import it is the alias, followed by `::` and the namespace when the imported project has several (`chief::story-3`). These names are reserved: `schema.valid` rejects a schema field that uses one, or any name starting with `$`. The list is closed; adding a pseudo-field later is a breaking change. `$body` is valid only as `f` inside `ref.*(f)` and `refby.*(f)`.
- **Reached documents** are read under their own schema. A reached document lacking the field, or a dangling ref, counts as absent (see Absence and negation); dangling refs also warn on stderr.
- **Absence and negation.** `k!=v` is exactly NOT `k=v`, in every form (single value, list, glob, array). Something absent, whether a document without the field or a dangling ref, fails every positive condition (`=` in any form, `k=*`) and satisfies every `!=`. The ordering comparisons (`<`, `<=`, `>`, `>=`) are the exception: absent fails them. This keeps paired queries complementary: `ref.all(blocked_by).status=resolved` and `ref.any(blocked_by).status!=resolved` split the open tickets between them, and none falls through. Because `k!=v` includes documents whose schema has no `k`, use it with `--collection` (or add `k=*`) when a query spans collections.
- **Values and escaping.** In a value only three characters are special: `,` (separates alternatives), `*` (glob) and `\`. Put `\` before one to mean it literally: `title=Cosmos\, or SQL`, `k=\*`. `\` before any other character, or at the end of a value, is an error, which catches typos and leaves room to add special characters later. `*` is the only glob; there is no `?` and no `[...]`. `k=*` means "present" and `k=\*` a literal star. `=`, `<`, `>` and `!` need no escape in a value, because the expression is split at the first operator after the field name, taking the longest of `!=`, `<=`, `>=`, `=`, `<`, `>`. The same rules apply to `--where`, `--if` and `--set`, except that `--set` splits a value on `,` only for array fields. Wrap the whole expression in single quotes so the shell leaves `\`, `*`, `<` and `>` alone. See Quoting in the shell.
- **Names and scope.** A field name is `[A-Za-z_][A-Za-z0-9_-]*`, and `schema.valid` holds schema fields to the same rule, so every field can be queried. A field name unknown to every schema in scope is an error, not an empty result. For a plain condition the scope is the collections chosen with `--collection` or `--code`, or every collection of the namespaces in scope when none is chosen; a document whose schema lacks the field counts as absent. In `ref.*(f)` and `refby.*(f)`, `f` must be a field of type `ref` or `ref[]`, or `$body`, defined in a schema of this project or one it imports. The scope of the condition after `ref.*(f)` is the schemas named by `f`'s `target` (every schema in this project and its imports when the target is `"*"`); after `refby.*(f)` it is the schemas that define `f`; for `$body` in either it is every schema in this project and its imports.
- **Syntax.** An expression is read whole, as one argument: spaces belong to names and values, so `status = open` is an error, with the hint `did you mean status=open?`. Field names, values, globs and enum values are case-sensitive. An empty value is an error in `--where` and `--if` (use `k!=*` to test for absent or empty); in `--set`, `k=` removes the field. On an array field `=` means some element matches and `!=` means no element does. The condition after `ref.*(f).` is a plain condition; another `ref.*` inside it is an error, as is anything after `)` that is not `.EXPR`. The ordering comparisons take one value, so `k<a,b` is an error. In a list, each value is coerced by the field's type on its own, and one that cannot be coerced makes the whole expression an error. In `--set`, an unescaped `*` in a value is an error (write `\*` for a literal star), and `,` in the value of a scalar field is an ordinary character.
- **Coercion.** Values are coerced by schema type. A value outside an `enum` is an error (except with globs), so typos fail loudly.
- **Comparisons** (`<`, `<=`, `>`, `>=`) apply to `number`, `date` and `datetime` only; on any other type they are an error. `datetime` values compare as instants, offsets included. A date-only value compared with a `datetime` field compares against the field's date part. A document without the field satisfies no ordering comparison. Always quote the expression: `<` and `>` are shell redirections.
- **Rule of thumb.** "Does such a document exist" → `any`; "is nothing in the way" → `all`; "is there none" → `none`. For a single `ref` field prefer `any`, since `all` is true when the field is empty.
- **v1 limits.** One hop, no OR across fields. All `--where` conditions are ANDed.

**Quoting in the shell.** Give `--where`, `--if`, `--set` and `--namespace` values to typdoc exactly as written, in single quotes: `--where 'title=Cosmos\, or SQL'`, `--namespace '*'`, `--namespace 'chief::*'`, `TYPDOC_NAMESPACE='*'`. v1 covers sh and bash, where single quotes pass every character, `\`, `*`, `<`, `>` and `!` included, to typdoc untouched. A shell is on this list only while a test runs the documented examples through that shell itself; when such a test cannot run, the shell is removed from the list, not kept without evidence. zsh, fish, PowerShell and cmd are not covered in v1. zsh is the default shell on macOS, so the most common way to run typdoc on macOS is not a covered shell. On a shell outside the list, avoid values that contain a backslash, or run a query whose answer you already know before relying on it: the danger is not an error but an expression the shell has changed that still parses and gives a plausible wrong answer. Supporting another shell means adding it here with its own test.

## Commands

Nine commands cover the lifecycle; `new`, `set` and `mv` write documents, `pull` writes pinned schemas, and `new` also records the number it allocated as the collection's `last` in the namespace's state file. Every document argument accepts a key (`WF-3`) or a path.

| Command | Writes | Purpose |
| --- | --- | --- |
| `new` | yes | Create a document; allocate a key for coded schemas |
| `get` | no | Read one document's frontmatter |
| `list` | no | Query documents |
| `set` | yes | Update fields, optionally compare-and-set |
| `toc` | no | Heading outline with line ranges |
| `refs` | no | Outgoing or incoming refs |
| `mv` | yes | Move a file and rewrite every ref to it |
| `pull` | schemas | Re-fetch remote schemas and update their pins |
| `validate` | no | Check schemas, documents and refs |

All commands accept `--json`; `--namespace` and `TYPDOC_NAMESPACE` choose the namespace (see Choosing a namespace) and `TYPDOC_DIR` names the project (see Discovery).

**Cost.** Every run builds its index from the files with no cache, so the time of a run grows with the number of documents. v1 promises no figure for it; one will be measured when the index exists. `list` reports `total`, so it filters every document even under `--limit`: a choice made so that a result says how much it left out (see JSON output), and the cost of it is part of the cost above.

### typdoc new

```bash
typdoc new <CODE> "<title>" [--set k=v ...]      # coded schema: prints the new key
typdoc new <path> [--set k=v ...]                  # path-identified schema
typdoc new WF "Cosmos or SQL?" --set kind=grilling --set blocked_by=WF-1
# stdout: WF-3
```

For a code, allocates the next number under the namespace's lock: the larger of the highest existing number in the collection within this namespace and the collection's `last` in `.typdoc/state/<namespace>.json`, plus one, then records it as the new `last`. It writes into exactly one namespace: if the scope holds more than one, it exits 1 with the choices. A number is never reused after its document is deleted, as long as the state file records the collection (see State); when it does not and the collection has coded documents in this namespace, `state.missing` stops the command with exit 2 and nothing is written. A file created by hand with a higher number is respected: the highest existing number is then larger than `last`, and the numbers in between stay unissued, which is harmless. The file is named from the collection's `match` template. For a path, the path must match a collection. A `new` whose file is already there writes nothing and exits 7, for a path given on the command line and for a name a template produced alike; the second should not be reachable, since the number is one nobody has used, and it is checked because an unchecked impossible state is how a number comes to be issued twice. The check is made under the namespace's lock, and the file is created with `O_EXCL`, so the file system refuses the write rather than typdoc remembering to look first: a look taken before the lock is advice that can go stale between the looking and the writing. Fills defaults and `auto` fields, validates, then writes. Array values are comma-separated.

### typdoc get

```bash
typdoc get <key|path> [--json]
```

Returns frontmatter plus `path`, `key`, `code`, `collection`, `schema` and `namespace`. With `--json` it prints the document described under JSON output.

### typdoc list

```bash
typdoc list [--collection c[,c]] [--code C[,C]] [--where EXPR ...] [--fields f,...]
            [--sort field[:asc|:desc] ...] [--limit n] [--ids] [--json]
typdoc list --collection wayfinder,decisions --where status=open --sort status --sort updated_at:desc
```

`--collection` selects by collection name; `--code` is a shorthand that selects the collections whose schema has that code. Expressions are listed under Query. Default output is a table of key or path, `title`, and every field used in `--where`. `--json` prints the documents described under JSON output, each with all its frontmatter. `--ids` prints one key or path per line. An empty result exits 0.

**Sorting.** `--sort field:dir` may repeat; the first flag sorts first and later flags break ties. The direction is `asc` (default when omitted) or `desc`; anything else is an error. Without `--sort`, results are in key or path order.

| Sorted value | Order |
| --- | --- |
| `number`, `date`, `datetime` | By value; `datetime` as instants, offsets included |
| `enum` | By position in the schema's `values`, e.g. `open → claimed → resolved → closed` |
| `key` | By code, then numerically: `WF-2` before `WF-10` |
| `string`, `path` | Lexicographic |
| Missing value | Last, in both directions |

Only my own fields and pseudo-fields can be sort keys; fields reached through refs cannot.

### typdoc set

```bash
typdoc set <key|path> k=v [k=v ...] [--if EXPR ...]
typdoc set WF-3 status=claimed owner=zeldia-7a2f --if status=open
```

Validates types, enums, transitions and refs, then writes all fields atomically. `--if` uses `--where` expressions and is checked under the same lock as the write; if any is false, nothing is written and the exit code is 3. `k=` removes a field. Writing an `auto` field directly is a validation error; when at least one value changes, `auto: update` fields are set to the current time in the same write.

### typdoc toc

```bash
typdoc toc <key|path> [--depth n] [--json]
```

Lists body headings with line ranges counted from the top of the file, frontmatter included, so they match editor and file-tool line numbers. Headings inside fenced code are ignored; headings inside block quotes and list items are listed. `--json` returns the headings as described under JSON output; `slug` is what a `#heading` link must use (see Heading anchors under Refs).

### typdoc refs

```bash
typdoc refs <key|path> [--field f|$body] [--reverse] [--json]
typdoc refs precedents/secret-handling.md --reverse
# chief:WF-7   context
# learnings/x.md   $body
```

Outgoing refs by default; `--reverse` scans every namespace of this project and the projects it imports. `--field` keeps only the refs held in that field, in either direction. With `--json` the output is described under JSON output.

### typdoc mv

```bash
typdoc mv <from> <to>                 # move within this project
typdoc mv <from> --renumber <namespace>   # move to another namespace, under a new key
# stdout for --renumber: WF-4
```

Moves a file and rewrites every ref to it that this project holds, in frontmatter and body links, across every namespace of the project, keeping each ref's written form (key, prefixed reference or relative path). It writes no file outside this project. A body link keeps its own form too: a destination written `<…>` stays that way, one written with `%20` stays percent-encoded, and if the new path contains a space, a `<` or unbalanced parentheses and the link used neither, it is written `<…>`. Takes the lock of every namespace it writes, in the order given under Lock order. Within its namespace a coded document keeps its key. A coded document cannot move to another namespace, because its key belongs to the namespace that issued it: the command fails, says so, and suggests `--renumber`. A document without a code can move between the namespaces of one project, with its refs rewritten. If the schema has a field with `auto: moves`, the previous key or path is appended to it on every move.

**How `mv` writes, and what a failure leaves.** `mv` writes many files, and a file system gives atomicity one rename at a time, so `mv` does not promise to be all or nothing. It prepares a temp file for every file it will change first, and then does the renames in one run at the end. The window that remains is the run of renames itself, and it is not closed: a failure or an interrupt inside it leaves some files renamed and the rest not.

The document itself is always moved last, after every ref has been rewritten. The reason is recovery rather than a smaller mess. If the document moved first and the command then stopped, the same command could not be run again, because the path it names as its source is gone. Moving it last means the document is still where it was, so running the same command again finishes the work: the refs already rewritten name the new path and are left alone, and the rest are rewritten. The command is its own way back, and no journal is needed.

Between the failure and the re-run the project is inconsistent, and `validate` says so: the refs already rewritten name a path that is not there yet. A `mv` that stops says plainly that the same command can be run again to finish, rather than only that it failed, so that recovering does not depend on the user working it out.

`--renumber` moves a coded document to another namespace under a new key. The destination is the value of the flag, not a second positional argument: `typdoc mv WF-2 --renumber story-3`. It has to be, because an argument that names a document is read by the rule under Arguments that name a document, where something ending in `.md` is a path, something of the form of a key is a key, and anything else is bad arguments; a bare namespace name is none of those, and making it one would mean cutting an exception into a rule that holds everywhere else. The shape also says what is true: this command does not take a destination the way `mv` does, it takes a namespace. So `mv` reads two positional arguments in its ordinary form and one with `--renumber`, and `--renumber` is a flag that must be given a value: `--renumber` with nothing after it is bad arguments, and the message says that a namespace is required.

A `--renumber` whose destination is the namespace the document is already in is refused, and nothing is written. Carrying it out would be worse than doing nothing: `WF-2` would become `WF-3` while the document stayed exactly where it was, the key `WF-2` would be dead for good because `last` had risen past it, everything outside this project that cites `WF-2` would be pointing at nothing, and `validate` would report none of it and exit 0. A command that quietly retires a key that is in use is not a move.

A coded document cannot be renumbered into another project, and that is deliberate rather than an omission. Reading across projects is written `project::namespace:key`, so the shape for naming another project's namespace exists and could be mistaken for something `--renumber` accepts. It does not: no command writes into another project (see Across namespaces).

It prints the new key, and nothing else, on standard output, as `typdoc new` prints the key it allocated, so that a shell can put it in a variable.

It holds the locks of both namespaces (in the order given under Lock order), issues the next number from the destination's `last`, moves the file and rewrites every visible ref in frontmatter and body links, bare and prefixed, in the form that is correct from each referencing document's own namespace. It reports what it cannot rewrite. The old key is never issued again, because the source namespace's `last` never goes down, so a ref left behind fails validation instead of pointing at another document. It writes under the same promise as `mv` above, with one ordering rule of its own: the destination namespace's `last` is written before the document appears under its new key. That is a rule about the document and the state file, not about the document and the refs, so it sits beside the rule that the document moves last rather than against it. A number that is recorded and then not used is skipped, and a skipped number is ordinary: a collection that runs `WF-3` and then `WF-5` is not missing a document, and nobody should go looking for one. The alternative, letting a document exist under a number the state file has not recorded, is what issues that number a second time.

**The collection a document lands in.** `mv` changes a path, and a path decides which collection a document belongs to, so a move can change a document's schema or take it out of every collection. That has three answers, and they are not the same answer.

- **Refused.** A coded document cannot move out of its own collection's folder, and a document without a code cannot move into a coded collection. Neither is a matter of policy. A coded collection's `match` takes `{key}` exactly once and allows no globs, so the name of the file is fixed by the template and a document with no code has no key to put in it; and moving a coded document out would leave the key every ref uses pointing at nothing, which is breaking references that are in use. `mv --renumber` is the way a coded document moves.
- **Moved, and reported.** A document that moves into another collection and then does not satisfy that collection's schema is moved, and what the schema rejects is reported. Refusing would leave no order of steps that works: the fields would have to be changed to suit the destination's schema before the move, which makes the document invalid under the schema it still has, and `set` validates before it writes, so that change is refused in its turn. The only way through would be to edit the file outside typdoc, and a rule that makes someone leave the tool to do ordinary work is the wrong rule. `pull` sets the precedent: it changes the schemas, reports what the change breaks, and does not roll back.
- **Allowed, and said out loud.** A document may move out of every collection. The command says so in its own words: the document will not appear in `list`, and its refs are no longer checked. Of the three this is the only one where nothing afterwards reports anything, so if the command does not say it at the time, nobody learns it.

**A move that lands on a schema the document does not satisfy exits 0.** Not 2. Exit 2 means validation failed, and everywhere else in this document it comes with nothing having been written: `set` validates before it writes, and `pull --check` writes nothing at all. A `mv` that returned 2 after moving the file would give one code two meanings in one tool, and a caller reading it could no longer tell whether the world had changed. That is the reason exit codes exist, so it is not spent here. Deciding whether a document satisfies its schema is `validate`'s work, not `mv`'s: the result of the check travels in `mv`'s `--json` output, where a caller can branch on it, and CI catches it by running `validate`, which is what `validate` is for.

What `mv` cannot rewrite, and how each case still surfaces:

- **Every other project.** A project that imports this one holds refs that `mv` does not touch, because an imported project is read-only and `mv` never crosses a project boundary. `mv` reports them, and the report names the project each unrewritten ref is in, and the refs themselves, so that whoever goes to fix them knows where to go; a count on its own is not something anyone can act on. A project this one cannot see at all cannot be reported, and is the case below.
- **Mentions.** Plain-text mentions are never rewritten. `body.mentions` checks them when it is on, and `refs.moved` names the new key.
- **Body links, when `body.links` is `off`.** A body link that `mv` cannot follow to the old path is caught by `body.links` at validation. With that rule set to `off`, it is not caught.
- **Refs from projects that do not import this one.** They are invisible from here. They are caught when that project runs `validate` itself, not when it runs here.

None of these four makes the run a failure. A `mv` that meets them finishes, exits 0 and reports each one in detail. What `mv` promises is the refs typdoc tracks: plain text was never one of them, and an import that is absent on this machine is ordinary by design, so failing on it would make `mv` fail routinely for anyone who does not have that project checked out. A project that needs its imports to be present has the mechanism already: set `imports.absent` to `error`, and a run that cannot see an import is an error before any of this.

For a coded document, none of these can point at a different document silently: a key is never issued again, and a path made from a key is never reused either. A document without a code is identified by its path, and a path can be created again later; a ref to it then resolves to the new document, which is the ref meaning what it says.

### typdoc pull

```bash
typdoc pull [<url> ...] [--check]
```

Re-fetches remote schemas (all of them, or the URLs given), including remote parents they extend, and updates `.typdoc/vendor/schemas/` and `.typdoc/lock.json` under the project lock. Prints which URLs changed. It then validates every document against the new schemas and reports what the change breaks, without rolling back. `--check` fetches and compares only, writes nothing, and exits 2 if any pin is stale, which suits CI. Other commands never touch the network except to fetch a URL that has no pin yet.

### typdoc validate

```bash
typdoc validate [<key|path> ...] [--schemas] [--strict] [--audit]
```

- **Schemas:** duplicate names or codes, `extends` cycles, undeclared overrides, invalid options, field names that break the naming rule or use a reserved name, import names colliding with URL schemes, a qualified `target` that names a schema that does not exist in the imported project.
- **Documents:** frontmatter that cannot be parsed, types, required fields, enum values, unknown fields, duplicate keys, files not fitting the collection's `match` template.
- **Refs:** missing targets, disallowed target schemas, missing `#heading` anchors, cycles on `acyclic` fields, coded documents referenced by path (warning).
- **Across namespaces:** a ref into an imported project that is absent on this machine is a warning; a present project missing the file is an error. `--strict` makes both errors.

`--schemas` checks schemas only, including that each qualified `target` names a schema that exists (schema drift). Suited to pre-commit, CI and agent post-edit hooks. The arguments are keys or paths of documents, in any mix. A key that exists in more than one namespace in scope stops the command with exit 1 and every choice listed, and an argument that names no document stops it with exit 5; in both cases before any report is made, never as a finding. `--schemas` and `--audit` describe the whole project, so combining either with arguments is bad arguments (exit 1).

**Audit mode.** `validate` is a gate: it respects configured levels and fails, so CI, hooks and agents can stop a bad change. `--audit` answers a different question, "what would I have to fix to adopt typdoc here?", and is meant for writing a config for existing files. It runs the same checks, with these differences:

|  | `validate` | `validate --audit` |
| --- | --- | --- |
| Rules set to `off` | Skipped | Reported as `info` |
| Exit code | 2 on any error | 0, unless the config itself is invalid |
| Files in no collection | Not reported | Listed, with a count |
| Files with no frontmatter | Errors per schema | Grouped separately |
| Output | One line per finding | Summary by collection and rule first, then details |

```
typdoc audit: 3 collections, 214 files (12 in no collection)

precedents    48 files   frontmatter.types 3 error · body.links 7 error · body.mentions 22 info
learnings    141 files   refs.resolve 2 error · frontmatter.unknown 18 warn
proposals     13 files   clean

in no collection: README.md, drafts/old-idea.md, ... (12)
```

A typical adoption: run `--audit`, adjust `match` until every intended file is covered, adjust schemas or fix files until the summary is clean, then turn on plain `validate` in CI. `--json` returns the summary and every finding, as described under JSON output. While the config cannot be loaded, `validate` (audit included) prints an error object with every config error it could determine; once the config loads it prints a report every time. The change from one form to the other is a change of state, from a config that cannot be loaded to one that can, and it does not alternate.

## Validation rules

Correctness rules are always on; quality rules are configured project-wide under `validation` in `config.json`, and per collection under `validation` in its collection file.

**Config shape.** One rule is one object: a `level` (`off`, `warn`, `error`) plus that rule's options, if any. The same object shape appears in `validation.global` and in a collection file's `validation`.

```json
// .typdoc/config.json
"validation": {
  "global": {
    "body.links":          { "level": "error", "ignore": ["assets/**", "generated/**"] },
    "body.mentions":       { "level": "warn", "inlineCode": true, "fencedCode": false },
    "frontmatter.unknown": { "level": "off" }
  }
}
```

```json
// .typdoc/collections/wayfinder.json
"validation": { "body.mentions": { "level": "error", "fencedCode": true } }
```

```json
// .typdoc/collections/drafts.json
"validation": { "body.links": { "level": "warn" } }
```

- **Merge order:** typdoc defaults → `validation.global` → the collection's `validation`. A collection file merges key by key, so it states only what differs.
- **`--strict`** raises every `warn` left after merging to `error`.

**Always on** (cannot be configured; queries and writes depend on them)

| Rule | Checks |
| --- | --- |
| `schema.valid` | Duplicate names or codes, `extends` cycles, undeclared overrides, invalid options, field names that break the naming rule or use a reserved name, import names colliding with URL schemes, a qualified `target` that names a schema that does not exist in the imported project |
| `frontmatter.parse` | The frontmatter block cannot be parsed (invalid YAML, or a block that is never closed). No other rule is evaluated for that file |
| `frontmatter.types` | Types, required fields, enum values |
| `frontmatter.transitions` | State changes follow `transitions` (checked on write) |
| `refs.resolve` | Frontmatter refs point at existing files |
| `refs.target` | Ref targets match the field's `target` |
| `refs.acyclic` | No cycle on `acyclic` fields |
| `keys.unique` | No two files share a key |
| `collections.overlap` | No file is matched by two collections |
| `state.missing` | A collection with coded documents in a namespace has a `last` recorded there (see State) |
| `files.unreadable` | A directory entry a `match` reaches, or a folder a `namespaces` glob reaches, that is a symbolic link, or whose name is not valid UTF-8; it is skipped and the run continues. A `namespaces` glob leaves a file that is a symbolic link alone, as it leaves any file, since only a folder can be a namespace |

**Configurable**

| Rule | Default | Options | Checks |
| --- | --- | --- | --- |
| `body.links` | `error` | `ignore` (globs of relative targets to skip, matched after percent-decoding) | Markdown links in the body (inline, image and reference-style) point at existing files; also text that looks like a link but is not, and a reference label defined twice (see Body links) |
| `body.anchors` | `error` | — | `#heading` in a link exists in the target (percent-decoded, case-insensitive; see Heading anchors) |
| `body.mentions` | `off` | `inlineCode` (`true`), `fencedCode` (`false`) | Keys mentioned in body text exist |
| `refs.codedByPath` | `warn` | — | A coded document is referenced by path instead of key |
| `refs.moved` | `error` | — | A ref, or a mention when `body.mentions` is on, points at a key or path recorded in some document's `auto: moves` field and no longer resolves. It replaces the ordinary missing-target finding for that ref and names the new key. Without a field with `auto: moves`, a moved ref is still reported as missing, without the new key. |
| `names.shadowed` | `warn` | — | A name that is both a sibling namespace and an import alias, so `name:` and `name::` reach different documents |
| `frontmatter.unknown` | `warn` | — | Frontmatter fields not in the schema |
| `filename.pattern` | `error` | — | A file in a coded collection's folder that fits no `match` template, e.g. `tickets/README.md` |
| `imports.absent` | `warn` | — | Refs into an imported project that is absent on this machine, including one whose path uses an environment variable that is unset or empty. A project that needs its imports to be there should set this to `error` in CI, because a mistyped variable name is otherwise only a warning. An import that is absent and that no ref names is not reported, even at `error`; a misspelt alias is caught where a ref names it (`bad-prefix`, see JSON output) |

**body.mentions.** Checks plain-text keys; it never turns them into refs, so `refby` and `mv` ignore mentions. The codes to look for come from the schemas of this project and the projects it imports; nothing is listed in config.

| Text | Checked |
| --- | --- |
| `see WF-3` | yes |
| `` `WF-3` `` (inline code) | per `inlineCode` |
| Inside a fenced code block | per `fencedCode` |
| `[WF-3](WF-3.md)` | no; `body.links` checks it |
| `UTF-8`, `SHA-256` | no; not a known code |
| `WF-3a`, `xWF-3` | no; word boundaries required |

A mention with no prefix is looked up in the document's own namespace only; a prefixed mention (`story-2:WF-5`, `memory::LRN-5`) in the namespace it names. No match reports *not found*; an imported project absent on this machine falls under `imports.absent`. `fencedCode` defaults to `false` because code blocks often hold logs, commands and diffs that contain key-like text.

**Config errors** are reported when the config loads. Each has an id, so a caller can branch on it without reading the message, and so that each one can have a fixture that shows it fires.

What a config error does is decided by one question: does it make checking impossible? A config error that does stops the command, which prints the error object on standard error and exits 2. The object carries every config error that can be determined, not only the first, because `validate --audit` exists to say what has to be fixed before adopting typdoc, and stopping at the first would turn it into fix one, run again. An error that makes the rest of the config impossible to interpret, a `config.json` that cannot be parsed or an unknown `version`, ends the list at that point, and the object says so with `"complete": false`; every other config error object has `"complete": true`. A config error that leaves checking possible is a finding in the report of `validate`, with its `config.` id and `file`, and stops nothing: the documents are still checked, and the finding is one of the things found. Someone adding a config error answers the question for it.



| Id | Reported when |
| --- | --- |
| `config.parse` | `config.json` cannot be parsed |
| `config.version` | `version` is missing or unknown |
| `config.unknown-key` | `config.json` or a collection file has an unknown key (`name` in `config.json` and `last` in a collection file included) |
| `config.collection-parse` | a collection file cannot be parsed |
| `config.collection-name` | a collection file's name uses anything but ASCII letters, digits, `-` and `_` |
| `config.collection-schema` | a collection names a schema that does not exist |
| `config.rule-unknown` | a rule name or option is unknown |
| `config.rule-always-on` | an always-on rule is configured |
| `config.match-template` | a `match` template breaks the placeholder rules |
| `config.coded-schema-shared` | two collections name the same coded schema |
| `config.state-uncoded` | a state entry names a collection whose schema has no code |
| `config.state-orphan` | a file in `.typdoc/state/` matches no current namespace (the message names the file and says to delete or rename it) |
| `config.namespaces-entry` | a `namespaces` entry contains `/` or `**`, names a folder that does not exist, or names a symbolic link |
| `config.namespace-name` | a matched folder's name uses anything but ASCII letters, digits, `-` and `_`, or is `default`, `http`, `https`, `mailto` or `file` |
| `config.namespace-nested` | a matched folder holds its own `.typdoc` |
| `config.schema-url` | a schema URL uses a scheme other than `http://` or `https://` |
| `config.schema-unpinned` | a remote schema has no pin and cannot be fetched |
| `config.vendor-missing` | a pinned copy is missing (run `typdoc pull`) |
| `config.vendor-edited` | a pinned copy's contents hash differently from its file name (it was edited by hand; run `typdoc pull`) |
| `config.config-dir` | `TYPDOC_CONFIG_DIR` is set but is not an absolute path to an existing directory |

**Output.** One line per finding, `path:line:col  level  message  rule`; `--json` returns the same fields, and the `path` in a line is the `path` in the JSON: relative to the project folder, so that a reader of either means the same file in any namespace. Every path `validate` prints, in a line or in the audit summary, is written that way. `line` and `col` are 1-based. `line` counts from the top of the file, frontmatter included, so it matches editors and file tools. A line ends at a line feed, at a carriage return and a line feed together, or at a carriage return alone, as in CommonMark, and a line ending at the end of the file does not begin another line, so `a\nb` and `a\nb\n` both have two lines. Every command that reports a line counts this way. `col` counts Unicode scalar values, so a Thai consonant, a Thai vowel or tone mark, an emoji and a tab each count as 1. It agrees with a UTF-16 count (the Language Server Protocol default) except after a character outside the BMP, such as an emoji, which UTF-16 counts as 2. `col` marks where the offending element starts (for a link, its `[`). `--json` carries no byte offset in v1.

```
tickets/WF-7.md:8:5              error  WF-3 not found                  body.mentions
learnings/proc-environ.md:14:22  warn   WF-9 not found                  body.mentions
drafts/idea.md:3:10              warn   link target missing: ../x.md    body.links
```

## Concurrency

One lock per namespace serializes writes, which is enough for number allocation, compare-and-set and `mv`. v1 supports locking on one machine only; a project on a shared network filesystem is not supported. v1 supports Linux. The code is written for macOS as well, but macOS is not yet a supported platform; on Windows the build fails with a message saying so (WSL is Linux and works). Keys and refs always use `/`, so the same repository can be shared across operating systems later.

- **Lock.** A file created with `O_EXCL`, holding pid, hostname and timestamp. Retries with backoff until a timeout (default 5 s, `--lock-timeout`), then exits 4. typdoc never deletes or takes over a lock that another process created, whatever its age and whether or not its pid is alive; there is no age threshold.
- **Lock order.** A command that takes more than one lock takes the project lock first, then every namespace lock in the order of the lock files' own paths, compared byte by byte as absolute paths. Two lock files that are not the same file have different paths, so the order is total and never needs a tie-break. Ordering by the file that is actually taken is what makes the rule hold wherever the locks come from: `local` and `git-common` both give every lock file a unique absolute path, so the rule needs no separate case for either. It is a total order over any set of lock files at all, which is the property relied on, and it does not depend on that set coming from one project. In v1 it always does: no command takes a lock outside the project it is run in, because no command writes outside it. Namespace names are not used, because two projects can have namespaces of the same name; the paths of documents are not used, because the namespace `default` has no folder of its own and so no path to sort by. A path is brought to its canonical form before it is compared, so that two spellings of one file are one lock. The lock file does not exist yet when the order is decided, and a path to a file that is not there cannot be canonicalized, so what is canonicalized is the directory that holds the lock file, with the file's name joined to it; the directory is created before the first lock is taken.
- **Exit 4.** The message gives the lock's path, pid, host and age, and what typdoc can tell about the owner. Still running on this machine: wait or retry; no way to remove it is suggested. No longer running on this machine: the lock is stale, and the path to delete is shown. On another host: it cannot be checked, so delete it only when the process is known to have stopped. The pid check only chooses the wording; it never decides whether a lock is valid. A hostname is taken to mean one set of processes, so a container that shares the folder and reuses the host's hostname makes the status unreliable.
- **Removing a stale lock.** At any moment exactly one party may remove a stale lock in a namespace. Two parties that each remove it on what they saw earlier can delete the new lock of a writer that has just acquired it. Who that party is belongs to the workflow; typdoc does not enforce it.
- **Releasing.** typdoc removes its own lock on normal completion, on error, and on the system's interrupt signals (on POSIX, SIGINT and SIGTERM); a forced kill (SIGKILL on POSIX) and power loss leave a stale lock. It holds the lock file open from creation to removal so the inode cannot be reused. Before removing, it checks that the path is still its own file by comparing the file identity the system provides (on POSIX, device and inode: `stat` on the path against `fstat` on the open file), never by reading the pid, host or time. If they differ, or the open file's link count is zero, it removes nothing and reports that the lock was removed by someone else during the operation. A gap remains between the check and the removal; it matters only when someone removes a lock that is in use, which the rule above forbids. The identity check and the removal are not safe to run inside a signal handler, so the handler does nothing but wake an ordinary thread, which does the work; the process then ends by the signal itself rather than with an exit code, because a run that was interrupted must not look to its caller as though typdoc decided something. A second interrupt does not cut the cleanup short: it runs once, further deliveries wait for it, and the wait is an identity check and an unlink for each lock held. `SIGKILL` stays immediate and uncatchable.

- **The window when a lock is taken.** The handler is registered before the first lock file is created, not when the first lock is wanted, so a run never holds locks it has not arranged to release. What remains is the instant inside the creating call itself: the file system makes the lock file, and the process records that it holds it when the call returns. An interrupt in between leaves a lock file that no list in the process names, and typdoc will not remove it, because in that instant it has no evidence the file is its own and removing a lock on no evidence is the takeover this document rules out. The result is a lock with no owner in one namespace. The next run that wants it waits, times out and exits 4, and that message gives the path, the pid, the host and the age, and says the owner is no longer running and which file to delete. No document is written and none is damaged; the window is left open knowingly.
- **Pins.** `lock.json` and `vendor/` belong to the project, not to a namespace, so a command that writes them (`pull`, or the first fetch of a URL with no pin) takes a project lock, `locks/.project.lock` (in `git-common` mode `<project-hash>.lock`). Its name starts with `.`, so no namespace can have it, and it follows the same rules as every other lock. A command never takes it while holding a namespace lock; one that needs both takes the project lock first, then the namespace locks in the order given under Lock order, so two commands cannot deadlock. Network fetches happen before any lock is taken. After taking the project lock the command re-reads `lock.json`: if a pin now exists it uses that, and if its bytes differ from what was just fetched it reports the difference and never overwrites it silently. A command that fetched a schema writes the pin and releases the project lock before it takes a namespace lock.
- **What v1 does not do.** It does not stop another party from removing a lock typdoc holds; it makes the affected writer notice and report. Not showing how to remove a lock while its owner is running is a way of not inviting it, not a mechanism: the path is in the message and in this document. There is no unlock command, no process start time in the lock and no random token, since all of those serve removal or takeover by the program, which v1 does not do.
- **Atomic writes.** Each file typdoc writes, `lock.json` and the files in `vendor/` included, is written to a temp file in the same directory and then renamed over the original, so a reader sees the old file or the new one whole. What v1 promises is that a document is never left damaged. It does not promise that no temp file is left behind: `SIGKILL` and a power cut cannot be intercepted, so a promise of no leftover would be false in ordinary circumstances, and a promise that ordinary events make false is worse than none. Atomicity is not durability: without an `fsync` before the rename a power cut can leave the new file in place and empty, and whether typdoc syncs before renaming is not decided.
- **Temp files.** A temp file's name follows a reserved shape, `.typdoc-tmp-<pid>-<random>`, and a file whose name has that shape is never a document, whatever any `match` says. The walker skips such a name by rule, before `match` is consulted, because a name cannot be made safe by falling outside a glob: `*` matches a leading dot in a file name (see Which files a run reads), and a project may write `match` as `*`, which matches everything. The pid and the random part are there so that two writers working at the same instant cannot choose one name. A leftover that a run meets is skipped and reported as a finding at `warn`, and it is counted in an audit among the entries that were not read, beside the symbolic links and the names that are not valid UTF-8.
- **Two kinds of rename.** Putting a document where no file was, in `new`, `mv` and `mv --renumber`, must not replace anything: a destination that exists is exit 7 and the write does not happen. Completing an atomic write of a file that is already there is the opposite case and replaces it on purpose, which is what makes a reader see the old file or the new one whole. They are different operations with different requirements, and treating them as one is how a `mv` comes to overwrite a document. Where the two are enforced differs. `new` creates the file itself, so `O_EXCL` refuses the write in the same call and nothing has to be remembered. `mv` and `mv --renumber` move a file that exists, and the plain rename a file system offers replaces in silence, so they check the destination under the lock and then rename. That leaves an instant between the check and the rename. Under the lock no other typdoc can be in the namespace, so only a program that is not typdoc can reach it, by creating a file at exactly that path in that instant; the instant is left open knowingly. A call that takes a name or fails would close it, at the cost of a document that briefly has two names and of a rule for meeting that state on a re-run, which buys less than it costs. typdoc does not own the file system, and a lock binds typdoc alone.
- **Removing a leftover.** A command that holds a lock may remove leftover temp files within that lock's scope, and no command removes one otherwise. The evidence is the lock: while it is held no other typdoc is writing in that namespace, so a leftover there belongs to a process that has gone. The age of the file is never a criterion, as it is never a criterion for a lock. A removal that fails does not make the write fail: the document matters more than a tidy folder.
- **Permissions.** A rename replaces the inode, so the mode of the file that ends up in place comes from the temp file rather than from the document that was there; a file set to `600` becomes whatever the umask allows, silently. typdoc therefore carries the existing file's mode to the temp file before renaming. It carries the mode and nothing else: the owner and group are not carried, since changing them needs privilege typdoc does not have, and access control lists and extended attributes are not carried either. A file that did not exist has no mode to carry and gets the default.
- **What is locked.** `new` holds its namespace's lock from reading `last` to writing the file and updating `last`. `set` holds it across read, `--if`, validation and write. `mv` takes the lock of every namespace it writes, in the order given under Lock order, so two `mv`s cannot deadlock. Reads never lock. The lock covers reading, checking and the rename only: no network and no waiting for input, which is why five seconds is a reasonable timeout. `mv` and `mv --renumber` are the only commands whose hold time grows with the size of the repository; a large repository may need a longer `--lock-timeout`.

| `lock` | Lock path | Use when |
| --- | --- | --- |
| `local` | `<project>/.typdoc/locks/<namespace>.lock` | All sessions share one working tree |
| `git-common` | `$(git rev-parse --git-common-dir)/typdoc/<project-hash>-<namespace>.lock` | Sessions run in separate git worktrees |

`git-common` stops two worktrees writing at the same instant, but each worktree still holds its own copy of the files: two worktrees can allocate the same key, and `validate` catches the duplicate after merge. Each worktree also holds its own copy of `state/<namespace>.json`. Worktrees that work on different namespaces never change the same state file, so their numbering cannot collide. Two worktrees on the same namespace still change the same line and conflict on merge, a louder signal than the duplicate key alone. Truly shared state across worktrees needs the project folder outside the worktrees.

**`<project-hash>`.** In `git-common` mode the lock file sits in the repository's common directory, which every worktree shares, so its name has to tell two projects apart without depending on where a worktree happens to be. `<project-hash>` is the SHA-256 of the project folder's path relative to the root of the worktree, truncated to its first 16 hexadecimal characters. The path is brought to one form before it is hashed: separators are `/`, and a trailing separator is removed. The absolute path cannot be used, because it is exactly what differs between worktrees, which is the case this mode exists for, while the path relative to the worktree root is the same in every worktree of the repository. A worktree that keeps the project at a different relative path is a different project under this rule and takes a different lock; that is the intended reading, not an accident of the formula. `git rev-parse --git-common-dir` may answer with a relative path, so it is canonicalized before a lock path is built from it.

Sixteen hexadecimal characters are 64 bits, so two projects in one repository can in principle hash the same. What follows from it is that they share a lock and wait for each other: nothing is corrupted and nothing is lost. That is accepted knowingly rather than guarded against.

The hash is stable across versions of typdoc. Changing how it is computed would make a new binary blind to a lock an old binary is holding, so it is a breaking change and is treated as one.

## JSON output

Every command accepts `--json`, and its result is one JSON object on standard output. Commands are of two kinds. A command whose result is a verdict on the project (`validate`, and `pull --check`) prints that verdict on standard output whether or not it is favourable: the findings are the result, and exit 2 says that some of them are errors. A command that does something (`get`, `list`, `toc`, `refs`, `new`, `set`, `mv`, `pull`) prints its result on standard output when it succeeds and, when it cannot, the error object described under Exit codes and errors on standard error. A new command falls on one side by asking whether its result is a judgement about the project or the outcome of an action. The result is held in a field named for it and is never printed bare, so that facts about the result can sit beside it: a `list` that `--limit` cuts short has to say so, and a bare array has no place to say it. Output may gain fields in later versions, so a consumer must ignore any field it does not know; this holds for the error object as well. A value in the output is a fact about the document, never about the command that asked: a flag that chooses or limits what is listed (`--depth`, `--limit`, `--field`) changes which items appear and never the values of an item. The output promises only what a caller cannot work out from what it is already given, because every field it promises has to stay true for as long as the version does: a count of findings per rule, for example, is not in it.

**Naming a document.** There is one way to name a document, and every shape that has to mention one uses it: `path`, `namespace` (absent for a file outside every namespace folder, which a ref can still reach), `key` when the document has a code, and `project` when the document belongs to an imported project. `project` is the alias under which this project imports it; it is absent for a document of this project. A document object is that name plus `code`, `collection`, `schema` and `fields`; a finding is that name, without `project` since a finding is always in this project, plus `rule`, `level`, `message` and a position; a reference is that name plus `field`, `written` and a position. A new shape that mentions a document adds to the name and never renames a part of it. The strings that name a document in an argument are under Arguments that name a document, after Discovery.

**A document** is the same object wherever it appears, in `get` and in `list` alike, including a `list` that reaches an imported project with `--namespace 'chief::*'`. It has the name above, `code`, `collection`, `schema`, and `fields`, which holds all of the document's frontmatter. The frontmatter stays apart in `fields` because a field that is not in the schema is kept and can have any name, `path` and `key` included.

`path` is the path of the file relative to the folder of the project the document belongs to, the folder that holds that project's `.typdoc`, so it is the same in every namespace and can be opened as it stands from there. `namespace` is not redundant with it: the namespace `default` has no folder of its own, so the paths of its documents contain no namespace name and none can be recovered from them, and that is the commonest case. Where a project has several namespaces, two documents in different ones can have the same path below their namespace folder, and the namespace tells them apart from the path's first segment onward.

**A finding** is the same object in the report of `validate` and in the `details` of an error object. It always has `rule`, `level` and `message`. It has `path`, the file it is about, relative to the project folder, for a document and for a configuration file alike; `namespace`, `collection` and `key` when the file is a document (`collection` when it is in one, `key` for a coded one only); `field` when it is about one field; and `line` and `col`, 1-based, when a position is known. A finding is always located in a file of the project being checked, never in one of an imported project: a broken ref is a fault of the document that holds it, not of its target.

| Command | Prints |
| --- | --- |
| `get` | `{ "document": <document> }` |
| `list` | `{ "documents": [<document>, ...], "total": n, "truncated": b }` |
| `validate` | `{ "summary": {...}, "findings": [<finding>, ...] }`, and with `--audit` also `"audit": {...}` |
| `refs` | `{ "document": <name>, "direction": "out" or "in", "refs": [<reference>, ...] }` |
| `toc` | `{ "document": <name>, "headings": [{ "level", "text", "slug", "line", "end" }, ...] }` |

`total` is the number of documents that match, counted before `--limit`, and `truncated` is true when `total` is larger than the number listed. Because `total` is always reported, a `list` filters every document even when `--limit` is small.

**The summary of `validate`** says what the report covers, so that a list of findings cannot be read as more than was checked. `scope` is `all` for the whole project, `paths` when arguments named the documents to check, and `schemas` for `--schemas`. `strict` is true when `--strict` was in effect. `checked` holds `namespaces`, the namespaces covered, sorted by name, and `documents`, the number of documents checked (0 for `schemas`); for `paths` it also holds `paths`, the `path` of each document checked, sorted and each once, so it can be matched with the `path` of a finding. `findings` counts the findings per `level` (`error`, `warn`, `info`) after the rule levels have been merged and after `--strict` has raised the warnings, so the numbers agree with the exit code. `strict` is there so that a count of errors is not read against a configuration file that calls the same findings warnings. With `--audit`, `summary` also has `audit`, true, and `unreported`, an object with `uncollected` and `no_frontmatter`, the number of files in each of the two lists under `audit` below. In an audit, `findings` is not the whole list of work: files in no collection and files with no frontmatter are outside it, and `unreported` puts their numbers where a reader of the summary sees them. These two counts can be worked out from the lists; they are in the summary because a reader of the summary and the findings alone would otherwise report less work than there is, which is the same reason as for `scope` and `strict`, and they are the exception to the rule that the output promises only what a caller cannot work out. A file matched by more than one collection is counted as `overlapping`, beside `unreported` and not inside it: it is reported under `collections.overlap`, so it is not outside `findings`, which is what `unreported` means, and it is checked against no schema, since there is no one schema to check it with. `not_read` is counted beside `unreported` and `overlapping` and not inside it, for the same reason as `overlapping`: `unreported` means outside `findings`, and an entry that was not read is reported in `findings`. The account a reader can take from the summary is therefore `checked.documents` plus every count of what was not checked, `unreported.uncollected` and `unreported.no_frontmatter` and `overlapping` and `not_read`, and that total is the number of directory entries the run met. Without `not_read` the account is short by every entry the run skipped, which is the one case where a file is reported and counted nowhere.

```json
{ "summary": { "scope": "all", "strict": false,
               "checked": { "namespaces": ["default"], "documents": 214 },
               "findings": { "error": 3, "warn": 12, "info": 0 } },
  "findings": [ { "level": "error", "rule": "body.links", "message": "...",
                  "path": "tickets/WF-7.md", "namespace": "default", "key": "WF-7", "line": 8, "col": 5 } ] }
```

**Audit.** With `--audit` the report also has `audit`: `collections`, one `{ "name", "documents" }` for each collection of the project, sorted by name, one that holds no document included with `documents` 0, and a file matched by more than one collection counted in the number of none of them, since it is listed under `overlapping` and checked against no schema, so a collection whose every file is also matched by another shows 0; `uncollected`, the `path` of every file that belongs to no collection; `no_frontmatter`, the `path` of every file that belongs to a collection and has no frontmatter; and `overlapping`, one `{ "path", "collections" }` for every file matched by more than one collection, with `collections` the names of the collections that match it, sorted by name. `not_read` holds one `{ "path", "reason" }` for every directory entry the run met and did not read — a symbolic link, a name that is not valid UTF-8, or a leftover temp file — so that an entry which is reported in `findings` is also counted somewhere in the account. `uncollected`, `no_frontmatter` and `overlapping` are sorted by `path`. The three lists do not overlap: a file in no collection has no schema to say what its frontmatter should hold, so it is only in `uncollected`, and a file matched by more than one collection belongs to collections rather than to none and its frontmatter is never read, so it is only in `overlapping`. The documents per collection cannot be worked out from `findings`, since a clean document produces none, and a finding carries its `collection` so that the counts per collection and rule, which the text form prints as a table, can be worked out from `findings`; they are not in the JSON.

The two modes treat a file with no frontmatter differently, and this is a difference of mechanism, not of presentation. `validate` evaluates it against its schema like any document, so each required field it lacks is a finding. `--audit` does not evaluate it: it lists the file in `no_frontmatter` and produces no findings for it, which is the point of grouping it, since a directory of plain notes would otherwise bury every other defect under required-field findings.

**Headings.** In `toc`, `document` is the name of the document asked about, and each heading has its `level`, `text`, `slug` and `line`, the line of the heading, counted as under Output. `end` is the last line of the heading's section: the section runs to the line before the next heading of the same or a shallower level, or to the last line of the file, and it includes the sections of the headings under it. A heading with nothing under it has `end` equal to `line`. The ranges therefore nest and do not tile the file: the range of a heading contains the ranges of the headings under it, so reading every range in turn reads some lines more than once. `end` is a property of the document: `--depth` chooses which headings are listed and never changes the `end` of one that is. Headings are in the order of `line`, and the order is guaranteed.

**References.** In `refs`, `document` is the name of the document asked about, and `direction` is `out` for the refs it holds and `in` for `--reverse`. A reference is the name of the document at the other end (the target for `out`, the document that holds the ref for `in`), plus `field`, the field that holds the ref (`$body` for a body link), and `written`, the text as it is written in the file, which cannot be worked out from the other end and which `mv` needs to keep the written form. A body link also has `line` and `col`, 1-based.

A reference that does not resolve has no `path` and has `unresolved` instead, one of three values: `not-found`, the place the ref names is present and the file or key is not; `import-absent`, the import it names is not on this machine, which `imports.absent` reports; and `bad-prefix`, the prefix names no namespace and no import, or names a project with several namespaces without saying which. A missing `path` alone would make a broken link and a machine that has not been set up look the same, and they are different problems with different fixes. `path` and `unresolved` never appear together, and `unresolved` occurs only for `out`, since a reference read from a document that holds it has been found. Unresolved references are listed: a ref is counted from what is written in the field. `--field` keeps only the refs in that field, `$body` for body links, and it means the field that holds the ref in both directions, so with `--reverse` it is a field of the document that holds it.

The order of `refs` is guaranteed. For `out` it is the fields in the order they appear in the document, then `$body` by position, and within a field the values in the order they are written. For `in` it is the documents that hold the refs, those of this project first and then those of imported projects by alias, each by `path` as under Order, and then as for `out`.

**Order.** The order of `findings` is guaranteed: by `path`, then `line`, `col`, `rule` and `message`, a finding with no position before one with a position in the same file. `path` is compared as it is under Sorting: lexicographically, by the bytes of the path. This differs from `list` on purpose. `findings` are ordered by position in a file, so the path decides; `list` orders documents, and a document's key contains a number, which is compared as a number. They order different things, and making them agree would leave the order of findings depending on the numbers in keys. In `audit`, `collections` is sorted by name and the two lists as `path` is.

## Exit codes and errors

Exit codes let an agent branch without parsing text.

| Code | Meaning |
| --- | --- |
| 0 | Success, including an empty `list` result and a well-formed query that matches nothing |
| 1 | Bad arguments: a malformed option or expression (a query that breaks the grammar included), or a key or write that is ambiguous across namespaces |
| 2 | Validation failed: schema, type, enum, transition or ref |
| 3 | An `--if` condition was false; nothing written |
| 4 | Lock not acquired within the timeout |
| 5 | Not found: the key, path or file the command was asked to act on does not exist |
| 6 | I/O: a file or directory cannot be read or written |
| 7 | The destination already exists: the write would replace a file that is there |

A new code is added only when the caller has to act differently: not found may lead to creating the document, bad arguments are a defect in the call and are not retried, and an I/O failure is a problem of the environment that may be retried. A destination that already exists earns its own code by the same test: the call was correct in every part, so it is not code 1, which says the call is a defect and is not retried; what has to happen next is to choose another name or open the file that is there, which is neither of those. Finer detail belongs in an id: every error carries in `details[].rule` an id that names its specific cause, and the ids are listed with the shapes of the output. A malformed query expression exits 1; a well-formed query that matches nothing exits 0 with an empty result and is never an error.

Errors go to stderr. With `--json`, stderr carries one object:

```json
{ "error": "transition not allowed: open -> resolved", "code": 2,
  "details": [{ "level": "error", "rule": "frontmatter.transitions",
                "message": "transition not allowed: open -> resolved",
                "path": "tickets/WF-3.md", "namespace": "default", "key": "WF-3", "field": "status" }] }
```

`details` holds findings, in the shape described under JSON output. For a config error, `rule` holds the error's id from the table under Config errors (all start with `config.`) and `path` is the configuration file it is about.

When a key or a write is ambiguous across namespaces, the exit code is 1 and the object carries `candidates`: every choice, written as a prefixed key or a namespace name.

When typdoc is interrupted (SIGINT or SIGTERM on POSIX) it removes its own locks and then ends by that signal, so the caller sees a process killed by a signal and no code from this table, and no error object is written. The table describes the outcomes of a command, not every way a process can end.

## Worked examples

Two projects: `chief` holds wayfinder and implementation tickets, `memory` holds typmem learnings, precedents and proposals, and `chief` imports `memory`.

### Wayfinder tickets (project: chief)

Decision-tickets and implementation tickets are one schema in one collection, numbered in one sequence and told apart by `kind`, since the skill requires one numbering for both. The decision kinds are `research`, `prototype`, `grilling` and `task`.

| Skill step | Command |
| --- | --- |
| Create a decision-ticket | `typdoc new WF "..." --set kind=grilling` |
| Wire blockers | `typdoc set WF-5 blocked_by=WF-3,WF-4` |
| Find the frontier | `typdoc list --code WF --where kind=research,prototype,grilling,task --where status=open --where 'ref.all(blocked_by).status=resolved'` |
| Find blocked tickets | `typdoc list --code WF --where kind=research,prototype,grilling,task --where status=open --where 'ref.any(blocked_by).status!=resolved'` |
| Claim | `typdoc set WF-3 status=claimed owner=$SID --if status=open` |
| Locate `## Answer` | `typdoc toc WF-3 --json`, then edit with file tools |
| Resolve | `typdoc set WF-3 status=resolved --if owner=$SID` |
| Who was waiting on it | `typdoc list --where blocked_by=WF-3` |
| Implementation waits on a decision | `typdoc new WF "..." --set kind=feature --set blocked_by=WF-3` |
| Open tickets others wait on | `typdoc list --where status=open --where 'refby.any(blocked_by).status=open'` |

Claiming the first ready ticket, retrying when another session wins:

```bash
while key=$(typdoc list --code WF --where kind=research,prototype,grilling,task \
              --where status=open --where 'ref.all(blocked_by).status=resolved' --limit 1 --ids) && [ -n "$key" ]; do
  typdoc set "$key" status=claimed owner="$SID" --if status=open && break
  [ $? -eq 3 ] || exit 1   # 3 = lost the race; anything else is a real error
done
```

The skill text needs two updates: its ticket template moves from inline `Type:` lines to frontmatter, and `Blocked by: None` becomes `blocked_by: []`.

### typmem memory (project: memory)

Existing precedents write `sources` relative to the memory namespace folder (`learnings/...`), so each collection sets `"refBase": "namespace"` and no file needs editing.

```json
// .typdoc/config.json
{ "version": 1 }
```

```json
// .typdoc/collections/precedents.json
{ "match": "precedents/*.md", "schema": "schemas/precedent.json", "refBase": "namespace" }
```

```json
// .typdoc/collections/learnings.json
{ "match": "learnings/*.md", "schema": "schemas/learning.json", "refBase": "namespace" }
```

```json
// .typdoc/collections/proposals.json
{ "match": "proposals/*.md", "schema": "schemas/proposal.json", "refBase": "namespace" }
```

| Question | Command |
| --- | --- |
| Precedents citing a retired learning | `typdoc list --collection precedents --where 'ref.any(sources).status=retired'` |
| Learnings nothing cites, in frontmatter or body | `typdoc list --collection learnings --where 'refby.none(sources)' --where 'refby.none($body)'` |
| Proposals not yet promoted | `typdoc list --collection proposals --where 'refby.none(proposal)'` |
| Who relies on this learning | `typdoc refs learnings/never-send-secrets-over-ship.md --reverse` |
| Broken frontmatter refs, body links and anchors | `typdoc validate` |

### Across the two projects

From `chief`, a ticket points into memory with a prefixed reference in frontmatter and an ordinary relative link in its body:

```yaml
context: [memory::precedents/secret-handling.md]
```

```bash
# open decision tickets that rely on at least one memory precedent
typdoc list --collection wayfinder --where status=open --where 'ref.any(context).collection=precedents'
```

`memory` does not import `chief`, so running `refs --reverse` from memory does not see the ticket; running it from `chief` does, because `chief` scans its own project.

### Several stories in one `.typdoc` (project: chief)

If each story is worked in its own worktree, one project can hold every story as a namespace: each story numbers its own tickets, and two stories never change the same state file.

```
.chief/.typdoc/config.json                  { "version": 1, "namespaces": "story-*" }
.chief/.typdoc/collections/wayfinder.json   { "match": "_tickets/{key}.md", "schema": "schemas/wayfinder.json" }
.chief/.typdoc/state/story-2.json           { "wayfinder": { "last": 15 } }
.chief/.typdoc/state/story-3.json           { "wayfinder": { "last": 7 } }
```

| Question | Command |
| --- | --- |
| Open tickets in this story | `cd .chief/story-3 && typdoc list --where status=open` |
| Open tickets in every story | `TYPDOC_DIR=.chief typdoc list --namespace '*' --where status=open` |
| A ticket in another story | `typdoc get story-2:WF-5` |
