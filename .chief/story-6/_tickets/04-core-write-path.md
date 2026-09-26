Type: implementation
Status: claimed
Blocked by: 03, and the batch before it has merged, and the pilot's judgment is accepted

# Ticket 04 — `typdoc-core` write path

Scope, `crates/typdoc-core/src/`: `namespace_lock.rs`, `lock.rs`, `fs.rs`, `state.rs`, `mv.rs`,
`frontmatter.rs`, `clock.rs`, `template.rs`, `env.rs`. `crates/typdoc-core/tests/`:
`allocation_race.rs`, `git_common_duplicate.rs`, `mv_seam.rs`, `mv_renumber_seam.rs`,
`namespace_lock.rs`, `namespace_lock_compile_fail.rs`, `deps.rs`, `trybuild/*`; and the `#`
comments in `typdoc-core`'s `Cargo.toml` and `clippy.toml`. Branch from `origin/main`.

`trybuild` `.stderr` files may change only in line and column numbers.

Follows the contract in full: the `comment-review` skill on every comment in scope, the three
citation cases, commit order (design changes, then comments), `typdoc validate` at every commit,
the comment-only proof in the PR body, the Investigate list, and the public-text rule for
everything added. Nothing on the Investigate list is fixed.

Done when: the PR is open from its own branch, CI is green on ubuntu and macOS, and the PR and
head commit are reported. The loop does not merge.
