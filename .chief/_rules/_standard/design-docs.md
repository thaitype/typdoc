# Design lives in `docs/design/`

`docs/design/` is the source of truth for how typdoc behaves:

- `spec/` — collection `spec`, `SPC-n`: prose for people. Code never reads it.
- `catalog/` — collection `catalog`: JSON bodies that code and tests read
  (rules, commands, exit codes, config errors, frontmatter losses). Each names the SPC
  that explains it in `explained_by`.

`docs/archived-design/` is frozen: never edit it.
`docs/migrating-design/` is a working copy being emptied: never add to it or edit it in place,
except to repoint a cross-reference whose target section was moved out, to the SPC that now
holds it.

## Every change to behavior updates `docs/design/` in the same PR

Behavior means anything a user can observe: config keys, discovery, rules, commands,
messages, exit codes, output.

1. The area is already in `spec/` or `catalog/` → change it there.
2. The area is still only in `docs/migrating-design/` → move it:
   - write the section into an SPC (a new one via `typdoc new SPC`, or an existing one that
     covers the area), describing the new behavior, not the old;
   - set `migrated_from: docs/archived-design/design.md#<section-anchor>`;
   - delete the moved text from `docs/migrating-design/`.
3. A new or changed item in a list the code reads goes into its catalog document, and the
   SPC that explains it is updated too.

Move only the sections the change touches. Sections nothing touched stay where they are.

## Done when

- `typdoc validate` passes on the repo.
- `git diff` under `docs/migrating-design/` is deletions only, apart from repointed
  cross-references.
- Every behavior change in the PR is described in an SPC or a catalog entry.

## Order of work in a story

1. Change `docs/design/` (spec, catalog) to describe the behavior the story wants — it must
   stay valid at every commit.
2. Then write the story's goal. The goal is valid only inside its story; the design is not.

`.chief/story-N/` holds only what chief creates (goal, contract, tickets, reports) — no other
planning files.
