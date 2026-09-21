# Ticket 19 Report

## Ticket
`validate --audit`: the audit report, rules that are off reported as information, and the accounting invariant.

## Outcome
**not resolved.** The work is built and its gates are green, and it is not finished, because the ticket is done when the contract's accounting invariant holds on every project in the fixtures and that invariant cannot hold as it is written. The ticket stays claimed until the shape of the answer is decided.

## What cannot hold, and how large it is
The contract asks that `summary.checked.documents` plus `summary.unreported.uncollected` plus `summary.unreported.no_frontmatter` equal the number of `.md` files a run reads. In the fixture with a file matched by two collections there are two such files and the three numbers give one.

A file matched by two collections belongs in none of the three: it is not checked, because there is no one schema to check it against; it is not uncollected, because it is in two collections rather than none; and it has frontmatter. The audit object the design describes has `collections`, `uncollected` and `no_frontmatter`, and none of them is a place for it.

**The size of the problem is worth stating exactly.** The file is not missing from the output: it is in `findings`, as an error under `collections.overlap`, in an audit exactly as in a plain run. What is missing is the file from the summary's arithmetic and from the audit object's lists. So a reader is not blind to it; a reader who adds up the summary is told of less work than the run found. That is the thing the design gives `unreported` to prevent, in its own words, and it is why this is a question about reconciliation rather than about a hidden file.

Every other project in the fixtures satisfies the invariant. Two that appear not to, on a first count, do satisfy it: one holds a file outside every namespace folder and one holds a file and a folder whose names begin with a dot, and a run reads none of those. The invariant is about the files a run reads, not the files on disk.

## The test in the tree asserts a shape nobody has decided
The suite contains a test of the invariant with a fourth term for the files matched by more than one collection. It is green, and it is **not** a statement of the contract: it records the gap rather than closing it, and the shape it assumes is one of the options and not a decision. It is left exactly as it is, deliberately, so that nothing is quietly turned into a specification by being written down and passing. A later reader should treat that fourth term as an open question, not as the rule.

## What is built and green
Rules set to `off` are reported as information under an audit; an audit exits 0 unless the config itself cannot be read; `summary.audit` and `summary.unreported`; the audit object with its collections, its uncollected files and its files with no frontmatter, which are listed and not evaluated; and the text form. The refusal that stood in for `--audit` is gone rather than left unreachable, along with its test.

## Decisions taken where the design is silent, with the doubt that remains
- **`--schemas` together with `--audit` is refused.** The design says either of them with document arguments is bad arguments and says nothing about the two together, and the first reading of the code answered `--schemas` and dropped the audit without a word. Refusing is the reading that does not answer a different question from the one asked. Doubt: an audit of schemas alone is a sensible thing to want, and nothing in the design forbids it.
- **A namespace whose only file is uncollected is named in `checked.namespaces`,** since the audit did cover it, which is how the file was found.
- **The walk that finds uncollected files uses the conventions the existing walk already uses:** a name beginning with a dot is skipped, a symbolic link is refused, and a folder holding its own project is not entered. Doubt: which files a run reads is the question still open, and the acceptance runs are where a different answer would show, since most of the frontmatter files of one of the two repositories sit in folders whose names begin with a dot.
- **The per-finding lines the design promises after an audit's summary are not built.** No command has its plain text form yet; building one for this alone would be a renderer with no other caller. It is named here so it is not lost.

## Notes
- Run: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (655 passed and 1 ignored, from 646) and `scripts/check-public-text.sh` with the names list, all green, every cargo command under a memory ceiling.
- Re-run by hand, each planted alone inside a function body, red with the error naming the banned call, removed: `std::fs::write` in the building of the audit report, and the same in a test beside it.
- Checked by running, not by reading: the two files of the overlapping fixture counted by hand against the three numbers, and the same sum taken across every project in the fixtures.
