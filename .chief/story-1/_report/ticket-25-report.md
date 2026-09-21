# Ticket 25 Report

## Ticket
Remove the rule `config.legacy-file` and everything that belongs to it, and pin what the "no project
found" error says: it names `.typdoc/config.json` as the file looked for.

## Outcome
done. Before the change, a project with a readable `.typdoc/config.json` and a stray `.typdoc.json`
beside the folder made `validate` and `list` exit 2 with `config.legacy-file`. Now both run as if the
stray file were not there: `validate` checks the two documents with no findings and `list` lists them.
The stray file is not read, whatever it holds (a valid config, `{}`, invalid JSON, empty).

Removed: the constant `LEGACY_FILE` and the check that read it in `crates/typdoc-core/src/config.rs`,
the comments there that spoke of the rule, the id in `crates/typdoc-core/src/rules.rs`, the fixture
folder `fixtures/broken/config.legacy-file/`, and the design's table row (the Discovery sentence was
rewritten in the same edit). `grep -rni legacy crates fixtures docs/design.md scripts examples` finds
nothing. Older reports and tickets that mention the rule are left as they are.

Suite: 698 passed, 0 failed, 1 ignored (695 before). `cargo fmt --check`, clippy with `-D warnings` and
the public-text gate are clean.

Tests removed, by name (`crates/typdoc/tests/config.rs`):
- `a_legacy_config_beside_the_folder_is_config_legacy_file_and_says_where_to_move_it`: it asserted
  the rule and nothing else.
- `a_legacy_config_is_reported_together_with_a_config_that_cannot_be_parsed`: replaced by
  `a_stray_typdoc_json_is_not_reported_with_a_config_that_cannot_be_parsed`, which keeps the
  `config.parse` half and asserts the stray file adds nothing to it.

Tests changed: `every_error_that_can_be_determined_is_reported_in_one_object_in_the_order_of_path_rule_message`
and `a_config_error_in_a_fixture_names_the_file_it_is_about` lose their `config.legacy-file` entry and
keep every other one.

Tests added:
- `a_stray_typdoc_json_beside_the_folder_changes_nothing_for_validate_and_list`: red on the unmodified
  tree (exit 2, `config.legacy-file`), green after.
- `an_empty_folder_is_no_project_and_the_message_names_the_config_file`,
  `a_folder_holding_only_a_typdoc_json_is_no_project_and_the_message_names_the_config_file` and
  `typdoc_dir_naming_a_folder_without_a_config_is_no_project_and_names_the_config_file`: each asserts
  exit 5, an error object with no `complete` key, and `.typdoc/config.json` in the message. These three
  passed on the unmodified tree, as they pin what the code already did.

Each pinning test was shown able to fail (all restored afterwards):
- A check for `.typdoc.json` put back in `Config::load`: both stray-file tests red.
- `NoProject` message without the file name: the empty-folder and only-`.typdoc.json` tests red.
- `NoProjectAt` message without the file name: the `TYPDOC_DIR` test red.
- `NoProject` message that mentions `.typdoc.json`: the only-`.typdoc.json` test red.

The read-only guard of `typdoc-core` was checked against `config.rs`: `std::fs::write` planted in the body
of `Config::load` fails clippy with `use of a disallowed method 'std::fs::write'`; removed again.

## Decision if any
- **Message wording.** Decided: `Error::NoProject` and `Error::NoProjectAt` are not changed. They already
  say `there is no .typdoc/config.json in <dir> or above it` and `<dir> has no .typdoc/config.json`,
  which is what the design asks for, and no test forced a change.
- **Helper.** Default: `Scratch::empty()` (a folder with nothing in it) was added to the shared test
  helper, because `Scratch::project` always writes a config; `get.rs` gained the same
  `#[allow(dead_code)]` on `mod common;` that every other test file already has, since it does not use
  the new function.
- **Report doc comment.** Default: the doc comment of `Report` keeps its general statement that no error
  added there answers the design's question on its own, and drops only the sentences about the removed
  rule.

## Notes
- Consumers checked: the list of rules and the two coverage checks over rules and fixtures (green with
  one rule and one fixture fewer); `Config::load` and `Report::finish` (the only stop-or-continue
  decision is unchanged, and nothing in `load` or `finish` existed only for the removed rule); the
  discovery functions `discover` and `discover_for` (untouched); the goldens and the shell examples (no
  mention of the rule or the file); `fixtures/broken/` and the tests that list fixtures (the config
  fixture tests iterate the folder, so they follow the removal).
- Not verified: the empty-folder test assumes no folder above the system temporary directory holds a
  `.typdoc/config.json`.
- `.chief/story-1/_tickets/4-config-and-namespaces.md` and the reports of tickets 4 and 8 and the
  story's closing report still describe the rule as it was; they are dated records and stay.
