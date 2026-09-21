# 24: The audit shows every collection and names the collections of each overlap

Type: implementation
Status: resolved
Blocked by: None (can start immediately)

## What this delivers

`docs/design.md` says that a file matched by two collections is an error and never settled by precedence, and that `--audit` shows every collection, including one that ends with no document, and names the collections of each overlap. The binary keeps the error as it is and does not yet show either. This ticket makes the audit agree.

- `audit.collections` has one entry for every collection of the project, including a collection whose number of documents is 0.
- `audit.overlapping` is a list of `{ "path", "collections" }`, with `collections` the names of the collections that match the file, sorted by name, the list sorted by `path`. The data is already in the `collections.overlap` finding's message; it is not yet in the list.
- The text form of `validate --audit` shows the same.

## What does not change

- `collections.overlap` stays an error, and no rule chooses between two collections.
- Every number that exists today keeps its value: `summary.checked.documents`, `summary.unreported.*`, `summary.overlapping`, and the `documents` of every collection that is listed today. A file matched twice is still counted in the number of no collection, and the accounting invariant holds on every fixture project as it did.
- The fields of `findings` and of `summary` keep their shape. Only the entries of `audit.overlapping` change type, from a path to an object, and `audit.collections` gains entries.

## The fault

A project of two files, `a.md` and `b.md`, with a collection `notes` that matches `*.md` and a collection `skills` that matches `a.md`. `a.md` is matched twice; `b.md` belongs to `notes` alone.

```console
$ typdoc validate --audit --json
... "collections":[{"name":"notes","documents":1}] ... "overlapping":["a.md"] ...
```

`skills` is missing from `collections`, though it is a collection of the project, and `overlapping` says a file is matched twice without saying by which. After the change `collections` holds `notes` with 1 and `skills` with 0, and `overlapping` holds `{ "path": "a.md", "collections": ["notes", "skills"] }`.

## Done when

- A test fails before the change and passes after it: the shape above, asserting `skills` with 0 and the named collections. A second case with a collection that matches nothing at all shows it with 0 as well.
- Every consumer of the audit object is walked and named in the ticket's report: the invariant test and its independent count, the golden files, the shell examples, the text form, and any fixture whose audit lists an overlap. A test or golden that read `overlapping` as a list of paths is changed to the new shape, and no number in one of them changes.
- The invariant still holds on every fixture project, and a test says that a file matched twice is counted in no collection's number.
