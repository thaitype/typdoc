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

**Match templates.** For a coded schema, `match` is a template with placeholders; for a schema without a code, it is a glob.

| Placeholder | Stands for | Allowed in |
| --- | --- | --- |
| `{key}` | `{code}-{number}`, with the code taken from the schema, e.g. `WF-3` | Coded schemas; exactly once, no globs |
| `*`, `**` | Glob | Schemas without a code; no placeholders |

One template serves both directions: it decides which files belong to the collection, and `typdoc new` uses it to name new files. Because `{key}` includes the schema's code, several coded collections can share one template in one folder: `tickets/{key}.md` matches `WF-3.md` for one schema and `RFC-4.md` for another. Each code counts on one number sequence of its own in each namespace, so a coded schema serves exactly one collection; two collections naming the same coded schema is a config error, as is a state entry for a collection whose schema has no code.

**Which files a run reads.** A glob does not enter a folder whose name begins with `.`, which is the rule namespaces already follow, so a collection and a namespace answer the question the same way. A literal segment does enter one: a project that keeps its documents under `.agents/` names that folder in `match` and gets them, because naming a folder is saying it is wanted, while a glob is saying "whatever is here". A `*` does match a leading dot in a file name, since a file is named by the template that reaches it rather than found by walking into it. A symbolic link to a folder is not followed, so a run cannot leave the project or read one file twice under two names. `.gitignore` is not read: what a version control system hides is a different question from what a project declares, and a file that no `match` reaches is already outside every collection. A directory entry a template reaches that is a symbolic link, or whose name is not valid UTF-8, is skipped and reported under `files.unreadable` rather than stopping the run: one name that cannot be read should not deny an answer about every other file beside it.

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

A pinned copy lives at `vendor/<section>/<sha256>`, with no extension: `vendor/schemas/9f2c…` for the entry in `docs/design/spec/SPC-16.md`. The path is never stored; it follows from the section name and the hash, so a second kind of pin needs no change to the rule. A copy counts as edited by hand when the hash of its contents differs from its file name. That lets a file in `vendor/` check itself without opening `lock.json`, and leaves one value where two could disagree. Files here are written like every other file (temp file, then rename), so a command that reads `lock.json` or a copy while `pull` writes it sees the old file or the new one whole. v1 never deletes anything in `vendor/`: it only grows, on purpose. Clearing copies nothing refers to is a separate job, and a large `vendor/` is not a bug to fix. A pinned copy that is missing is a config error that says to run `typdoc pull`, which fetches again and compares with the pinned SHA-256: equal restores the copy; different is reported as a changed URL, an update of the pin, as `pull` always reports.

There is one config location, `.typdoc/config.json`. The error for a folder with no project names that file as the one looked for, so a person who looked for the config under any other name is told where it is, whatever the name was; no other name is checked for.

**Machine-specific imports.** When an imported project's location differs per machine, put it in an environment variable or in a machine file, `imports.json`, which is merged under the project's own `imports`, so no machine-specific path is committed. The file is found in this order, stopping at the first step that applies:

1. `TYPDOC_CONFIG_DIR`, if set: the file is `$TYPDOC_CONFIG_DIR/imports.json`. The value must be an absolute path to a directory that exists; anything else is an error, since it was set on purpose.
2. `XDG_CONFIG_HOME`, if set to an absolute path: the file is `$XDG_CONFIG_HOME/typdoc/imports.json`. Unset, empty or relative counts as not set, as the XDG specification says.
3. The platform default. In v1, on Linux and macOS: `~/.config/typdoc/imports.json`.

Support for another platform is one new line at step 3; steps 1 and 2 do not change. `TYPDOC_CONFIG_DIR` also lets tests and containers with an odd `HOME` move the file without borrowing the system's variable. If the file does not exist there are no machine-specific imports, and a ref into an import that this leaves absent is reported by `imports.absent` (a warning by default); there is no second mechanism. An error about the file names the path that was searched and which of the three steps it came from. No test reads the real home directory of whoever runs it, and each step is exercised with a fake `HOME` or environment.

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
| `values` | `enum` | Allowed values, in order; the order is also the sort order |
| `transitions` | `enum` | Map of value → allowed next values. Omitted: any change allowed. |
| `target` | `ref`, `ref[]` | `"*"` (default: any file) or a list of schema names |
| `acyclic` | `ref`, `ref[]` | Reject any cycle formed through this field |
| `override` | all | Required to redefine an inherited field |

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
- Frontmatter fields not in the schema are kept on write and reported by `validate`.

