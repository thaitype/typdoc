# Ticket 21 Report

## Ticket
`validate --audit`, run by hand on copies of the public `chief` and `typmem` repositories, each
with a `.typdoc` folder and local schemas written for the run, and the accounting invariant
checked against a count made without typdoc.

## Outcome
done. Every run accounts for every file it reads, and the lists agree with the independent count
file by file, not only in total.

## Method
- The binary is `target/debug/typdoc` built from `ed3c4c8` on a clean tree.
- Each repository is a fresh copy in a scratch folder outside every repository that is worked
  on: `chief` and `typmem` cloned from the public remote, plus `chief` on its
  `design/v5-ai-workflow` branch, copied from the local repository (see Results for why). The
  local repository and `~/.typmem` were not run against, and `~/.typmem` has no `.typdoc` folder
  and no change after the runs.
- The `.typdoc` folder, the collection files and three schemas (`skill`, `agent`, `plain`) were
  written for the run inside each copy. Nothing was committed to any of those repositories.
- The collections use every kind of match the walk decides: a glob that does not enter a dot
  folder (`docs/**/*.md`), a literal dot folder (`.chief/**/*.md`, `.agents/skills/*/SKILL.md`),
  a `*` that reaches a link (`*.md`, which reaches `CLAUDE.md`), and two collections that reach
  the same file (`skills/*/SKILL.md` and `**/SKILL.md` in `typmem`).
- The independent count is a short script that reads only the `match` strings of the collection
  files and the file system: it walks the copy under the rules of the design's paragraph
  **Which files a run reads**, matches each path against each `match` with its own glob reading,
  and derives the documents checked, the files in no collection, the files with no frontmatter,
  the files matched twice and the entries a link makes unreadable. It never runs typdoc. It is
  kept outside the repository, since this is not part of `cargo test`.
- Before the script, a plain `find` gave the same totals for the two public copies (53 and 21
  regular `.md` files).

## Results
| Copy | Read | Checked | In no collection | No frontmatter | Overlapping | Unreadable | Independent count |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `chief`, public `main` at `4de1630` | 53 | 19 | 2 | 32 | 0 | 1 (`CLAUDE.md`) | 53 |
| `chief`, branch `design/v5-ai-workflow` at `bd82460` | 60 | 26 | 1 | 33 | 0 | 1 (`CLAUDE.md`) | 60 |
| `typmem` at `652e34e` | 21 | 16 | 2 | 0 | 3 | 0 | 21 |

- For each row the sum of the four counts of the summary equals the independent count, and each
  of the three lists (`uncollected`, `no_frontmatter`, `overlapping`), the documents per
  collection and the `files.unreadable` paths equals the independent one. `validate --audit`
  exits 0 in all three, which the design gives it whatever the findings.
- The goal's figures (26 of 61 tracked `.md` files carry frontmatter in `chief`, 23 of 26 in
  `typmem`) match the copies only in part, and the reason is not a fault: the `chief` figures were
  counted on a local branch 34 commits ahead of public `main`, where the count is 61 tracked and
  26 with frontmatter, so the third row is the run that matches them (60 read, the 61st being the
  link, and 26 checked). Public `main` has 54 tracked and 20 with frontmatter. `typmem` has 26
  tracked, of which 5 are links; the 21 regular files are the ones read, and 23 carry
  frontmatter only when the four linked agent files are counted through their targets.
- The comparison was shown able to fail: a copy of the `chief` output with one path dropped from
  `no_frontmatter` and its count lowered gave 52 against 53, and its list no longer equalled the
  independent one.

## Differences from the design found by these runs, not fixed here
- **`audit.collections` leaves out a collection that holds no document, though the design says
  one entry for each collection.** In `typmem` the collections `skills` and `every-skill` reach
  only files that a second collection also reaches, so their files are counted as overlapping
  and neither collection appears. Reproduced on a two-file project with a collection that matches
  nothing: it is absent too. The number a collection reports counts only the files checked
  against its schema, so an overlapping file belongs to no collection's number. The invariant
  holds either way, since overlapping files are counted beside the rest. Whether an empty
  collection should be listed with 0, and whether an overlapping file counts in the number of
  each collection that reaches it, is a decision about what the audit shows. It is not in
  `KNOWN_GAPS`, since nothing pins the present behaviour yet.
- A link to a file that no `match` reaches is silent, as the design has it: `CLAUDE.md` is a
  finding in `chief` because a root `*.md` reaches it, and is not mentioned in `typmem`, which
  has no such collection. The four linked agent files of `typmem` sit in a dot folder that no
  `match` names.

## Notes
- The findings themselves (four `frontmatter.unknown` warnings for the field `model`, the three
  `collections.overlap` errors, the link) are what the schemas written for the run produce. They
  are not a claim about the two repositories.
- Not verified: the runs on a platform other than Linux, and the repositories at any commit other
  than the ones named.
