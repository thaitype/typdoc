Type: implementation
Status: claimed
Blocked by: 07, and the batch before it has merged, and the pilot's judgment is accepted

# Ticket 08 — `typdoc/tests`, read commands and the rest

Scope: every file in `crates/typdoc/tests/` not covered by ticket 07 (`validate.rs`,
`imports.rs`, `list.rs`, `refs.rs`, `get.rs`, `toc.rs`, `body.rs`, `arguments.rs`, `config.rs`,
`namespaces.rs`, `outside_namespaces.rs`, `schemas.rs`, `examples.rs`, `golden.rs`,
`coverage.rs`, `shell_examples.rs`). Branch from `origin/main`.

Follows the contract in full: the `comment-review` skill on every comment in scope, the three
citation cases, commit order (design changes, then comments), `typdoc validate` at every commit,
the comment-only proof in the PR body, the Investigate list, and the public-text rule for
everything added. Nothing on the Investigate list is fixed.

Done when: the PR is open from its own branch, CI is green on ubuntu and macOS, and the PR and
head commit are reported. The loop does not merge.
