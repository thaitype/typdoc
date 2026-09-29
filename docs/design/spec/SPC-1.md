---
title: Validation rules explained
status: active
migrated_from: docs/archived-design/design.md#validation-rules
follows: [PRN-99, PRN-3, PRN-6, PRN-7, PRN-12]
---

Every rule `typdoc validate` can report falls into one of two groups.

**Always on.** Fifteen rules — `schema.valid`, `frontmatter.parse`, `frontmatter.types`,
`frontmatter.transitions`, `refs.resolve`, `refs.target`, `refs.acyclic`, `keys.unique`,
`collections.overlap`, `collections.empty`, `state.missing`, `state.malformed`, `state.behind`,
`state.retired`, and `files.unreadable` — cannot be turned off or reconfigured. Queries and
writes depend on the correctness they guarantee, so there is no `validation` key that changes
their level. `collections.empty` fires once, at `warn`, when the project has no collections at
all, regardless of what else `.typdoc/` holds — it is a finding of `validate`'s, not a config
error, so it never stops another command from running.

**Configurable.** Ten rules — `body.links`, `body.anchors`, `body.mentions`, `refs.codedByPath`,
`refs.moved`, `refs.slug`, `names.shadowed`, `frontmatter.unknown`, `filename.pattern`, and
`imports.absent` — have a default level and can be raised, lowered, or turned off project-wide under
`validation.global` in `config.json`, or per collection in that collection's own file. `--strict`
raises every remaining `warn` to `error`.

`docs/design/catalog/rules.md` holds the machine-readable form of this same list: one entry per
rule id, each carrying `configurable: true` for a rule in the second group and `configurable:
false` for a rule in the first. This document explains why the split exists; that one is what
code and tests read.

## Default levels

| Rule | Default | Options | Checks |
| --- | --- | --- | --- |
| `body.links` | `error` | `ignore` | Links in the body point at existing files (below) |
| `body.anchors` | `error` | — | `#heading` in a link exists in the target (percent-decoded, case-insensitive; `SPC-14`) |
| `body.mentions` | `off` | `inlineCode` (`true`), `fencedCode` (`false`) | Keys mentioned in body text exist |
| `refs.codedByPath` | `warn` | — | A coded document is referenced by path instead of key |
| `refs.moved` | `error` | — | A ref points at a key or path recorded as moved (below) |
| `refs.slug` | `warn` | — | A ref written with a slug that is not the file's slug now (below) |
| `names.shadowed` | `warn` | — | A name that is both a sibling namespace and an import alias, so `name:` and `name::` reach different documents |
| `frontmatter.unknown` | `warn` | — | Frontmatter fields not in the schema |
| `filename.pattern` | `error` | — | A file in a coded collection's folder that fits no `match` template, e.g. `tickets/README.md`; also a coded document whose name is not in the form its collection's `slug` expects, or whose slug is empty or holds an excluded character, which stays a document (`SPC-17`) |
| `imports.absent` | `warn` | — | Refs into an imported project that is absent on this machine, including one whose path uses an environment variable that is unset or empty. A project that needs its imports to be there should set this to `error` in CI, because a mistyped variable name is otherwise only a warning. An import that is absent and that no ref names is not reported, even at `error`; a misspelt alias is caught where a ref names it (`bad-prefix`, `SPC-12`) |

## Merge order

typdoc's defaults, then `validation.global`, then the collection's `validation`. A collection file
merges key by key, so it states only what differs, and this holds inside one rule's own setting
as well: a collection that sets only `level` keeps every option `validation.global` gave the same
rule, and an option the collection does set replaces only that option.

## `frontmatter.parse`

The frontmatter block cannot be parsed: invalid YAML, or a block that is never closed. No other
rule is evaluated for that file, and it takes no other part in a run: a query has no fields of
it to filter, sort or print by, and a reverse lookup reads no refs from it.

## `frontmatter.transitions`

A field whose schema gives it `transitions` may change only to a value the map allows from the
value it had. The rule is checked on write, since only a write has a value before and a value
after; `validate` does not report it on a file as it stands.

## `body.links`

Markdown links in the body (inline, image and reference-style) point at existing files; also text
that looks like a link but is not, and a reference label defined twice. Its `ignore` option holds
globs (`SPC-19`) of targets to skip, matched after percent-decoding against the target's path from
the project folder, not the link as written: `**/assets/**` skips a link into any `assets` folder,
from any document.

A link's destination is read as a name (`SPC-18`). An absolute path is reported and never followed.
`name:rest.md`, where `name` is no namespace of the project, is reported as a prefix naming none;
`tel:123` and other destinations that do not end in `.md` are URLs, and not checked.

