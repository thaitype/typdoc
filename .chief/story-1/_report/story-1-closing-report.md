# Story 1 closing report

## What the story delivers, as measured
- `get`, `list`, `toc`, `refs` and `validate` (with `--schemas` and `--audit`) read a project, its
  namespaces, collections, schemas and imports, and answer with `--json` in the shapes and exit
  codes of the design. Nothing is written: `typdoc-core` calls no function that changes the file
  system, spawns a process or reads the environment, and the ban is checked by clippy.
- 23 tickets, all resolved; one report per ticket in this folder. The suite is 688 tests passed,
  0 failed, 1 ignored (the golden-file regenerator, by design), run through `scripts/test.sh`
  under a memory ceiling.
- The shared test infrastructure is in place: fixtures with coverage checks, golden files with a
  guard on the generator, the spawn helper, the lint on test code, the harness for the shell
  examples, and the lists of differences between the design and the binary.

## The two criteria of the goal
- **The accounting.** Checked on every fixture project by a test, and by hand on the copies of
  `chief` and `typmem`; the results and the method are in `ticket-21-report.md`. In all three
  runs the documents checked, the files in no collection, the files with no frontmatter and the
  overlapping files add up to the number of `.md` files the run reads, counted without typdoc,
  and the lists agree file by file.
- **A program can drive the commands and read JSON.** Each command has `--json` and the exit
  codes of the design's table. Exit codes 3 and 4 belong to story 2 and are in
  `UNPRODUCED_EXIT_CODES`.

## Where the binary and the design still differ
Kept here rather than tidied away; none is a missing command.
- `[reverse-scope]`: a reverse lookup scans this project's namespaces only, not those of the
  projects it imports. Closing it is not "scan the imports too": a project loaded as an import
  does not load its own imports, so a ref pointing back into the importer cannot be resolved
  from inside it. When it is closed, the reverse direction will need to print a `project`.
- `[namespace-scheme]`: a namespace named after a URL scheme is not reported, while an import
  alias of that name is refused.
- `[import-anchor]`: a body link across an import has its file checked and its anchor not.
- `config.legacy-file` stops every command, though the design's own question, whether checking
  is still possible, says it should be a finding that stops nothing (ticket 8).
- `audit.collections` omits a collection that holds no document and does not count an
  overlapping file for any collection (ticket 21).
  Decided since this report was written: a file matched twice stays an error, with no rule that
  chooses between the two collections, which is recorded in the design as intended; the audit
  will list every collection, one with 0 documents included, and name the collections of each
  overlap. Ticket 24 holds the change and is resolved: the audit now does both, and no number in
  it moved.
- A `namespaces` entry that matches a symbolic link still stops the run with exit 6, while a
  `match` skips it and reports `files.unreadable` (ticket 23).
- `files.unreadable` is reported by the whole-project scan of `validate` only: `validate <path>`
  in a folder that holds a link says nothing about it (ticket 23).

## Decisions taken where the design is silent
Each is written in the report of the ticket that took it, with the doubt that remained: the
placeholder choices of tickets 1 to 20, and for the walk, the six of ticket 23 (a template that
ends in `**` takes a dot file; the name in a finding about a name that is not valid UTF-8; and
the others named there).

## Not done, on purpose
`new`, `set`, `mv`, `pull`, locks, and `frontmatter.transitions` are in later stories. Nothing in
this story fetches, so a remote schema with no pin is `config.schema-unpinned`.
