---
title: Refs explained
status: active
migrated_from: docs/archived-design/design.md#refs
follows: [PRN-2, PRN-6, PRN-11]
---

Refs come from two places, frontmatter fields and body links, and both resolve through the same
index.

## One index

Before any query or validation, typdoc builds one index mapping every key, within its namespace,
and every path to its file. Refs of either kind resolve through it, so a keyed document can point
at a path-identified one and the reverse. Paths are compared exactly as they are written, case
included, on every platform: a ref is resolved through the index of names as they are on disk, and
a file outside every collection by comparing the names in each folder, so a path that differs from
the file's name in case does not resolve, even where the file system would open it. Two keys cannot
differ only in case, because a code is capital letters and digits, so `keys.unique` has nothing to
do about case.

## Frontmatter values

A `ref` or `ref[]` value is a plain string, read as a name (`SPC-18`): a key, or a path, with or
without a `namespace:` or `project::` prefix. A key, and a path after a prefix, mean what they mean
anywhere else. A path with no prefix is read from the document's folder (`refBase: file`) or from
its namespace folder (`refBase: namespace`), and `./` or `../` from the document's folder.

Wherever a key is read, the key followed by its slug is read too, in every form:
`story-2:WF-5-json-output-shape` and `chief::story-3:WF-5-json-output-shape` are the keys
`story-2:WF-5` and `chief::story-3:WF-5`. A ref of this form resolves by its key alone. The slug
in it is not needed to find the document, so a ref whose slug is out of date still reaches it,
and `validate` reports the difference under `refs.slug`. A value of this form was read as a
relative path before slugs existed, and it named no document then, since a document is always
a `.md` file.

A name may be both a sibling and an import; the `names.shadowed` rule warns. Sibling names and
import aliases may not collide with URL schemes (`http`, `https`, `mailto`, `file`); `validate`
enforces this, as `config.namespace-name` for a namespace and as `schema.valid` for an import alias.

## Canonical form

A coded document is referenced by key in frontmatter. Referencing it by path works, but `validate`
warns under `refs.codedByPath`, since a path changes when the file is moved and a key does not.
The key alone and the key with its slug are both references by key; no rule prefers one of them.
Body links always use paths, and a path names the file exactly, slug included: GitHub opens a
link as it is written.

## Body links that are not refs

A body link that names no path, such as `[t]()` or `[t](#a)`, is not a ref, and neither is a link
with a URL scheme (`SPC-18`, step 4): neither has a document at the other end. `refs` lists neither,
in either direction.

## Ownership rules

So that two namespaces never conflict:

1. A file is validated only by the namespace that owns it. A ref from another namespace or
   project checks only that the target exists, matches `target`, and has the linked heading.
2. `match` stops at a nested project's boundary; namespaces of one project never nest.
3. A bare schema name in `target` always means this project.
4. Imports are followed one level; imports of imports are ignored.
5. Writes stay inside the namespace the scope names, except `mv`, which also rewrites refs in the
   other namespaces of the same project. No write ever leaves the project.

## Schema drift

A schema names a schema of an imported project only through a qualified name in `target`
(`"memory::learning"`). If the imported project renames or removes that schema, the `target` no
longer names anything, and `validate` reports it under `schema.valid`, at the schema file of this
project and naming the ref field. A qualified name whose alias this project does not configure
is reported the same way, since either way the target names nothing. An import that is absent on
this machine is reported by `imports.absent` instead and is not an error here. A field that an
imported project renames is not checked ahead of time, because nothing in a project's files
records which fields of another schema it relies on; a query that names a field no schema in
scope defines is an error and not an empty result, so such a change is loud when the query runs.
Run `validate --schemas` in CI when projects live in different repos.

## Imports absent on this machine

`${NAME}` in an import path is replaced by the variable's value; one rule serves the `imports` in
`config.json` and the ones in `imports.json`. A variable that is unset, or set to an empty value,
is never replaced by an empty string: that would turn `${HOME}/projects` into `/projects`, a path
that may exist and be the wrong project. The import is instead treated as absent on this machine
and reported by `imports.absent`, with a message that names the variable (`TYPMEM_DIR is not
set`), which is different from a path that does not exist. A location with no project of its own
is absent in the same way. Other imports and namespaces load normally. Imports that differ per
machine exist so that no machine-specific path is committed; if one missing import stopped the
whole project from loading, the easy way out would be to commit the path, which is what this is
meant to prevent. So a project that needs its imports to be there should set `imports.absent` to
`error` in CI: a mistyped variable name is otherwise only a warning.

A location that does hold a project whose own config cannot be loaded is an ordinary error, not
an absent import: it is a mistake at a real location, and folding it into `imports.absent` would
hide it. A `project::` argument that names an import absent on this machine is bad arguments
rather than a finding: an argument is a direct request for that document.

## Machine-specific imports

When an imported project's location differs per machine, it goes in an environment variable or in
a machine file, `imports.json`, which is merged under the project's own `imports`: for an alias
both name, the project's own entry wins. No machine-specific path is then committed. The file is
found in this order, stopping at the first step that applies:

1. `TYPDOC_CONFIG_DIR`, if set and not empty: the file is `$TYPDOC_CONFIG_DIR/imports.json`. The
   value must be an absolute path to a directory that exists; anything else is
   `config.config-dir`, since it was set on purpose.
2. `XDG_CONFIG_HOME`, if set to an absolute path: the file is
   `$XDG_CONFIG_HOME/typdoc/imports.json`. Unset, empty or relative counts as not set, as the XDG
   specification says.
3. The platform default, on Linux and macOS: `~/.config/typdoc/imports.json`. With `HOME` unset
   or empty there is no machine file.

## Heading anchors

The `slug` of a heading follows GitHub's algorithm, so a link that passes `validate` also works on
GitHub. Take the heading's plain text (text and code spans; image alt text, line breaks and inline
HTML contribute nothing), lowercase it, delete punctuation other than `-` and `_`, symbols and other
characters that are not letters or digits, and turn each space into `-`. Marks count as part of a
letter, so a combining vowel or tone mark stays; non-ASCII text is kept as written, never
transliterated. A slug that repeats an earlier one in the same document gets `-1`, `-2` and so on,
skipping any result already taken (`Dup`, `Dup`, `Dup 1` give `dup`, `dup-1`, `dup-1-1`). Every
heading counts, including those inside block quotes and list items but not those inside fenced code,
so the numbering matches GitHub's. A heading whose slug is empty (`## !!!`, `## 😀`) is not special:
the first gets `""` and cannot be linked to, the next `-1`, then `-2`. In a link, the fragment is
percent-decoded (a `%` not followed by two hex digits is kept as written) and then compared with the
slug without regard to case. A heading's slug is used only for anchors; it is not the slug in a
coded document's file name (`SPC-17`), which the caller chooses. A fixture of headings rendered by
GitHub holds the expected slugs, so the character classes are checked against GitHub's own output.