**Text that looks like a link but is not.** A `[text](inner)` or `![text](inner)` outside code that
the parser does not read as a link, and a line `[label]: inner` that it does not read as a
definition, is reported when `inner` has no URL scheme (a namespace name or an import alias does
not count as one) and, after a trailing title (`"…"`, `'…'` or `(…)`) is removed, ends with a file
extension, optionally followed by `#anchor`. An extension is a `.` followed by one to eight ASCII
letters or digits, at least one of them a letter, so `version 1.2` has none and `ask Mr.Smith` has
one. The usual cause is an unescaped space, which CommonMark does not allow in a bare destination;
the message suggests writing `<my file.md>` or using `%20`. Without this check such a link would be
invisible: the reader sees a link and the checker sees text. The finding is at the `[` (or the
`!`), or at the start of the definition line; for a rejected definition it cannot say how many
places use it. A `[text][ref]` with no definition is not reported: CommonMark reads it as plain
text, and the shape is too common in ordinary prose (`a[0][1]`).

## `refs.moved`

A ref, or a mention when `body.mentions` is on, points at a key or path recorded in some
document's `auto: moves` field and no longer resolves. The two are matched by what they read as
(`SPC-18`), not by their text: a recorded key in the namespace that issued it, and a recorded path
from the project folder, whatever folder the ref is written from. It replaces the ordinary
missing-target finding for that ref and names the document now: its portable name (`SPC-18`) for a
recorded key, its path for a recorded path. A body link is a ref here like a frontmatter value,
so a body link to a moved target is `refs.moved`, not `body.links`; it is looked up without its
`#anchor`, since a recorded move never has one. Without a field with `auto: moves`, a moved ref
is still reported as missing, without the new key.

## `refs.slug`

A ref written with a slug (`story-2:WF-5-json-output-shape`) whose slug is not the one the file
carries now, including a file that now carries none. The ref still resolves, by its key
(`SPC-14`); the finding names the key and the file's current name: `story-2:WF-5-json-output-shape`
refers to `WF-5`, whose file is now `WF-5-json-shapes.md`. A ref by the key alone is never
reported, since it cannot go out of date. `mv` rewrites every ref this project holds, so what
this rule finds is a ref `mv` did not reach: one edited by hand, one in a project that imports
this one, or a file renamed without `mv`. `new` and `set` check it before they write, at its
level (`SPC-2`).

## `body.mentions`

It checks plain-text keys and never turns them into refs, so `refby` never counts a mention and `mv`
never rewrites one. Mentions are keys alone: text that writes a key followed by its slug (`SPC-17`)
is not read as a mention, so neither it nor its key is checked.

| Text | Checked |
| --- | --- |
| `see WF-3` | yes |
| `` `WF-3` `` (inline code) | per `inlineCode` |
| Inside a fenced code block | per `fencedCode` |
| `[WF-3](WF-3.md)` | no; `body.links` checks it |
| `UTF-8`, `SHA-256` | no; not a code of the project the mention reads into |
| `WF-3a`, `xWF-3` | no; word boundaries required |
| `WF-3-lock-order` | no; a key written with its slug is not a mention |

A mention is read as a key (`SPC-18`): with no prefix, in the document's own namespace only; with a
sibling prefix (`story-2:WF-5`), in the namespace it names; with an import prefix
(`chief::story-3:WF-5`), in that import. A mention has one outcome for every
lookup that fails, not found, unlike a ref, whose `unresolved` tells the causes apart.
`fencedCode` defaults to `false` because code blocks often hold logs, commands and diffs that
contain key-like text.

## Lines and columns

A finding's `path` is relative to the project folder, in the text output and in `--json` alike,
so a reader of either means the same file in any namespace; every path `validate` prints, in a
finding or in the audit summary, is written that way. `line` and `col` are 1-based. `line` counts
from the top of the file, frontmatter included, so it matches editors and file tools. A line ends
at a line feed, at a carriage return and a line feed together, or at a carriage return alone, as
in CommonMark, and a line ending at the end of the file does not begin another line, so `a\nb`
and `a\nb\n` both have two lines. Every command that reports a line counts this way, and the
frontmatter block is found by the same count. `col` counts Unicode scalar values, so a
consonant, a combining vowel or tone mark, an emoji and a tab each count as 1. That agrees with a
UTF-16 count, the Language Server Protocol default, except after a character outside the Basic
Multilingual Plane, such as an emoji, which UTF-16 counts as 2. `col` marks where the offending
element starts: for a link, its `[`, or the `!` before it for an image. `--json` carries no byte
offset.
