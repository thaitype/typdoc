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
| Namespace | A folder containing `.typdoc/config.json`. A file belongs to the nearest namespace above it. Keys, collection names and schema names are unique within a namespace. | `chief` at `.chief/`, `memory` at `typmem/memory/` |
| Collection | Files in a namespace matching one pattern, sharing one schema; defined by one file in `.typdoc/collections/` | `learnings/*.md` |
| Schema | JSON definition of fields and rules, optionally with a `code` | `wayfinder.json` (`code: WF`) |
| Document | One `.md` file | `learnings/never-send-secrets-over-ship.md` |
| Key | `{code}-{number}`, the identity of a document whose schema has a code | `WF-3` |
| Path | The identity of a document whose schema has no code | `precedents/secret-handling.md` |
| Ref | A pointer to another document, from a frontmatter field or a body link | `blocked_by: [WF-1]` |

Before any query or validation, `typdoc` builds one index mapping every key and every path to its file. Refs of either kind resolve through it, so a keyed ticket can point at a path-identified note and the reverse.

**Collection vs schema.** A collection selects files; a schema describes their shape. One schema without a `code` may serve several collections (`notes` and `drafts` both using `note.json`), so anything about choosing documents uses the collection: `list --collection`, the `collection` pseudo-field, a collection's `validation`. Anything about data shape uses the schema: types, fields, `extends`, and a ref field's `target`. `target` names schemas rather than collections because a schema, possibly published remotely, cannot know what a given namespace calls its collections.

## Config: .typdoc/config.json

Each namespace has one `.typdoc/config.json` that names the namespace, sets namespace-wide options and optionally imports other namespaces, plus one file per collection in `.typdoc/collections/` that maps files to a schema.

```json
{
  "version": 1,
  "name": "chief",
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
| `name` | yes | This namespace's name, the prefix other namespaces use in namespaced references and qualified schema names |
| `imports` | no | Name → folder of another namespace. Paths may use environment variables. See Refs → Across namespaces. |
| `validation` | no | Rule levels and options that apply namespace-wide, under `global`. A collection tunes them in its own file. See Validation rules. |
| `lock` | no | `local` (default) or `git-common`. See Concurrency. |

**Collection files.** Each collection is one file, `.typdoc/collections/<name>.json`. The file name without `.json` is the collection's name: ASCII letters, digits, `-` and `_`, so it is unique by construction. The file maps files to a schema and holds that collection's state:

```json
// .typdoc/collections/wayfinder.json
{
  "match": "tickets/{key}.md",
  "schema": "schemas/wayfinder.json",
  "validation": { "body.mentions": { "level": "error" } },
  "last": 12
}
```

| Key | Required | Meaning |
| --- | --- | --- |
| `match` | yes | Which files belong to the collection. See Match templates. |
| `schema` | yes | Relative path or `https://` URL of the schema. See Remote schemas. |
| `refBase` | no | How frontmatter paths resolve: `file` (default, relative to the document) or `namespace` (relative to the namespace folder) |
| `validation` | no | Rule levels and options for this collection only, merged over `validation.global`. See Validation rules. |
| `last` | no | Coded schemas only. The highest number `typdoc new` has allocated in this collection. Written by `typdoc new`, not by hand; it is what stops a number being reused after its document is deleted. |

**Loading.** Every `*.json` file in `.typdoc/collections/` is a collection; other files are ignored. A file that cannot be parsed, has an unknown key or names a schema that does not exist is a config error that names the file, and `typdoc` stops rather than skip it, because a skipped collection would silently shrink every result. Collections have no order; anything that lists them sorts by name. A document matched by two collections is an error (`collections.overlap`), never settled by precedence.

**Writing `last`.** `typdoc new` replaces only the number, in place, then re-parses the result and compares it with what it meant to write before renaming the temp file over the original, so it cannot corrupt a collection file.

**Match templates.** For a coded schema, `match` is a template with placeholders; for a schema without a code, it is a glob.

