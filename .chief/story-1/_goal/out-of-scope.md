# Out of Scope

- Writing anything: `new`, `set`, `mv` and `mv --renumber`, the frontmatter round trip, locks, the state file's writing, and signal handling. They are story 2.
- Remote schemas fetched by typdoc: `pull`, the fetch adapter, `vendor/` and `lock.json` as things typdoc writes, and the project lock. They are story 3. Story 1 only reads pinned copies that are already there.
- Any release, packaging or distribution of typdoc, including how the tests run from a published package.
- A claim that macOS is supported. The code is written for it, and no run on macOS has shown it.
- A macOS runner and running the public-text gate in CI or a hook.
- The design's own list of what v1 leaves out: editing body sections, SQL queries, saved query aliases, multi-hop ref traversal, and following imports of imports.
- Checking only the schema files named on the command line, and a published meta-schema for editors.
