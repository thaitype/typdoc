# 21: Acceptance runs on copies of `chief` and `typmem`

Type: implementation
Status: resolved
Blocked by: 19, 20 and the decision on which files a run reads

## What this delivers

- `validate --audit` run by hand on a copy of the public `chief` repository and on a copy of the public `typmem` repository, each with a `.typdoc` folder and local schemas written for the run, and the accounting invariant checked against an independent count. The results go in the story's closing report.

## Done when

- Both runs account for every file the run reads. This is not part of `cargo test`, since the repositories are not fixtures.