| Placeholder | Stands for | Allowed in |
| --- | --- | --- |
| `{key}` | `{code}-{number}`, with the code taken from the schema, e.g. `WF-3` | Coded schemas; exactly once, no globs |
| `{slug}` | A slug derived from the title, e.g. `cosmos-or-sql` | Coded schemas, alongside `{key}` |
| `*`, `**` | Glob | Schemas without a code; no placeholders |

One template serves both directions: it decides which files belong to the collection, and `typdoc new` uses it to name new files. Because `{key}` includes the schema's code, several coded collections can share one template in one folder: `tickets/{key}.md` matches `WF-3.md` for one schema and `RFC-4.md` for another. Each code counts on one number sequence of its own, so a coded schema serves exactly one collection; two collections naming the same coded schema is a config error, as is a `last` on a collection whose schema has no code.

**Discovery.** `typdoc` finds its namespace by walking up from the current directory, or from a document path given as an argument, to the nearest folder containing `.typdoc/config.json`. `--dir <path>` overrides it. Config, collection files and local schemas are read on every run with no cache; remote schemas are read from their pinned copies (see Remote schemas).

**Nested namespaces.** A collection's `match` never crosses into a nested namespace: a file with a nearer `.typdoc/config.json` always belongs to that namespace.

**The .typdoc folder.** Everything typdoc reads as configuration or writes for itself lives in one folder at the top of the namespace; the folder is also what marks a namespace.

```
.typdoc/
  config.json    config (this section)            commit
  collections/   one file per collection          commit
  lock.json      pins for remote content          commit
  vendor/        pinned copies, e.g. schemas/     commit
  write.lock     mutex held while writing         .gitignore
```

`lock.json` has no version of its own (the `version` in `config.json` covers every file typdoc owns) and is split into sections so pins other than schemas can join later without a rename:

```json
{
  "schemas": {
    "https://schemas.example.dev/chief/wayfinder/v1.json": {
      "sha256": "9f2c…", "path": "vendor/schemas/3a7e1c.json", "fetchedAt": "2026-09-19T14:30:00+07:00"
    }
  }
}
```

There is one config location. A legacy `.typdoc.json` beside the folder is a config error that says to move it to `.typdoc/config.json`, rather than being read silently.

**Machine-specific imports.** When an imported namespace's location differs per machine, put it in an environment variable or in `~/.config/typdoc/imports.json`, which is merged under the namespace's own `imports`, so no machine-specific path is committed.

## Schema format

A schema is a JSON file with a name, an optional code, an optional parent, and its fields. The format is typdoc's own; it is not JSON Schema, and no JSON Schema tool reads it.

**Top-level keys**

| Key | Required | Meaning |
| --- | --- | --- |
| `name` | yes | Schema name, unique within its namespace |
| `code` | no | `[A-Z][A-Z0-9]*`. Present: documents get keys and `typdoc new` allocates numbers. Absent: documents are identified by path. |
| `extends` | no | Parent schema: a relative path or an `https://` URL. Chains allowed; cycles rejected. A parent is usually abstract (no `code`, not used by any collection). |
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
| `auto` | `date`, `datetime` | `create`: set by `new`, never changed after. `update`: set by `new` and by every `set` that changes a value. |
| `override` | all | Required to redefine an inherited field |

**Auto fields.** Field names carry no meaning; only `auto` does. A field such as `created_at` without `auto` is an ordinary field that `typdoc` never fills. `set` may not write an `auto` field directly. Because edits made outside `typdoc` (by hand, or a file tool changing the body) are invisible to it, an `auto: update` field means "frontmatter last changed through `typdoc`", not "file last modified".

**Target names.** A bare name (`"learning"`) means a schema in this namespace. A qualified name (`"memory:learning"`) means a schema in the imported namespace `memory`. Qualified names live only in schema JSON, never in Markdown files. `"*"` also accepts files outside any collection, such as a README.

**Extends rules.** New fields merge in. Redefining an inherited field without `"override": true` is a schema error, so a child cannot silently change what a shared field means. Sibling schemas may define same-named fields independently.

**Remote schemas.** Anywhere a schema is referenced, in a collection's `schema` or in `extends`, an `https://` URL to a JSON file works as well as a path. A workflow can publish its schemas so users need not write them:

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

