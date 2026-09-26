Type: implementation
Status: open
Blocked by: 05, and the batch before it has merged, and the pilot's judgment is accepted

# Ticket 06 — `typdoc/src`

Scope: `crates/typdoc/src/` (`cli.rs` holds 23 history citations) and the `#` comments in
`typdoc`'s `Cargo.toml` and `clippy.toml`. Branch from `origin/main`.

Follows the contract in full: the `comment-review` skill on every comment in scope, the three
citation cases, commit order (design changes, then comments), `typdoc validate` at every commit,
the comment-only proof in the PR body, the Investigate list, and the public-text rule for
everything added. Nothing on the Investigate list is fixed.

Done when: the PR is open from its own branch, CI is green on ubuntu and macOS, and the PR and
head commit are reported. The loop does not merge.
