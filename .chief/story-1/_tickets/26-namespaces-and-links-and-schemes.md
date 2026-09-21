# 26: A `namespaces` entry and a symbolic link; a namespace named after a URL scheme

Type: implementation
Status: resolved
Blocked by: None (can start immediately)

## What this delivers

Two places where a rule reaches one mechanism and not the one beside it. Both are decided in `docs/design.md` (the paragraph **Namespaces**, the config errors table, the sentence on sibling names and import aliases, and the `files.unreadable` row).

**Symbolic links in `namespaces`.**

- A glob in `namespaces` that reaches a folder that is a symbolic link skips it, and the run reports it under `files.unreadable`, the same as a `match` that reaches a link. The run goes on and answers about every other namespace.
- An entry that names a symbolic link in plain text is a config error, `config.namespaces-entry`, and its message says to name the folder the link points to.
- The message that still says "whether a run follows one is not decided" (`Error::symbolic_link`) is corrected or goes with the code that used it; nothing in the tree may say the question is open.

A project with the namespaces `story-1` and `story-2` and `ln -s story-2 current` now stops with exit 6. After the change, `namespaces: "*"` gives both documents sets, each once under its own path, and a finding that `current` was skipped.

**Namespaces named after a URL scheme.** A namespace named `http`, `https`, `mailto` or `file` is `config.namespace-name`, as the import alias of that name is refused under `schema.valid`. The entry `[namespace-scheme]` leaves `KNOWN_GAPS`, and the test that pins it becomes a test that the name is reported.

**The sweep.** Both are one symptom: a rule that reaches one mechanism and not the other. List every place a name enters the two name spaces (the namespaces, the import aliases, and anything else that becomes the left side of `name:` or `name::`) and every place a directory entry is read, and say in the report, for each, whether it applies the rule. Fix a place that does not if the design already says what it should do; if it needs a decision the design does not hold, stop and say so.

## Done when

- A test fails before the change and passes after it for each of: the link matched by a glob (the shape above), a link named in plain text, and a namespace named for each of the four schemes.
- `KNOWN_GAPS` no longer lists `[namespace-scheme]`, and the test that pinned it is replaced.
- The exit code 6 that a `namespaces` link used to produce is either produced elsewhere by a test or is listed in `UNPRODUCED_EXIT_CODES`; ticket 23's report says which places produce it.
- The accounting invariant still holds on every fixture project, and the sweep is in the report.
