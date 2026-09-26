Type: implementation
Status: open
Blocked by: 01

# Ticket 02 — pilot: `typdoc-fs`, `typdoc-testkit`, `json_body.rs`

Scope: `crates/typdoc-fs` (src and tests), `crates/typdoc-testkit/src`,
`crates/typdoc-core/src/json_body.rs`, and the `#` comments in those crates' `Cargo.toml` and
`clippy.toml`.

The pilot must show all three citation cases (already in spec, moved into an SPC, process only);
`json_body.rs` holds six citations for that reason. If the scope turns out not to contain one of
the three, say so in the PR body rather than reaching outside the scope for an example.

Before the proof is used, show it red: a planted one-token code change reported as a
difference, a comment-only change not, recorded in the PR body.

Opens the pilot PR from `story-6-comment-review` to `main`; the PR also carries
`.chief/story-6/` and ticket 01's rules.

Follows the contract in full: the `comment-review` skill on every comment in scope, the three
citation cases, commit order (design changes, then comments), `typdoc validate` at every commit,
the comment-only proof in the PR body, the Investigate list, and the public-text rule for
everything added. Nothing on the Investigate list is fixed.

Done when: the PR is open from its own branch, CI is green on ubuntu and macOS, and the PR and
head commit are reported. The loop does not merge.

**After this ticket the loop stops.** No later ticket starts until the pilot's judgment is
accepted; changes that review asks for are applied here first and carried into later tickets.
