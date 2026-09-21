# 27: A ref to a file outside every namespace folder stops the process

Type: implementation
Status: resolved
Blocked by: None (can start immediately)

## What this delivers

With `namespaces` set, a frontmatter ref or a body link that reaches an existing file outside every namespace folder (`up: ../README.md`) makes `refs`, `list --where 'ref.any(f)'`, `ref.all(f)` and `refby.any(f)` exit 101 with a panic at `ref_name_in` (`project.rs`), which expects a namespace with an empty folder that only the default single namespace has. The same happens through an import: a project imports a multi-namespace project and points a ref at that project's root file. `validate`, `get` and `toc` do not stop.

Decided in `docs/design.md` (Namespaces): such a ref resolves; the file is named by its path alone; `--json` leaves `namespace` out.

- `ref_name_in` returns an `Option`/`Result` instead of panicking; the three callers give the name with no namespace, and `RefName.namespace` is optional where it is printed. No `expect` stays on this path.
- Every consumer of `RefName.namespace` is walked (refs output both directions, `ref.*` and `refby.*`, the reverse index key of ticket 22, text output).

## Done when

- Tests fail before the change and pass after it, for each of `refs` (frontmatter and body link), `list` with `ref.any`, `ref.all` and `refby.any`, and through an import; each also asserts the exit code, that nothing is written to stderr, and the printed name has no `namespace`.
- A control with one namespace (`default`) is unchanged.
- No number or shape that exists today changes for a file inside a namespace.
