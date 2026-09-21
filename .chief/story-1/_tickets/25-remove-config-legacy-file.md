# 25: Remove the rule `config.legacy-file`

Type: implementation
Status: open
Blocked by: None (can start immediately)

## What this delivers

typdoc has never been released, so no file in the old format exists anywhere, and the rule guards against something that has not happened to anyone. It also guards in the wrong place. Run on 2026-09-21:

- A project whose `.typdoc/config.json` reads fine and which has a stray `.typdoc.json` beside the folder: every command stops with exit 2, on a config that could be read.
- A folder with only `.typdoc.json` and no `.typdoc` folder: the rule does not run, and the answer is the same "no project found" as for an empty folder, so the advice reaches nobody who needs it.

The rule is removed with everything that belongs to it, and the "no project found" error carries what the rule was trying to say. The design already says both (Discovery and the config errors table).

- The constant `LEGACY_FILE`, the check that reads it, and the comments that speak of it in `crates/typdoc-core/src/config.rs`.
- The id `config.legacy-file` in the list of rules (`crates/typdoc-core/src/rules.rs`), the fixture `fixtures/broken/config.legacy-file/`, and every test that uses the id, in `crates/typdoc/tests/config.rs` and wherever else it appears. A test that mixed it with another config error keeps its other half.
- The "no project found" errors (`Error::NoProject`, `Error::NoProjectAt`) already name `.typdoc/config.json` as the file looked for. Keep that, and pin it: a test for a folder with no project, for one that holds only a `.typdoc.json`, and for `TYPDOC_DIR` naming a folder without a config, each asserting the message names `.typdoc/config.json` and the exit code is unchanged. No check for any other file name is added.

## Done when

- `grep -rn 'legacy' crates fixtures docs/design.md` finds nothing that names the rule; the coverage checks over the rules and the fixtures are green with the rule gone.
- A test fails before the change and passes after it: a project with a readable `.typdoc/config.json` and a stray `.typdoc.json` validates and lists as if the stray file were not there.
- The three "no project found" cases above are pinned.
- Older reports and tickets that mention the rule are left as they are.
