# Goal

A document has one identity, and every name is that document written from a context. A key, and a path with a
`namespace:` or `project::` prefix, names the same document in a frontmatter ref, in a body link, as a command's
argument and in what a command prints; only a path with no prefix depends on where it is written. What typdoc
prints in a context is accepted in that context, and a document's portable name, the new `ref`, in every
context. `--json` `path` is unchanged: it is where the file is, for opening it.

Two central, inverse functions do it: `resolve(text, context) -> document` reads every name (refs, body links,
arguments, `mv`), and `format(document, context) -> text` writes every name (`path`, `ref`, the names in text
output, the refs `mv` rewrites). For every document and every context, `resolve(format(document, context),
context)` is that document, and its portable name resolves to it from every context.

PRN-11 states the principle, and one spec states the name grammar; SPC-2 and SPC-14 point to it.

The same story adds Windows to `publish.yml`'s test gate.

## User-visible changes (decided)

- D1: a path after `namespace:` is read from that namespace's folder everywhere; as an argument it was read from
  the project folder (`story-2:story-2/notes/x.md` stops working, `story-2:notes/x.md` starts to).
- D2: a name with the shape `CODE-n` is a key in a ref as on the command line: `no document with key XX-1`.
- D3: an absolute path in a ref or a body link is a finding, never resolved.
- D4: every document in `--json` and in text output gains its portable name, `ref`; `path` does not change.
- D5: a document of an import is printed with its `project::` prefix.
- D6: in a body link, `name:rest`, where `rest` does not start with `//` and ends in `.md`, is read as a
  namespace; a `name` that is no namespace is a finding. Real URLs are not refs, as before.
- D7: in a project with one namespace, the portable name of a document with no code is `default:notes/x.md`; a
  coded one's stays `WF-1`.
- D8: a namespace prefix is an exact name everywhere; globs and lists stay in `--namespace` and
  `TYPDOC_NAMESPACE` (`story-*:WF-1` becomes `WF-1 --namespace story-*`).
- D9: text output keeps every name it prints, `path` included, and gains the portable name beside it, labelled
  `ref`.
- D10: the `auto: moves` map keeps a key's namespace, `mv` rewrites a mention only of the moved document, and a
  mention with `::` is read like any other name.
- A fix: `mv` keeps a body link's `./` and anchor when it rewrites it, and rewrites every link on a line (0.6.0
  dropped both and left the second of two links to the moved file as it was).

## Out of Scope

- A release.
- Changing what a path with no prefix is relative to (a ref: the document or the namespace folder per `refBase`;
  an argument: the project folder, or the current directory with `./`).
- Keys in body links: a body link is always a path.
