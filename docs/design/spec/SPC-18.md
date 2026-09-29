---
title: Names
status: active
follows: [PRN-11]
---

A document has one identity: the project it is in (this one, or an import by its alias), its
namespace, and its path from the project folder. Every name is that identity written from a
context, and one grammar reads every name: a frontmatter ref (`SPC-14`), a body link, a mention, a
command's argument (`SPC-2`) and what a command prints (`SPC-12`). A key is an attribute of a
document, not part of its identity.

## Where a name is written

| Place | A path with no prefix is read from | `./` and `../` from | Keys |
| --- | --- | --- | --- |
| a frontmatter ref | the document's folder, or its namespace folder (`refBase`) | the document's folder | yes |
| a body link | as a ref | as a ref | no: a body link is always a path |
| a mention (`body.mentions`) | not a path | not a path | only |
| a command's argument | the project folder | the current directory | yes |

A ref and a body link are two kinds of text. A ref is a name: the direction is that a path in a ref
carries its prefix, so that it means the same from every document, while a key may drop its prefix
(`PRN-11`). A body link is a Markdown link, read as Markdown readers read it: a path from the
document, never a key, and in the direction of `PRN-6` never prefixed. The grammar below still reads
a path with no prefix in a ref and a prefix in a body link; requiring either form is later work. In
both, `./` and `../` are read from the document's folder, whatever `refBase` says; `refBase` moves
only a path with no prefix.

## Reading a name

A name is read in this order; the first step that applies decides.

1. As an argument, a path on disk: an absolute path, or one beginning with `/`, `./` or `../`, and
   on Windows `\`, `.\` or `..\`, read from the current directory (`SPC-2`). In a ref or a body
   link, an absolute path, a drive letter included (`C:\a.md` on Windows), is never resolved: it is
   reported under `refs.resolve` or `body.links` as an absolute path, and `refs` lists it as
   `unresolved: absolute` (`SPC-12`).
2. In a body link, a URL is not a ref: a name that begins with a scheme followed by `//`, or with
   `http:`, `https:`, `mailto:` or `file:`, the schemes no namespace or alias may be named.
   `alias::rest` otherwise: `rest` is read in the imported project `alias`, from its folder and with
   its namespaces. An alias the project does not configure is a bad prefix, and one configured but
   not on this machine is an absent import (`imports.absent`). A second `::` is a bad prefix:
   imports of imports are not read. A key into a project with several namespaces names one
   (`chief::story-3:WF-5`); `chief::WF-5` is a bad prefix there, since only a project with one
   namespace has `default`.
3. `name:rest`, where `name` is exactly a namespace of the project, or `default` in a project with
   one namespace: `rest` is a key in that namespace when it has the shape of a key, and otherwise a
   path from that namespace's folder. `name` is never a glob or a list; those are for `--namespace`
   and `TYPDOC_NAMESPACE` only.
4. `name:rest`, where `name` is no namespace: a bad prefix. In a body link it is one only when
   `rest` ends in `.md`; otherwise the link is a URL (`tel:123`), and not a ref.
5. A name with the shape of a key, `CODE-number` with or without a slug (`SPC-17`): a key, in the
   writer's namespace in a ref, in the command's scope as an argument, whatever its code. A key no
   document has is `no document with key <key>`. In a body link this step does not apply. A key
   recorded in an `auto: moves` field is read the same way: with no prefix, in the namespace of
   the document that holds it.
6. Anything else: a path, from the base the place gives in the table above.

`name:` and `name::` never fall back to each other, nor to a path: a name the prefix does not
reach is an error. A path that really contains a colon is written with a leading `./`.

A name in step 2 or 3, and a key, mean the same wherever they are written. Only a path with no
prefix, and `./`, depend on the place.

## Writing a name

typdoc writes a document's name in one of three forms, from the context it is written for:

- **`path`**: its path from the project folder, the file to open. It is not a name to copy into a
  ref: there, a path with no prefix is read from the document.
- **`ref`**, the portable name: `KEY` for a coded document in a project with one namespace,
  `namespace:KEY` in a project with several; `default:path` for a document with no code in a project
  with one namespace, `namespace:path-from-its-folder` in a project with several; a document of an
  import is prefixed with `alias::`. The portable name reads back as the document from every context
  of the project it is printed for. A file outside every namespace folder has none: no prefix names
  where it is, and a path with no prefix depends on the place.
- **as written**: when `mv` rewrites a ref to a document it moved, the new name keeps the form the
  old one had: a key stays a key, a prefix stays, a slug is kept or renamed as `SPC-17` says, and a
  body link keeps its `./`, its `#anchor`, its `<…>` and its percent-encoding. Where the old form
  can no longer reach the document, a path with no prefix under `refBase: namespace` to a document
  moved out of the namespace folder, a ref takes the document's prefix (`PRN-11`) and a body link
  the path from the document (`PRN-6`).

Every name typdoc writes in a context reads back, in that context, as the document it was written
for; and every document's portable name reads back as it from every context. A test holds this for
every document of the fixture projects in every context.
