# 7: Keys, arguments that name a document, and `get` by key

Type: implementation
Status: resolved
Blocked by: 4, 5

## What this delivers

- Keys per namespace, and the identity of a document by key or by path with the case of the path kept.
- An argument told to be a key or a path by its form; a path on disk (`/`, `./`, `../`) or relative to the project; a key that exists in several namespaces exits 1 with `candidates`; a missing document exits 5, with a hint when `./name` exists.
- `get` by key in a project with one namespace and in one with several.

## Done when

- A test for the table of forms in the design, and one for the case where a file of the same relative name exists in the current directory and the project's file is still the one read.
- A ref or path that differs from the file's name only in case is not found.
