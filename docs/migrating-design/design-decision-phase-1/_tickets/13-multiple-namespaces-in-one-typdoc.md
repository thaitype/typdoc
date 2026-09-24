# 13: How do several namespaces live in one `.typdoc`?

Type: wayfinder:grilling
Status: resolved
Blocked by: None (can start immediately)

## Question

A design change (2026-09-19): an optional `namespaces` option in `config.json` (string or array of path/glob, each matching folder is one namespace, no nesting); collections shared, with `match` counted from each namespace folder; `last` moves to `.typdoc/state/<namespace>.json`; one lock per namespace at `.typdoc/locks/<namespace>.lock`; sibling refs such as `story-2:WF-5` with no import; `refby` and `mv` scan every namespace in the same `.typdoc`; CLI scope by working directory with `--namespace` when ambiguous; and `imports` pointing at a whole multi-namespace project (`chief:story-3:WF-5`), whose place in v1 was undecided.

Motivation: one `.typdoc` per project; per-worktree work without number collisions; `last` out of the hand-edited collection file.

Find the gaps in the spec before it is written into `docs/design.md`. Known gaps so far: what a namespace is named and how that maps to file names; the default single-namespace case; the `name:rest` syntax shared by sibling refs and imports; the limit of the worktree fix (two worktrees on the same namespace still collide); scope of key uniqueness; whether whole-project import is in v1.

Amends the answers of tickets 2 (where `last` lives), 12 (collection files), 7 (lock path) and touches 5 (query scope) once resolved.

## Answer

Decided 2026-09-19. `docs/design.md` is amended throughout (Model, Config, Namespaces, State, Discovery, Choosing a namespace, Refs, Query, Commands including `mv`, Validation rules, Concurrency, Exit codes, Worked examples).

**Vocabulary.** A *project* is a folder with `.typdoc/config.json` (what `imports` points at). A *namespace* owns documents and numbers its own keys. A project is one namespace named `default`, or several when `namespaces` is set.

1. **Namespace name.** The name of the folder, one level directly inside the folder holding `.typdoc`; `[A-Za-z0-9_-]` only. Every `namespaces` entry is one segment: `/` and `**` are config errors. `*` never matches a folder starting with `.`. A matched folder with a bad name is a config error that names the folder, never skipped. Reason: a path reference across projects (`chief::projects/a/_tickets/WF-5.md`) could not show where the namespace ends if a name could contain `/`. Deeper grouping means placing `.typdoc` deeper.
2. **Default and state.** With no `namespaces`, the namespace is `default` and its state is `state/default.json`, not tied to any name, so nothing in config can reissue numbers. `default` is reserved for folders. The `namespace` pseudo-field is `default` in a one-namespace project; an imported project shows as its alias. A file in `state/` matching no current namespace is a config error naming the file and the fix; this covers switching modes and deleting a story.
3. **Syntax.** `name:` reaches a sibling namespace only; `name::` an import only. No fallback either way; a name missing on the side the syntax names is an error. An import alias may equal a sibling name; the new configurable rule `names.shadowed` (default `warn`) reports it. Both kinds of name may not collide with URL schemes. A ref or mention with no prefix means the document's own namespace only. `target` qualifiers use `::`. `body.mentions` changes from "own namespace first" to "only".
4. **Whole-project import is in v1.** `alias::ns:KEY`, and `alias::KEY` for a one-namespace project. `--namespace` may name an imported namespace for reading only; `mv` still rewrites the refs it can see in imported projects.
5. **`name` removed from `config.json`.** Namespace names come from folders (or `default`), import names from the importer's aliases, collection names from file names. The smallest config is `{ "version": 1 }`; a leftover `name` is an unknown-key config error. Reason: adding an optional field later is not breaking, removing one is.
6. **Choosing a namespace.** Order: prefix on the argument, `--namespace`, `TYPDOC_NAMESPACE`, current directory inside a namespace folder (or below), otherwise no scope (reads span all, writes are an error). `TYPDOC_DIR` replaces `--dir` because an agent often runs from a repository or worktree root above `.typdoc`. An ambiguous key or write exits 1 with every choice and `candidates` in `--json`; typdoc never picks. Refs inside a file always mean that file's namespace.
7. **`--namespace` grammar.** Names, `,`-separated lists and globs; `'*'` means every namespace of this project (imports excluded; `'chief::*'` names them). It beats `TYPDOC_NAMESPACE` and the working directory. A write that could reach more than one namespace is an error. Docs always quote `'*'`.
8. **Moving across namespaces.** A coded document cannot move to another namespace (its key belongs to its namespace): an error suggesting `--renumber`. A document without a code can move and its refs are rewritten. `mv --renumber` issues the next key from the destination's `last`, holds both locks in name order, and rewrites visible refs (bare and prefixed) in the form correct from each document's namespace; the old key is never reissued because the source `last` does not go down. New `auto: moves` for `list` fields (opt-in per schema, `set` cannot write it, always prefixed, appended on every move) and new rule `refs.moved` (default `error`) that names the new key. Two cases `mv` cannot rewrite: body links when `body.links` is `off`, and refs from projects that do not import this one (caught when that project runs `validate`). Written into the `mv` section.
9. **`--renumber` is in v1.**

**Reading of `refs.moved`:** it only adds the new-key hint. A ref to a moved key with no `auto: moves` field is still reported as missing by the ordinary rule, so it is never silently valid.

**Defaults:**
- A matched namespace folder that holds its own `.typdoc` is a config error.
- `TYPDOC_NAMESPACE` uses the same grammar as `--namespace`; `TYPDOC_DIR` names the folder that holds `.typdoc` and, when set, skips the walk; a document path argument wins over it.
- In a sibling reference the part after `name:` may be a key or a path; in a body link it is always a path. A relative path containing a colon needs a leading `./`.
- A ref into a multi-namespace import must name the namespace (`chief::WF-5` is an error).
- The `namespace` pseudo-field for a document reached through an import is the alias, followed by `::` and the namespace when the imported project has several.
- `refs.moved` replaces the missing-target finding for the same ref (one finding, not two).
- The guarantee written for `mv`: for a coded document nothing can point at a different document silently; a document without a code can have its path recreated later, and a ref to it then means what it says. So "no way to point wrongly and silently" holds for coded documents only.
- Text output states the scope and `--json` rows carry `namespace`, so a result never hides how wide it was.
- `--json` errors gain `candidates`; the exit code stays 1.

**Amends earlier answers (each also carries a note):** ticket 2 (where `last` lives: `state/<namespace>.json`, per namespace and collection), ticket 12 (collection files are configuration only, `match` counts from the namespace folder, overlap checked per namespace, `last` is an unknown key there, config errors list), ticket 7 (lock path `.typdoc/locks/<namespace>.lock`; the git-common paragraph now says worktrees on different namespaces never share a state file, two on the same one still conflict on merge), ticket 5 (scope is the project and its imports; `namespace` pseudo-field values), ticket 4 (keys unique per namespace).

**Limit stated in the design:** the worktree fix works when each worktree owns its namespace. Two worktrees on the same namespace still conflict on `state/<namespace>.json` at merge, which is loud.
