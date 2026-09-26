Type: implementation
Status: open
Blocked by: 06, and the batch before it has merged, and the pilot's judgment is accepted

# Ticket 07 — `typdoc/tests`, write commands

Scope, `crates/typdoc/tests/`: `new.rs`, `set.rs`, `mv.rs`, `mv_renumber.rs`, `state.rs`,
`lock_contention.rs`, `signals.rs`, `templates.rs`, `frontmatter_scalars.rs`, `common/mod.rs`,
`support/`. Branch from `origin/main`. Most `//! Covers SPC-N.` lines for the write commands land
here.

Follows the contract in full: the `comment-review` skill on every comment in scope, the three
citation cases, commit order (design changes, then comments), `typdoc validate` at every commit,
the comment-only proof in the PR body, the Investigate list, and the public-text rule for
everything added. Nothing on the Investigate list is fixed.

Done when: the PR is open from its own branch, CI is green on ubuntu and macOS, and the PR and
head commit are reported. The loop does not merge.
