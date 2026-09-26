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

Done when: that record is reported. The story is accepted outside the loop.
