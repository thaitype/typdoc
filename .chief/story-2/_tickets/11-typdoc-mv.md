# 11: `typdoc mv` within the project

Type: implementation
Status: open
Blocked by: 3, 5

## What this delivers

- The move and the rewrite of every ref this project holds to the document, in frontmatter and in body links, in every namespace of the project, each ref keeping its written form; no file outside the project is written and no lock is taken there (decision 2).
- Temp files prepared for every file that changes, then the renames in one run, with the document moved last (decision 1); a run that stops says the same command can be run again.
- The refusals: a destination that exists at exit 7 (decision 15); source and destination that are one file, compared by file identity, at exit 7 with its own message (decision 12); the moves across collection boundaries that decision 16 refuses.
- A move onto a schema the document does not satisfy is carried out, exits 0, and reports what the schema rejects.
- `--json` adding `unrewritten`, in the reference shape `refs --reverse` uses with a `reason`, and `findings` in `validate`'s shape.

## Done when

- A stop in the middle of the renames leaves the document where it was; `validate` reports the refs already rewritten as pointing at a path that is not there; the same command run again completes the move and leaves no finding.
- Each refusal is produced by a test and leaves every file byte-identical.
- A move that lands on a schema the document fails exits 0 and the rejection is in `findings`, which is what a caller reads instead of the exit code.
- Refs held by another project are reported with the project named, not counted.
