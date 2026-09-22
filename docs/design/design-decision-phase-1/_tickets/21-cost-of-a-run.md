# 21: What does v1 promise about the cost of a run?

Type: wayfinder:grilling
Status: resolved
Blocked by: 15

## Question

The design has no figure or statement about the cost of a run beyond "no cache". The index is built on every run, and `list` reports `total`, which forces it to filter every document.

## Answer

Decided: v1 promises no figure. The design says plainly that every run builds its index from the files with no cache, so the time of a run grows with the number of documents, and that one figure will be measured when the index exists. A figure now would have no source, and this project does not invent numbers. The projects that would use typdoc first are small (the corpus of real files counted for ticket 1 was 26 and 23 files with frontmatter).

Part of the cost is a choice and is recorded as one. `list` reports `total`, so it filters every document even under `--limit` (ticket 15). Whoever measures later will know where that part of the cost comes from and what it was traded for: a result that says how much it left out.

Rejected: a target with a generated fixture as a gate, for the reason above. Rejected: a cache, which is a second source of truth that can disagree with the files.

Not verified: the size of the projects users will have.
