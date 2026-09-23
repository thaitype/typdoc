# Out of Scope

- Remote schemas: `pull`, the fetch adapter, `vendor/` and `lock.json` as things typdoc writes, and the project lock that guards them. They are story 3. The one place they touch this story is the order in which a project lock and a namespace lock may be held, which the design already fixes.
- Comparing numbers that no primitive holds. Two documents whose numbers differ in a digit beyond what a primitive keeps compare equal, so a query that should separate them returns nothing. It is a real defect and it is on the query path, not the write path; this story fixes how such a number is printed and deliberately leaves how it is compared alone.
- The other findings story 1's review left open, none of which the write path introduces or depends on: an invalid `body.links` `ignore` glob dropped silently, the cost of scanning one line for body links, and the recursive walk that overflows the stack on a long ref chain.
- The two known gaps this story does not touch: `[reverse-scope]`, where a reverse lookup does not reach into imported projects, and `[import-anchor]`, where an anchor across an import is not checked.
- A command that deletes a document. v1 has nine commands and none of them deletes one.
- Everything on the design's own "Out of scope for v1" list: editing body sections, SQL queries, saved query aliases, multi-hop ref traversal, and following imports of imports.
- Any release, packaging or distribution of typdoc, including whether the suite runs from a published package.
- A claim that macOS is supported. This story decides what a `mv` does when a file system does not tell two spellings apart, which is a behaviour, not a platform: supporting macOS needs a run on macOS that nobody has made.
- Writing the two sections of user-facing documentation that the map carries as work on the paper: the README's `Concept and Mental model`, and the guidance for a merge conflict in a state file. Neither decides anything this story needs.