## Refs

**Body links.** Standard Markdown links count, in every form the parser reads as a link: inline `[text](path)` and `[text](path#heading)`, images `![alt](path)`, and reference-style `[text][ref]`, `[ref][]` and `[ref]` with a definition `[ref]: path`. Autolinks (`<https://…>`) are URL-scheme links and are skipped. Paths are relative to the document, percent-decoded, and may be written `<my file.md>`. Links starting with a URL scheme (`https:`, `mailto:` and so on) that is not a namespace name or an import alias are always skipped; no configuration is needed. A prefixed link is written `name:path` or `name::path`; after the prefix comes a path, never a key. Links inside fenced code blocks and inline code are not links. Relative paths that should not be checked (images, generated files) go in the `ignore` option of the `body.links` rule. Plain-text mentions are never refs; the `body.mentions` rule can check that they exist (see Validation rules). Body links are exposed to queries as the virtual ref field `$body`.

**Reference definitions.** Every definition line is checked once, at the definition, whether or not anything uses it, with the number of places that use it (`link target missing: notes/x.md (used 3 times)`); the uses are not reported separately. Labels are compared as CommonMark does: case-folded, whitespace collapsed. When a label is defined twice, CommonMark ignores the later definition; `body.links` reports it (`already defined at line N; this definition is ignored`) even when both point at the same file, and does not check its target. A definition inside a code block is not a definition. A definition that nothing uses is checked but does not appear in `$body`, so `refby` never counts a document that does not actually link to another. `mv` rewrites the active definition and leaves an ignored one alone.

**Text that looks like a link but is not.** Under `body.links`, a `[text](inner)` or `![text](inner)` outside code that the parser does not read as a link, and a line `[label]: inner` that it does not read as a definition, is reported when `inner` has no URL scheme (a namespace name or import alias does not count as one) and, after removing a trailing title (`"…"`, `'…'` or `(…)`), ends with a file extension, optionally followed by `#anchor`. An extension is a `.` followed by one to eight ASCII letters or digits, at least one of them a letter. The usual cause is an unescaped space, which CommonMark does not allow in a bare destination; the message says to write `<my file.md>` or `my%20file.md`. Without this check such a link would be invisible: the reader sees a link and the checker sees text. The finding is at the `[` (or the `!`), or at the definition line; for a rejected definition it cannot say how many places use it. A `[text][ref]` with no definition is not reported: CommonMark reads it as plain text, and the shape is too common in ordinary prose (`a[0][1]`).

**Heading anchors.** The `slug` of a heading follows GitHub's algorithm, so a link that passes `validate` also works on GitHub. Take the heading's plain text (text and code spans; image alt text, line breaks and inline HTML contribute nothing), lowercase it, delete punctuation other than `-` and `_`, symbols and other characters that are not letters or digits, and turn each space into `-`. Marks count as part of a letter, so Thai vowels and tone marks stay; non-ASCII text is kept as written, never transliterated. A slug that repeats an earlier one in the same document gets `-1`, `-2` and so on, skipping any result already taken (`Dup`, `Dup`, `Dup 1` give `dup`, `dup-1`, `dup-1-1`). Every heading counts, including those inside block quotes and list items but not those inside fenced code, so the numbering matches GitHub's. A heading whose slug is empty (`## !!!`, `## 😀`) is not special: the first gets `""` and cannot be linked to, the next `-1`, then `-2`. In a link, the fragment is percent-decoded (a `%` not followed by two hex digits is kept as written) and then compared with the slug without regard to case. Slugs are used only for anchors, never for file names. The exact character classes are pinned by the contract's fixtures, generated from GitHub's renderer.

**Canonical form.** A coded document should be referenced by key in frontmatter; referencing it by path works but `validate` warns, since a path changes when the file is moved and a key does not. Body links always use paths.

