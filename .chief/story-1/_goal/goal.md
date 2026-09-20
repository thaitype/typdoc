# Goal

Story 1 of the three that deliver typdoc v1. It builds the read core: the part of typdoc that reads a project and answers questions about it, and changes nothing. `docs/design.md` is the source of truth for every behaviour named here; this goal adds no requirement that the design does not state. The decisions taken before planning are in `docs/design-decision-phase-1/`.

When the story is done, a person or a program can point typdoc at a folder of Markdown files with YAML frontmatter and:

- have typdoc find the project (`.typdoc/config.json`), its namespaces, its collections and its schemas, and, through `imports`, other projects on the same machine;
- read a document by key or by path, list and filter documents with the query language, see the headings of a document with their line ranges, and follow refs in both directions (`get`, `list`, `toc`, `refs`);
- check the whole project, or named documents, against its schemas and rules, and get a report that says what it covered (`validate`), including `validate --audit`, which answers what has to be fixed before typdoc can be adopted on files that already exist;
- rely on the output of every one of these commands: each has `--json`, whose shape, order and exit codes the design fixes, and each has the exit codes of the design's table.

The story is done when both of these hold:

- `validate --audit`, run on a copy of the public `chief` repository (26 of its 61 tracked `.md` files carry frontmatter) and on a copy of the public `typmem` repository (23 of its 26), counted on 2026-09-20, with a `.typdoc` folder and local schemas written for the run and not committed to those repositories, gives a report that accounts for every file the run reads. The documents checked, plus the files in no collection, plus the files with no frontmatter, equal the number of `.md` files the run reads, counted independently of typdoc, so no file is missing from the report without a place in it. The intent is a report someone can act on; this is the part of that which a machine can check, and the one that catches a report that looks clean because it skipped something without saying so.
- A program can drive `get`, `list`, `toc`, `refs` and `validate` and read their JSON without parsing text.

The same accounting is checked automatically on every project in the fixtures, and the two runs on the named repositories are made by hand at the end of the story and their results recorded in its closing report, since the repositories are not fixtures.

The story also lays the test infrastructure that the later stories share: the fixtures and their coverage checks, the golden-file generator with its guard, the spawn helper, the lint that reads test code, the harness for the shell examples, and the lists that record differences between the design and the binary. The table "Where each part is built" in ticket 9 of `docs/design-decision-phase-1/` says which part each story builds; this story builds the part marked for it.

It reads and never writes. It writes no document and no state file, takes no lock and makes no network request. It reads what has been pinned already: a state file for the rule `state.missing`, and pinned copies of remote schemas.

## Constraints that hold for the whole story

- Which files a run reads (whether it enters folders whose names start with `.`, and what it does about symbolic links and ignored files) is not yet fixed in the design, and it decides the number in the first criterion above; most of the frontmatter files of `typmem` are in folders that start with a dot. It is fixed in the design before the contract is complete.
- The story is an internal milestone and is not released on its own. Until story 3 adds fetching, a remote schema that has no pin is reported as `config.schema-unpinned`, so the story cannot be tried on a project whose schemas are remote.
- Every rule and every config error that the story builds has a fixture that turns it red, and the checks that keep that true are part of the story (see the contract's testing decisions).
- Every line of the repository reads as its owner's own work (project rule 7); the story adds nothing that does not.
