# Contract

## Identity and context

A document's identity is `(project, namespace, path)`: the import alias (none for this project), the namespace
and its path from the project folder, as the index holds it. Its key is an attribute of it, not part of it.

A context is where a name is written or printed: the project (and, inside an import, the import), the namespace
of the writer, and the place:

| Place | A path with no prefix is read from | `./` and `../` from | Keys |
| --- | --- | --- | --- |
| frontmatter ref | the document's folder, or its namespace folder (`refBase`) | the document's folder | yes |
| body link | as a ref | as a ref | no: a body link is always a path |
| command argument | the project folder | the current directory | yes |
| body mention (`body.mentions`) | not a path | not a path | only |

## One grammar, two functions

A new module, `crates/typdoc-core/src/name.rs`, holds both directions and nothing else reads or writes a name:

- `resolve(text, place, scene) -> Result<Resolved, Unresolved>` (`Resolved` as refs report it today, with the
  path, the project and a written slug; `Unresolved` adds `Absolute`, `NotARef` and, for an argument's key,
  `Ambiguous`), read in this order:
  1. An argument only: a path on disk (absolute, `/`, `./`, `../`, and on Windows `\`, `.\`, `..\`), from the
     current directory. This step needs the current directory and stays with the argument's caller, before
     `resolve`.
  2. `alias::rest`: the import `alias`, `rest` read as from that project's folder with its own namespaces. An
     alias not configured is a bad prefix; one not present is an absent import. A second `::` is a bad prefix.
  3. `name:rest`, `name` exactly a namespace (D8: never a glob or a list, which only `--namespace` and
     `TYPDOC_NAMESPACE` take); in a project with one namespace, `default`: `rest` is a key in it when it has the shape `CODE-n` (with or
     without a slug), else a path from that namespace's folder (D1).
  4. `name:rest`, `name` not a namespace: a bad prefix; in a body link, only when `rest` does not start with `//`
     and ends in `.md` (D6), otherwise the link is a URL and not a ref.
  5. The shape `CODE-n` (with or without a slug): a key, in the writer's namespace for a ref, in the command's scope
     for an argument, whatever the code (D2: `no document with key XX-1`). Not in a body link.
  6. An absolute path in a ref or a body link: `Unresolved::AbsolutePath`, a finding (D3).
  7. Anything else: a path from the place's base.
- `format(identity, form, place, namespaces) -> String`, `identity` carrying the key and slug a form may
  write, four forms:
  - `Path`: the path from the project folder; what `--json` `path` is today, byte for byte.
  - `Portable`: `[alias::][namespace:]KEY` for a coded document, the namespace written when the project has more
    than one; `[alias::]namespace:path-from-its-folder` for any other, `default:path` in a project with one
    namespace (D7). It resolves to the document from every
    context of the project it is printed for.
  - `Relative`: the path from the place's base, what a path with no prefix is in a document.
  - `Like(written)`: the name written the way `written` was (key or path, with or without a prefix, `./`, a slug,
    an anchor kept by the caller), for the refs `mv` rewrites.
- Round trip: for every document and every context, `resolve(format(d, form, c), c) == d` for each form that
  context accepts, and `resolve(format(d, Portable, c), c') == d` for every context `c'` of the project.

`argument.rs`, `refs.rs` (`classify`, `classify_body`, `resolve_into_import`, `resolve_into_project`), `mv.rs`
(`sibling_prefix`, `key_form`, `namespace_of`, `folder_of`), `project.rs` (`ref_name_in`, `mention_missing`,
`printed_key`, the `auto: moves` reading, `mv_reverse_mentions`' key match, `resolve_uncoded_target`'s namespace)
and `cli.rs` (`identity_text`, `ref_name_text`) call these functions instead of reading or building names
themselves. The path-to-namespace rule is one function.

## Output

- `--json`: every document object (`get`, `set`, `new`, `mv`, `list`, `toc`, `refs` and its entries) keeps `path`,
  `namespace`, `key`, `project`, and gains `ref`, its portable name (D4). Nothing is renamed or removed (SPC-12).
- Text output keeps every name it prints today, `path` included, and gains the portable name beside it,
  labelled `ref` (PRN-9): a `ref` column in `list` and `refs`, a `ref:` line in `get`, `set`, `new` and `mv`'s
  labeled output. A document of an import is printed with its `alias::` (D5).
- Messages name a document the way its context would accept it.

## mv

The refs `mv` rewrites are `format(new identity, Like(written), holder's context)`: a ref keeps its form, a body
link keeps `./`, its anchor and its `<…>` or percent encoding. (Measured on 0.6.0: `[one](./b.md#part)` became
`[one](c.md)`, and `[two](b.md#part)` on the same line was not rewritten.)

## Spec

PRN-11, its wording reviewed in the pull request. A new spec, the name grammar, `follows: [PRN-11]`: the
context table, the reading order, the three forms and the round trip. SPC-2 and SPC-14 point to it instead of
restating it; SPC-12 gains `ref`.

## Windows in publish.yml

`publish.yml`'s `cargo test + clippy` gains `windows-latest`, running the suite as `ci.yml`'s `Windows tests` does
(`cargo test --workspace --no-fail-fast --features typdoc/test-stand-in`), clippy as on the others. Shown red on a
planted failing test, then green, on a throwaway pull request.

## CHANGELOG `[Unreleased]`

Changed: D1 for arguments, D2 for refs, D3, D5, D8 (`story-*:WF-1` becomes `WF-1 --namespace story-*`). Added:
`ref`, in `--json` and in text output. Fixed, on a line of its own: the `mv` body link rewrite that dropped `./` and
the anchor and missed a second link on the line (0.6.0). Fixed: D10. The library: whatever `typdoc-core` API the new module replaces.

## Testing Decisions

- `crates/typdoc/tests/name_forms.rs` (TK-1) is the table of record: each row a decision changes is changed in the
  commit that changes it, and no other row moves.
- The round trip as a test over every document of `fixtures/valid/*` and the TK-1 project, every context: every
  document as a ref holder with each `refBase`, as a body link holder, and arguments from the project folder and
  from each namespace folder.
- `name.rs` unit tests for the reading order, one per step, and for each form of `format`.
- The `mv` body link fix has its own test: `[one](./b.md#part) and [two](b.md#part)` on one line, both rewritten,
  `./` and the anchor kept, and `validate` clean after the move.
- Golden files that print a document gain `ref`; `path` stays byte-identical.
- Strict mode for the tickets that touch `resolve`, `format` and their callers; standard for docs, `publish.yml`
  and the CHANGELOG.