**Across namespaces.** A relative path that leaves the namespace, a sibling prefix or an import prefix lands in another namespace or project. `typdoc` finds that file's nearest `.typdoc/config.json` to learn its schema; namespaces of one project share collections and schemas, so a sibling is checked exactly like the document's own namespace. Forward traversal needs no configuration. Reverse lookup (`refby`, `refs --reverse`, `mv`) scans every namespace of this project and the projects it imports. Imports are one-way: `chief` importing `memory` does not let `memory` see `chief`. An imported project is read-only here, with no exception: no command writes a file in it, `mv` included. A `mv` reads the refs of an imported project so that it can report the ones that will be left pointing at the old path, and it changes none of them. Because it writes nothing there, it takes no lock there either: a lock belongs to the project that owns the file, and typdoc never takes one in a project it does not write to.

|  | Without imports | With imports |
| --- | --- | --- |
| Forward refs and validation across namespaces and projects | yes | yes |
| `refby` sees refs from | every namespace of this project | also the imported projects |
| `mv` rewrites refs in | every namespace of this project, preserving each ref's written form | nothing: an imported project is read-only, and its refs are reported instead |

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

- **Pseudo-fields** on every document: `path`, `key` (coded only), `code`, `collection`, `schema`, `namespace`. `$body` is a virtual ref field holding body links. A reached document in another namespace reports that namespace's collection name. `namespace` is the namespace's folder name, `default` in a one-namespace project; for a document reached through an import it is the alias, followed by `::` and the namespace when the imported project has several (`chief::story-3`). These names are reserved: `schema.valid` rejects a schema field that uses one, or any name starting with `$`. The list is closed; adding a pseudo-field later is a breaking change. `$body` is valid only as `f` inside `ref.*(f)` and `refby.*(f)`.
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

All commands accept `--json`; `--namespace` and `TYPDOC_NAMESPACE` choose the namespace (see `docs/design/spec/SPC-7.md`) and `TYPDOC_DIR` names the project (see `docs/design/spec/SPC-7.md`).

### typdoc get

```bash
typdoc get <key|path> [--json]
```

Returns frontmatter plus `path`, `key`, `code`, `collection`, `schema` and `namespace`. With `--json` it prints the document described under JSON output.

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

- **`--strict`** raises every `warn` left after merging to `error`.

**Always on** (cannot be configured; queries and writes depend on them)

| Rule | Checks |
| --- | --- |
| `schema.valid` | Duplicate names or codes, `extends` cycles, undeclared overrides, invalid options, field names that break the naming rule or use a reserved name, import names colliding with URL schemes, a qualified `target` that names a schema that does not exist in the imported project |
| `frontmatter.types` | Types, required fields, enum values |
| `frontmatter.transitions` | State changes follow `transitions` (checked on write) |
| `refs.resolve` | Frontmatter refs point at existing files |
| `refs.target` | Ref targets match the field's `target` |
| `refs.acyclic` | No cycle on `acyclic` fields |
| `keys.unique` | No two files share a key |
| `collections.overlap` | No file is matched by two collections |
| `state.missing` | A collection with coded documents in a namespace has a `last` recorded there (see `docs/design/spec/SPC-8.md`) |
| `state.malformed` | A recorded `last` is a whole number that can be held: not text, `null`, negative, a fraction, or too large. The message gives the value written and what is expected |
| `state.behind` | A recorded `last` is not lower than the highest existing number of that collection in the namespace. At `warn`: allocation takes the larger of the two, so the next number is still right |
| `state.retired` | At `warn`: a state entry names a collection this project no longer has. It is kept, because it is the only record that those numbers were issued; nothing removes it |
| `files.unreadable` | A directory entry a `match` reaches, or a folder a `namespaces` glob reaches, that is a symbolic link, or whose name is not valid UTF-8; it is skipped and the run continues. A `namespaces` glob leaves a file that is a symbolic link alone, as it leaves any file, since only a folder can be a namespace |

**Configurable**