- **Pinned, not live.** The first time a URL is needed, `typdoc` fetches it, stores a copy under `.typdoc/vendor/schemas/`, and records its SHA-256 in `.typdoc/lock.json`. Every later run reads the stored copy, so results never change because a server did, and runs work offline. Commit both so CI and teammates use the same bytes. `typdoc pull` re-fetches on purpose.
- **Relative references** inside a remote schema, such as its own `extends`, resolve against its URL.
- **Only `https://`.** Plain `http://` is refused. A schema is JSON data; nothing in it is executed.
- **Names.** Only schemas used directly by a collection share the namespace's names, since those are the names `target` refers to. A parent reached only through `extends` does not, so a local schema may reuse its parent's name, as above. Two collection schemas with one name are reported by `schema.valid`.
- **Versioning** is the publisher's job: put the version in the URL (`.../v1.json`), so a breaking change is a new URL that users adopt deliberately.

**Targets in published schemas.** An import name belongs to the user's namespace, so a published schema should not name one in `target` (`"memory:precedent"`). Publish `"*"` or bare schema names, and let users narrow `target` in a local schema that extends the published one, using `"override": true`.

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
    "context": { "type": "ref[]", "target": ["memory:precedent", "memory:learning"] }
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

A document is YAML frontmatter plus a free Markdown body; `typdoc` owns only the frontmatter.

