# 10: Refs in frontmatter and their rules

Type: implementation
Status: open
Blocked by: 7, 8

## What this delivers

- The forms of a ref (bare key, sibling prefix, relative path with `refBase`), resolved through the index of names as they are on disk.
- `refs.resolve`, `refs.target`, `refs.acyclic`, `refs.moved`, `refs.codedByPath` and `names.shadowed`.
- The reasons a ref does not resolve, `not-found` and `bad-prefix` (`import-absent` comes with ticket 17).

## Done when

- Each rule leaves `unimplemented_rules` and has fixtures; a ref whose case differs from the file's is `not-found`; `chief::WF-5` in a project with several namespaces is `bad-prefix`.