| Rule | Default | Options | Checks |
| --- | --- | --- | --- |
| `body.anchors` | `error` | — | `#heading` in a link exists in the target (percent-decoded, case-insensitive; see Heading anchors) |
| `body.mentions` | `off` | `inlineCode` (`true`), `fencedCode` (`false`) | Keys mentioned in body text exist |
| `refs.codedByPath` | `warn` | — | A coded document is referenced by path instead of key |
| `names.shadowed` | `warn` | — | A name that is both a sibling namespace and an import alias, so `name:` and `name::` reach different documents |
| `frontmatter.unknown` | `warn` | — | Frontmatter fields not in the schema |
| `filename.pattern` | `error` | — | A file in a coded collection's folder that fits no `match` template, e.g. `tickets/README.md` |
| `imports.absent` | `warn` | — | Refs into an imported project that is absent on this machine, including one whose path uses an environment variable that is unset or empty. A project that needs its imports to be there should set this to `error` in CI, because a mistyped variable name is otherwise only a warning. An import that is absent and that no ref names is not reported, even at `error`; a misspelt alias is caught where a ref names it (`bad-prefix`, see JSON output) |

**body.mentions.** Checks plain-text keys; it never turns them into refs, so `refby` and `mv` ignore mentions. The codes to look for come from the schemas of this project and the projects it imports; nothing is listed in config.

A mention with no prefix is looked up in the document's own namespace only; a prefixed mention (`story-2:WF-5`, `memory::LRN-5`) in the namespace it names. No match reports *not found*; an imported project absent on this machine falls under `imports.absent`. `fencedCode` defaults to `false` because code blocks often hold logs, commands and diffs that contain key-like text.

## Concurrency

One lock per namespace serializes writes, which is enough for number allocation, compare-and-set and `mv`. v1 supports locking on one machine only; a project on a shared network filesystem is not supported. v1 supports Linux. The code is written for macOS as well, but macOS is not yet a supported platform; on Windows the build fails with a message saying so (WSL is Linux and works). Keys and refs always use `/`, so the same repository can be shared across operating systems later.

- **Removing a stale lock.** At any moment exactly one party may remove a stale lock in a namespace. Two parties that each remove it on what they saw earlier can delete the new lock of a writer that has just acquired it. Who that party is belongs to the workflow; typdoc does not enforce it.
- **Releasing.** typdoc removes its own lock on normal completion, on error, and on the system's interrupt signals (on POSIX, SIGINT and SIGTERM); a forced kill (SIGKILL on POSIX) and power loss leave a stale lock. It holds the lock file open from creation to removal so the inode cannot be reused. Before removing, it checks that the path is still its own file by comparing the file identity the system provides (on POSIX, device and inode: `stat` on the path against `fstat` on the open file), never by reading the pid, host or time. If they differ, or the open file's link count is zero, it removes nothing and reports that the lock was removed by someone else during the operation. A gap remains between the check and the removal; it matters only when someone removes a lock that is in use, which the rule above forbids. The identity check and the removal are not safe to run inside a signal handler, so the handler does nothing but wake an ordinary thread, which does the work; the process then ends by the signal itself rather than with an exit code, because a run that was interrupted must not look to its caller as though typdoc decided something. A second interrupt does not cut the cleanup short: it runs once, further deliveries wait for it, and the wait is an identity check and an unlink for each lock held. `SIGKILL` stays immediate and uncatchable.

- **Pins.** `lock.json` and `vendor/` belong to the project, not to a namespace, so a command that writes them (`pull`, or the first fetch of a URL with no pin) takes a project lock, `locks/.project.lock` (in `git-common` mode `<project-hash>.lock`). Its name starts with `.`, so no namespace can have it, and it follows the same rules as every other lock. A command never takes it while holding a namespace lock; one that needs both takes the project lock first, then the namespace locks in the order `docs/design/spec/SPC-10.md` gives, so two commands cannot deadlock. Network fetches happen before any lock is taken. After taking the project lock the command re-reads `lock.json`: if a pin now exists it uses that, and if its bytes differ from what was just fetched it reports the difference and never overwrites it silently. A command that fetched a schema writes the pin and releases the project lock before it takes a namespace lock.
- **What v1 does not do.** It does not stop another party from removing a lock typdoc holds; it makes the affected writer notice and report. Not showing how to remove a lock while its owner is running is a way of not inviting it, not a mechanism: the path is in the message and in this document. There is no unlock command, no process start time in the lock and no random token, since all of those serve removal or takeover by the program, which v1 does not do.

| `lock` | Lock path | Use when |
| --- | --- | --- |
| `local` | `<project>/.typdoc/locks/<namespace>.lock` | All sessions share one working tree |
| `git-common` | `$(git rev-parse --git-common-dir)/typdoc/<project-hash>-<namespace>.lock` | Sessions run in separate git worktrees |

