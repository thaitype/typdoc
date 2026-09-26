Type: implementation
Status: resolved
Blocked by: 04, and the batch before it has merged, and the pilot's judgment is accepted

# Ticket 05 — `typdoc-core` read path

Scope: every file in `crates/typdoc-core/src/` and `crates/typdoc-core/tests/` not covered by
tickets 02, 03 and 04 (the read side: `refs`, `links`, `validate`, `query`, `schema`, `index`,
`config`, `argument`, `document`, `imports`, `namespaces`, `scope`, `error`, `lines`, `slug`,
`coerce`, `rules`, `body`, `lib`, and their tests, `tests/common/mod.rs` included). Branch from
`origin/main`.

Follows the contract in full: the `comment-review` skill on every comment in scope, the three
citation cases, commit order (design changes, then comments), `typdoc validate` at every commit,
the comment-only proof in the PR body, the Investigate list, and the public-text rule for
everything added. Nothing on the Investigate list is fixed.

Done when: the PR is open from its own branch, CI is green on ubuntu and macOS, and the PR and
head commit are reported. The loop does not merge.