```markdown
---
title: Cosmos or SQL?
kind: grilling
status: open
blocked_by: [WF-1]
context: [memory:precedents/secret-handling.md]
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
| With `code`, slug included | `decisions/{key}-{slug}.md` | `decisions/DEC-7-cosmos-or-sql.md` |
| Without `code` | `learnings/*.md` | `learnings/never-send-secrets-over-ship.md` |

`{key}.md` is the recommended form because a body link to it never breaks: the name does not change when the title does. Including `{slug}` reads better in `ls`, at the cost of `mv` on retitle. Either way, keys resolve to files, and two files with the same key are a validation error.

**Rules**

- The key is not stored in frontmatter; the filename is the single source of truth.
- `title` is an ordinary frontmatter field, not the H1. The body has no required structure.
- Writes preserve key order and existing YAML style where possible, and never reformat the body.
- Frontmatter fields not in the schema are kept on write and reported by `validate`.

## Refs

Refs come from two places, frontmatter fields and body links, and both resolve through the same index.

**Frontmatter values.** A `ref` or `ref[]` value is a plain string, read in this order:

| Value | Read as | Condition |
| --- | --- | --- |
| `WF-3` | Key | Matches `^[A-Z][A-Z0-9]*-\d+$` and the code exists in a reachable namespace |
| `memory:precedents/x.md` | Namespaced reference: a path inside the imported namespace `memory` | `memory` is listed in `imports` |
| anything else | Relative path | Resolved from the document (`refBase: file`) or the namespace folder (`refBase: namespace`) |

If nothing is imported, nothing is ever read as a namespaced reference. Import names may not collide with URL schemes (`http`, `https`, `mailto`, `file`); `validate` enforces this.

**Body links.** Only standard Markdown links count: `[text](path)` and `[text](path#heading)`, with paths relative to the document. Links starting with a URL scheme (`https:`, `mailto:` and so on) that is not an import name are always skipped; no configuration is needed. Links inside fenced code blocks and inline code are not links. Relative paths that should not be checked (images, generated files) go in the `ignore` option of the `body.links` rule. Plain-text mentions are never refs; the `body.mentions` rule can check that they exist (see Validation rules). Body links are exposed to queries as the virtual ref field `$body`.

**Canonical form.** A coded document should be referenced by key in frontmatter; referencing it by path works but `validate` warns, since a slug path can change. Body links always use paths.

**Write-time checks** (`new`, `set`): the target exists; its schema is allowed by `target`; no cycle forms on `acyclic` fields.

**Across namespaces.** A relative path that leaves the namespace, or a namespaced reference, lands in another namespace. `typdoc` finds that file's nearest `.typdoc/config.json` to learn its schema. Forward traversal needs no configuration. Reverse lookup (`refby`, `refs --reverse`, `mv`) scans only this namespace and the namespaces it imports. Imports are one-way: `chief` importing `memory` does not let `memory` see `chief`.

|  | Without imports | With imports |
| --- | --- | --- |
| Forward refs and validation across namespaces | yes | yes |
| `refby` sees refs from other namespaces | no, own namespace only | yes, from imported namespaces |
| `mv` rewrites refs in other namespaces | no, warns where known | yes, preserving each ref's written form |

**Ownership rules** (so two namespaces never conflict):

1. A file is validated only by the namespace that owns it. Another namespace checks only that the target exists, matches `target`, and has the linked heading.
2. `match` stops at a nested namespace's boundary.
3. A bare schema name in `target` always means this namespace.
4. Imports are followed one level; imports of imports are ignored.
5. Writes stay inside the owning namespace, except `mv`.

**Schema drift.** If an imported namespace renames a field that this namespace queries through a ref, queries silently return nothing. `validate --schemas` checks that every field used across namespaces still exists in the imported schemas; run it in CI when namespaces live in different repos.

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
- **Pseudo-fields** on every document: `path`, `key` (coded only), `code`, `collection`, `schema`, `namespace`. `$body` is a virtual ref field holding body links. A reached document in another namespace reports that namespace's collection name. These names are reserved: `schema.valid` rejects a schema field that uses one, or any name starting with `$`. The list is closed; adding a pseudo-field later is a breaking change. `$body` is valid only as `f` inside `ref.*(f)` and `refby.*(f)`.
- **Reached documents** are read under their own schema. A reached document lacking the field, or a dangling ref, counts as absent (see Absence and negation); dangling refs also warn on stderr.
- **Absence and negation.** `k!=v` is exactly NOT `k=v`, in every form (single value, list, glob, array). Something absent, whether a document without the field or a dangling ref, fails every positive condition (`=` in any form, `k=*`) and satisfies every `!=`. The ordering comparisons (`<`, `<=`, `>`, `>=`) are the exception: absent fails them. This keeps paired queries complementary: `ref.all(blocked_by).status=resolved` and `ref.any(blocked_by).status!=resolved` split the open tickets between them, and none falls through. Because `k!=v` includes documents whose schema has no `k`, use it with `--collection` (or add `k=*`) when a query spans collections.
- **Values and escaping.** In a value only three characters are special: `,` (separates alternatives), `*` (glob) and `\`. Put `\` before one to mean it literally: `title=Cosmos\, or SQL`, `k=\*`. `\` before any other character, or at the end of a value, is an error, which catches typos and leaves room to add special characters later. `*` is the only glob; there is no `?` and no `[...]`. `k=*` means "present" and `k=\*` a literal star. `=`, `<`, `>` and `!` need no escape in a value, because the expression is split at the first operator after the field name, taking the longest of `!=`, `<=`, `>=`, `=`, `<`, `>`. The same rules apply to `--where`, `--if` and `--set`, except that `--set` splits a value on `,` only for array fields. Wrap the whole expression in single quotes so the shell leaves `\`, `*`, `<` and `>` alone.
- **Names and scope.** A field name is `[A-Za-z_][A-Za-z0-9_-]*`, and `schema.valid` holds schema fields to the same rule, so every field can be queried. A field name unknown to every schema in scope is an error, not an empty result. For a plain condition the scope is the collections chosen with `--collection` or `--code`, or every collection in the namespace when none is chosen; a document whose schema lacks the field counts as absent. In `ref.*(f)` and `refby.*(f)`, `f` must be a field of type `ref` or `ref[]`, or `$body`, defined in a schema of this namespace or one it imports. The scope of the condition after `ref.*(f)` is the schemas named by `f`'s `target` (every schema in this namespace and its imports when the target is `"*"`); after `refby.*(f)` it is the schemas that define `f`; for `$body` in either it is every schema in this namespace and its imports.
- **Syntax.** An expression is read whole, as one argument: spaces belong to names and values, so `status = open` is an error, with the hint `did you mean status=open?`. Field names, values, globs and enum values are case-sensitive. An empty value is an error in `--where` and `--if` (use `k!=*` to test for absent or empty); in `--set`, `k=` removes the field. On an array field `=` means some element matches and `!=` means no element does. The condition after `ref.*(f).` is a plain condition; another `ref.*` inside it is an error, as is anything after `)` that is not `.EXPR`. The ordering comparisons take one value, so `k<a,b` is an error. In a list, each value is coerced by the field's type on its own, and one that cannot be coerced makes the whole expression an error. In `--set`, an unescaped `*` in a value is an error (write `\*` for a literal star), and `,` in the value of a scalar field is an ordinary character.
- **Coercion.** Values are coerced by schema type. A value outside an `enum` is an error (except with globs), so typos fail loudly.
- **Comparisons** (`<`, `<=`, `>`, `>=`) apply to `number`, `date` and `datetime` only; on any other type they are an error. `datetime` values compare as instants, offsets included. A date-only value compared with a `datetime` field compares against the field's date part. A document without the field satisfies no ordering comparison. Always quote the expression: `<` and `>` are shell redirections.
- **Rule of thumb.** "Does such a document exist" → `any`; "is nothing in the way" → `all`; "is there none" → `none`. For a single `ref` field prefer `any`, since `all` is true when the field is empty.
- **v1 limits.** One hop, no OR across fields. All `--where` conditions are ANDed.

## Commands

Nine commands cover the lifecycle; `new`, `set` and `mv` write documents, `pull` writes pinned schemas, and `new` also records the number it allocated as the collection's `last`. Every document argument accepts a key (`WF-3`) or a path.

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

All commands accept `--json` and `--dir`.

### typdoc new

```bash
typdoc new <CODE> "<title>" [--set k=v ...]      # coded schema: prints the new key
typdoc new <path> [--set k=v ...]                  # path-identified schema
typdoc new WF "Cosmos or SQL?" --set kind=grilling --set blocked_by=WF-1
# stdout: WF-3
```

For a code, allocates the next number under the lock: the larger of the highest existing number in the collection and the collection's `last`, plus one, then records it as the new `last`. A number is never reused after its document is deleted, and a file created by hand with a higher number is respected; the only way the two sources can disagree is by leaving a gap, which is harmless. The file is named from the collection's `match` template. For a path, the path must match a collection. Fills defaults and `auto` fields, validates, then writes. Array values are comma-separated.

### typdoc get

```bash
typdoc get <key|path> [--json]
```

Returns frontmatter plus `path`, `key`, `code`, `collection`, `schema` and `namespace`.

### typdoc list

```bash
typdoc list [--collection c[,c]] [--code C[,C]] [--where EXPR ...] [--fields f,...]
            [--sort field[:asc|:desc] ...] [--limit n] [--ids] [--json]
typdoc list --collection wayfinder,decisions --where status=open --sort status --sort updated_at:desc
```

`--collection` selects by collection name; `--code` is a shorthand that selects the collections whose schema has that code. Expressions are listed under Query. Default output is a table of key or path, `title`, and every field used in `--where`. `--json` returns `{ path, key, code, collection, schema, namespace, fields }` per document with all frontmatter. `--ids` prints one key or path per line. An empty result exits 0.

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

Lists body headings with line ranges counted from the top of the file, frontmatter included, so they match editor and file-tool line numbers. Headings inside fenced code are ignored. `--json` returns `{ level, text, slug, line, end }`; `slug` is what a `#heading` link must use.

### typdoc refs

```bash
typdoc refs <key|path> [--field f|$body] [--reverse] [--json]
typdoc refs precedents/secret-handling.md --reverse
# chief:WF-7   context
# learnings/x.md   $body
```

Outgoing refs by default; `--reverse` scans this namespace and the namespaces it imports.

### typdoc mv

```bash
typdoc mv <from> <to>
```

Moves a file and rewrites every ref to it that is visible from this namespace, in frontmatter and body links, keeping each ref's written form (key, namespaced reference or relative path). Takes the lock of every namespace it writes. For a coded document, `<to>` must keep the key. Namespaces that cannot be reached are reported with the refs left to fix.

### typdoc pull

```bash
typdoc pull [<url> ...] [--check]
```

Re-fetches remote schemas (all of them, or the URLs given), including remote parents they extend, and updates `.typdoc/vendor/schemas/` and `.typdoc/lock.json` under the namespace's lock. Prints which URLs changed. It then validates every document against the new schemas and reports what the change breaks, without rolling back. `--check` fetches and compares only, writes nothing, and exits 2 if any pin is stale, which suits CI. Other commands never touch the network except to fetch a URL that has no pin yet.

### typdoc validate

```bash
typdoc validate [<key|path> ...] [--schemas] [--strict] [--audit]
```

- **Schemas:** duplicate names or codes, `extends` cycles, undeclared overrides, invalid options, field names that break the naming rule or use a reserved name, import names colliding with URL schemes.
- **Documents:** types, required fields, enum values, unknown fields, duplicate keys, files not fitting the collection's `match` template.
- **Refs:** missing targets, disallowed target schemas, missing `#heading` anchors, cycles on `acyclic` fields, coded documents referenced by path (warning).
- **Across namespaces:** a ref into an imported namespace that is absent on this machine is a warning; a present namespace missing the file is an error. `--strict` makes both errors.

`--schemas` checks schemas only, including fields used across namespaces (schema drift). Suited to pre-commit, CI and agent post-edit hooks.

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

A typical adoption: run `--audit`, adjust `match` until every intended file is covered, adjust schemas or fix files until the summary is clean, then turn on plain `validate` in CI. `--json` returns the summary and every finding.

## Validation rules

Correctness rules are always on; quality rules are configured namespace-wide under `validation` in `config.json`, and per collection under `validation` in its collection file.

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
| `schema.valid` | Duplicate names or codes, `extends` cycles, undeclared overrides, invalid options, field names that break the naming rule or use a reserved name, import names colliding with URL schemes |
| `frontmatter.types` | Types, required fields, enum values |
| `frontmatter.transitions` | State changes follow `transitions` (checked on write) |
| `refs.resolve` | Frontmatter refs point at existing files |
| `refs.target` | Ref targets match the field's `target` |
| `refs.acyclic` | No cycle on `acyclic` fields |
| `keys.unique` | No two files share a key |
| `collections.overlap` | No file is matched by two collections |

**Configurable**

| Rule | Default | Options | Checks |
| --- | --- | --- | --- |
| `body.links` | `error` | `ignore` (globs of relative targets to skip) | Markdown links in the body point at existing files |
| `body.anchors` | `error` | — | `#heading` in a link exists in the target |
| `body.mentions` | `off` | `inlineCode` (`true`), `fencedCode` (`false`) | Keys mentioned in body text exist |
| `refs.codedByPath` | `warn` | — | A coded document is referenced by path instead of key |
| `frontmatter.unknown` | `warn` | — | Frontmatter fields not in the schema |
| `filename.pattern` | `error` | — | A file in a coded collection's folder that fits no `match` template, e.g. `tickets/README.md` |
| `imports.absent` | `warn` | — | Refs into an imported namespace that is absent on this machine |

**body.mentions.** Checks plain-text keys; it never turns them into refs, so `refby` and `mv` ignore mentions. The codes to look for come from the schemas of this namespace and the namespaces it imports; nothing is listed in config.

| Text | Checked |
| --- | --- |
| `see WF-3` | yes |
| `` `WF-3` `` (inline code) | per `inlineCode` |
| Inside a fenced code block | per `fencedCode` |
| `[WF-3](WF-3.md)` | no; `body.links` checks it |
| `UTF-8`, `SHA-256` | no; not a known code |
| `WF-3a`, `xWF-3` | no; word boundaries required |

A mention is looked up in this namespace and the namespaces it imports: no match reports *not found*; more than one reports *ambiguous*; an imported namespace absent on this machine falls under `imports.absent`. `fencedCode` defaults to `false` because code blocks often hold logs, commands and diffs that contain key-like text.

**Config errors** (reported when the config loads): a missing or unknown `version`; a collection file that cannot be parsed, has an unknown key, names a schema that does not exist, or has a name outside ASCII letters, digits, `-` and `_`; an unknown rule name or option; any attempt to configure an always-on rule; a `match` template breaking the placeholder rules; a `last` on a collection whose schema has no code; two collections naming the same coded schema; a schema URL that is not `https://`; a remote schema with no pin that cannot be fetched; a pinned copy whose SHA-256 no longer matches `lock.json` (the copy was edited by hand; run `typdoc pull` to restore it).

**Output.** One line per finding, `path:line:col  level  message  rule`; `--json` returns the same fields.

```
tickets/WF-7.md:8:5              error  WF-3 ambiguous: chief, memory   body.mentions
learnings/proc-environ.md:14:22  warn   WF-9 not found                  body.mentions
drafts/idea.md:3:10              warn   link target missing: ../x.md    body.links
```

## Concurrency

One lock per namespace serializes writes, which is enough for number allocation, compare-and-set and `mv`.

- **Lock.** Created with `O_EXCL`, holding pid, hostname and timestamp. Retries with backoff until a timeout (default 5 s, `--lock-timeout`), then exits 4.
- **Stale locks.** A lock whose pid is dead on the same host, or older than 30 s, is taken over.
- **Atomic writes.** Each file is written to a temp file in the same directory, then renamed over the original.
- **What is locked.** `new` holds its namespace's lock from reading `last` to writing the file and updating `last`. `set` holds it across read, `--if`, validation and write. `mv` takes the lock of every namespace it writes, in path order, so two `mv`s cannot deadlock. Reads never lock.

| `lock` | Lock path | Use when |
| --- | --- | --- |
| `local` | `<namespace>/.typdoc/write.lock` | All sessions share one working tree |
| `git-common` | `$(git rev-parse --git-common-dir)/typdoc/<namespace-hash>.lock` | Sessions run in separate git worktrees |

`git-common` stops two worktrees writing at the same instant, but each worktree still holds its own copy of the files: two worktrees can allocate the same key, and `validate` catches the duplicate after merge. Each worktree also holds its own copy of the collection's `last`, so two branches that both allocate the next number change the same line and conflict on merge, a louder signal than the duplicate key alone. Truly shared state across worktrees needs the namespace folder outside the worktrees.

## Exit codes and errors

Exit codes let an agent branch without parsing text.

| Code | Meaning |
| --- | --- |
| 0 | Success, including an empty `list` result |
| 1 | General error: not found, bad arguments, I/O |
| 2 | Validation failed: schema, type, enum, transition or ref |
| 3 | An `--if` condition was false; nothing written |
| 4 | Lock not acquired within the timeout |

Errors go to stderr. With `--json`, stderr carries one object:

```json
{ "error": "transition not allowed: open -> resolved", "code": 2,
  "details": [{ "doc": "WF-3", "field": "status", "rule": "transitions" }] }
```

## Worked examples

Two namespaces: `chief` holds wayfinder and implementation tickets, `memory` holds typmem learnings, precedents and proposals, and `chief` imports `memory`.

### Wayfinder tickets (namespace: chief)

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

### typmem memory (namespace: memory)

Existing precedents write `sources` relative to the memory namespace folder (`learnings/...`), so each collection sets `"refBase": "namespace"` and no file needs editing.

```json
// .typdoc/config.json
{ "version": 1, "name": "memory" }
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

### Across the two namespaces

From `chief`, a ticket points into memory with a namespaced reference in frontmatter and an ordinary relative link in its body:

```yaml
context: [memory:precedents/secret-handling.md]
```

```bash
# open decision tickets that rely on at least one memory precedent
typdoc list --collection wayfinder --where status=open --where 'ref.any(context).collection=precedents'
```

`memory` does not import `chief`, so running `refs --reverse` from memory does not see the ticket; running it from `chief` does, because `chief` scans its own namespace.