`git-common` stops two worktrees writing at the same instant, but each worktree still holds its own copy of the files: two worktrees can allocate the same key, and `validate` catches the duplicate after merge. Each worktree also holds its own copy of `state/<namespace>.json`. Worktrees that work on different namespaces never change the same state file, so their numbering cannot collide. Two worktrees on the same namespace still change the same line and conflict on merge, a louder signal than the duplicate key alone. Truly shared state across worktrees needs the project folder outside the worktrees.

**`<project-hash>`.** In `git-common` mode the lock file sits in the repository's common directory, which every worktree shares, so its name has to tell two projects apart without depending on where a worktree happens to be. `<project-hash>` is the SHA-256 of the project folder's path relative to the root of the worktree, truncated to its first 16 hexadecimal characters. The path is brought to one form before it is hashed: separators are `/`, and a trailing separator is removed. The absolute path cannot be used, because it is exactly what differs between worktrees, which is the case this mode exists for, while the path relative to the worktree root is the same in every worktree of the repository. A worktree that keeps the project at a different relative path is a different project under this rule and takes a different lock; that is the intended reading, not an accident of the formula. `git rev-parse --git-common-dir` may answer with a relative path, so it is canonicalized before a lock path is built from it.

Sixteen hexadecimal characters are 64 bits, so two projects in one repository can in principle hash the same. What follows from it is that they share a lock and wait for each other: nothing is corrupted and nothing is lost. That is accepted knowingly rather than guarded against.

The hash is stable across versions of typdoc. Changing how it is computed would make a new binary blind to a lock an old binary is holding, so it is a breaking change and is treated as one.

## JSON output

**A field written with no value** is `null` in `--json`, and one written as an empty string is `""`. The output says what the file says, for the same reason a `number` carries its digits: the caller is told what is there, not what typdoc would have made of it. Every rule and every command treats the two alike, so nothing else in the output moves.

**A `number` in `--json`** is printed with the digits written in the document, not with a value converted from them. JSON puts no limit on the digits of a number; the readers do, each in its own way, and a reader that cannot hold one rounds it knowingly from a true value instead of being handed a different one. The reason is the same one that makes the frontmatter reader keep text: nothing between the file and the caller decides what `1e3` is. Converting first loses more than digits. Two documents whose numbers differ by one print the same value and cannot be told apart, and `1e3` becomes `1000.0`, which is not what the file says. A field of any other type is printed as it always was, and a `string` holding the same digits has never been affected.

`path` is the path of the file relative to the folder of the project the document belongs to, the folder that holds that project's `.typdoc`, so it is the same in every namespace and can be opened as it stands from there. `namespace` is not redundant with it: the namespace `default` has no folder of its own, so the paths of its documents contain no namespace name and none can be recovered from them, and that is the commonest case. Where a project has several namespaces, two documents in different ones can have the same path below their namespace folder, and the namespace tells them apart from the path's first segment onward.

**A finding** is the same object in the report of `validate` and in the `details` of an error object. It always has `rule`, `level` and `message`. It has `path`, the file it is about, relative to the project folder, for a document and for a configuration file alike; `namespace`, `collection` and `key` when the file is a document (`collection` when it is in one, `key` for a coded one only); `field` when it is about one field; and `line` and `col`, 1-based, when a position is known. A finding is always located in a file of the project being checked, never in one of an imported project: a broken ref is a fault of the document that holds it, not of its target.

| Command | Prints |
| --- | --- |
| `get` | `{ "document": <document> }` |
| `list` | `{ "documents": [<document>, ...], "total": n, "truncated": b }` |
| `validate` | `{ "summary": {...}, "findings": [<finding>, ...] }`, and with `--audit` also `"audit": {...}` |
| `refs` | `{ "document": <name>, "direction": "out" or "in", "refs": [<reference>, ...] }` |
| `toc` | `{ "document": <name>, "headings": [{ "level", "text", "slug", "line", "end" }, ...] }` |
| `new` | `{ "document": <document> }` |
| `set` | `{ "document": <document> }` |
| `mv` | `{ "document": <document>, "unrewritten": [<reference>, ...], "findings": [<finding>, ...] }` |

