Type: implementation
Status: open
Blocked by: 08, and every PR merged

# Ticket 09 — story close check

No code or comment changes. On `origin/main` after every batch has merged:

- a repository-wide search of `crates/` finds no `ticket N`, `decision N`, `M-N`, `.chief/`,
  `docs/design.md`, `migrating-design` or `archived-design` citation; the search and its empty
  result are recorded;
- every Investigate item from every batch is listed in one place with its decision, or named as
  still waiting for one.

Known so far: `crates/typdoc-core/src/json_body.rs`, the test named
`a_document_with_no_frontmatter_block_at_all_is_reported_as_its_own_case` asserts
`MissingContentType` (pilot batch). Fixed in the ticket 03 PR: the test is renamed to name what it
asserts.

From the ticket 03 PR, deferred by decision to later work (the design text stays in
`docs/migrating-design/`): the `git-common` lock mode, which `mv` does not check; reverse scans
across imported projects (`refs --reverse`, `refby`, `mv`); query scope across imported projects;
`body.mentions` with imported projects; the loose lock taken for a file inside a namespace folder.

From the ticket 04 PR: `pid_alive` has no `/proc` on macOS, so a live lock owner is reported as
stale. Deferred by decision (fix right after the story). The release result that is discarded
while the design text says a lock taken away is reported: deferred by decision; the design text
stays in `docs/migrating-design/`.

From the ticket 05 PR. Deferred by decision (bug fix after the story): a body link under
`refBase: namespace` resolves against the namespace folder while the design says a body link is
relative to the document; `mv` neither rewrites nor reports reference-style links and their
definitions; `config.state-orphan` stops the command with exit 2 while SPC-8 and SPC-6 say it
stops nothing (the code is to change). Deferred by decision, design text left in
`docs/migrating-design/`: a relative `extends` in a pinned schema resolves against the project
folder rather than its URL; the text audit prints no finding lines while the design's audit table
says it does; machine-file errors do not name the lookup step. Fixed in the ticket 05 PR: SPC-4
no longer says a `frontmatter.parse` finding has a position; the `refs.rs` test named as if the
import form were not read is renamed for what it asserts.

Done when: that record is reported. The story is accepted outside the loop.