```json
{ "summary": { "scope": "all", "strict": false,
               "checked": { "namespaces": ["default"], "documents": 214 },
               "findings": { "error": 3, "warn": 12, "info": 0 } },
  "findings": [ { "level": "error", "rule": "body.links", "message": "...",
                  "path": "tickets/WF-7.md", "namespace": "default", "key": "WF-7", "line": 8, "col": 5 } ] }
```

**Headings.** In `toc`, `document` is the name of the document asked about, and each heading has its `level`, `text`, `slug` and `line`, the line of the heading, counted as `docs/design/spec/SPC-1.md` gives. `end` is the last line of the heading's section: the section runs to the line before the next heading of the same or a shallower level, or to the last line of the file, and it includes the sections of the headings under it. A heading with nothing under it has `end` equal to `line`. The ranges therefore nest and do not tile the file: the range of a heading contains the ranges of the headings under it, so reading every range in turn reads some lines more than once. `end` is a property of the document: `--depth` chooses which headings are listed and never changes the `end` of one that is. Headings are in the order of `line`, and the order is guaranteed.

A reference that does not resolve has no `path` and has `unresolved` instead, one of three values: `not-found`, the place the ref names is present and the file or key is not; `import-absent`, the import it names is not on this machine, which `imports.absent` reports; and `bad-prefix`, the prefix names no namespace and no import, or names a project with several namespaces without saying which. A missing `path` alone would make a broken link and a machine that has not been set up look the same, and they are different problems with different fixes. `path` and `unresolved` never appear together, and `unresolved` occurs only for `out`, since a reference read from a document that holds it has been found. Unresolved references are listed: a ref is counted from what is written in the field. `--field` keeps only the refs in that field, `$body` for body links, and it means the field that holds the ref in both directions, so with `--reverse` it is a field of the document that holds it.

The order of `refs` is guaranteed. For `out` it is the fields in the order they appear in the document, then `$body` by position, and within a field the values in the order they are written. For `in` it is the documents that hold the refs, those of this project first and then those of imported projects by alias, each by `path` as under Order, and then as for `out`.

**An `--if` that is false** exits 3, writes nothing, and prints the error object on standard error, as every non-zero exit does. One rule a caller can rely on — a zero exit puts the result on standard output, a non-zero one puts the error object on standard error — is worth more than marking this case as a success because nothing went wrong. Its `details` name the condition that was false, which a caller that gave several cannot otherwise work out.

**`mv`.** `document` is the document under its new name, so a `--renumber` reports the key it issued in the field every other shape carries a key in. The old name is not repeated: the caller gave it. `unrewritten` holds one entry for every ref that still points at the old name, in the reference object, from the direction `refs --reverse` takes: the name is the document that holds the ref, with `field`, `written` and a position. Each also has `reason`, one of `imported-project` for a ref held by a project this one imports, which is read-only; `mention` for plain text that the `body.mentions` rule checks and `mv` does not rewrite; and `links-rule-off` for a body link in a document whose `body.links` rule is off. A count of the refs that were rewritten is not printed: it is a fact about the command, and a caller that needs it can read the document's refs. `findings` carries what the destination's schema rejects when a move lands a document in a collection whose schema it does not satisfy: the move is carried out, the exit code is 0 because the world changed, and this is the field to branch on rather than the code. They are the finding object `validate` prints, so one consumer reads both. A `mv` that stops partway is not this shape at all: it is an error, on standard error, saying what was done and that running the same command again finishes it.

**Order.** The order of `findings` is guaranteed: by `path`, then `line`, `col`, `rule` and `message`, a finding with no position before one with a position in the same file. `path` is compared as it is under Sorting: lexicographically, by the bytes of the path. This differs from `list` on purpose. `findings` are ordered by position in a file, so the path decides; `list` orders documents, and a document's key contains a number, which is compared as a number. They order different things, and making them agree would leave the order of findings depending on the numbers in keys. In `audit`, `collections` is sorted by name and the two lists as `path` is. In `mv`, `findings` is ordered the same way, and `unrewritten` is ordered as `refs --reverse` is: the documents that hold the refs, those of this project first and then those of imported projects by alias, each by `path` as under Sorting, and within a document the fields in the order they appear, then `$body` by position.

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
