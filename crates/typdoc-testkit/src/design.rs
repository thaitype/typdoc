//! What `docs/design.md` names, read from the document itself so that a test compares the
//! code with the document and not with a second copy of it.

use std::collections::BTreeSet;

/// The ids in the tables of the section "Validation rules": the always-on and configurable
/// rules, and the config errors, which carry an id in `details[].rule` as a rule does.
pub fn rule_ids(design: &str) -> Result<BTreeSet<String>, String> {
    let mut ids = BTreeSet::new();
    for table in tables(&section(design, "Validation rules")) {
        if !matches!(table.header.as_str(), "Rule" | "Id") {
            continue;
        }
        for cell in table.first_cells {
            ids.insert(rule_id(&cell)?);
        }
    }
    if ids.is_empty() {
        return Err("no table headed Rule or Id is in the section Validation rules".into());
    }
    Ok(ids)
}

/// The ids of the table of always-on rules: the table headed `Rule` whose second column is
/// `Checks`.
pub fn always_on_rule_ids(design: &str) -> Result<BTreeSet<String>, String> {
    rules_of_table(design, "Checks")
}

/// The ids of the table of configurable rules: the table headed `Rule` whose second column is
/// `Default`.
pub fn configurable_rule_ids(design: &str) -> Result<BTreeSet<String>, String> {
    rules_of_table(design, "Default")
}

fn rules_of_table(design: &str, second_column: &str) -> Result<BTreeSet<String>, String> {
    let mut ids = BTreeSet::new();
    for table in tables(&section(design, "Validation rules")) {
        if table.header == "Rule" && table.second == second_column {
            for cell in table.first_cells {
                ids.insert(rule_id(&cell)?);
            }
        }
    }
    if ids.is_empty() {
        return Err(format!(
            "no table headed Rule with the second column {second_column} is in the section Validation rules"
        ));
    }
    Ok(ids)
}

/// The commands that have a heading `### typdoc <name>` in the section "Commands".
pub fn command_names(design: &str) -> Result<BTreeSet<String>, String> {
    let names: BTreeSet<String> = section(design, "Commands")
        .iter()
        .filter_map(|line| line.strip_prefix("### typdoc "))
        .map(|name| name.trim().to_owned())
        .collect();
    if names.is_empty() {
        return Err("no heading `### typdoc <name>` is in the section Commands".into());
    }
    Ok(names)
}

/// The codes in the table of the section "Exit codes and errors".
pub fn exit_codes(design: &str) -> Result<BTreeSet<u8>, String> {
    let mut codes = BTreeSet::new();
    for table in tables(&section(design, "Exit codes and errors")) {
        if table.header != "Code" {
            continue;
        }
        for cell in table.first_cells {
            let code = cell
                .parse::<u8>()
                .map_err(|_| format!("the exit code `{cell}` is not a number"))?;
            codes.insert(code);
        }
    }
    if codes.is_empty() {
        return Err("no table headed Code is in the section Exit codes and errors".into());
    }
    Ok(codes)
}

/// What a write does not keep, from the table in the section "Document files": the first cell
/// of each row of the table headed "Written in the file", whose second column is "After any
/// write". A test that wants to know whether a shape is covered reads this rather than copying
/// the table a second time, so a row the design adds or removes is a row the test sees too.
pub fn frontmatter_losses(design: &str) -> Result<Vec<String>, String> {
    for table in tables(&section(design, "Document files")) {
        if table.header == "Written in the file" && table.second == "After any write" {
            if table.first_cells.is_empty() {
                return Err("the table of frontmatter losses has no rows".into());
            }
            return Ok(table.first_cells);
        }
    }
    Err(
        "no table headed `Written in the file` with the second column `After any write` is in \
         the section Document files"
            .into(),
    )
}

/// The lines of the section `## <title>`, outside code fences, up to the next `## `.
fn section(design: &str, title: &str) -> Vec<String> {
    let heading = format!("## {title}");
    let mut in_fence = false;
    let mut inside = false;
    let mut lines = Vec::new();
    for line in design.lines() {
        if line.starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        if line.starts_with("## ") {
            inside = line.trim_end() == heading;
            continue;
        }
        if inside {
            lines.push(line.to_owned());
        }
    }
    lines
}

struct Table {
    header: String,
    second: String,
    first_cells: Vec<String>,
}

/// The pipe tables among `lines`: the first cell of the header, and the first cell of each row.
fn tables(lines: &[String]) -> Vec<Table> {
    let mut tables = Vec::new();
    let mut open: Option<Table> = None;
    let mut rows_seen = 0;
    for line in lines {
        if !line.starts_with('|') {
            tables.extend(open.take());
            continue;
        }
        let mut cells = line.trim_start_matches('|').split('|').map(str::trim);
        let first = cells.next().unwrap_or("").to_owned();
        match &mut open {
            None => {
                open = Some(Table {
                    header: first,
                    second: cells.next().unwrap_or("").to_owned(),
                    first_cells: Vec::new(),
                });
                rows_seen = 0;
            }
            Some(table) => {
                rows_seen += 1;
                // The first line under the header is the separator.
                if rows_seen > 1 {
                    table.first_cells.push(first);
                }
            }
        }
    }
    tables.extend(open);
    tables
}

fn rule_id(cell: &str) -> Result<String, String> {
    let id = cell
        .strip_prefix('`')
        .and_then(|rest| rest.strip_suffix('`'))
        .filter(|id| !id.contains('`'))
        .ok_or_else(|| {
            format!("the first cell {cell} of a rule table is not one id in backticks")
        })?;
    let name_part = |part: &str| {
        part.starts_with(|c: char| c.is_ascii_lowercase())
            && part.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
    };
    if id.contains('.') && id.split('.').all(name_part) {
        Ok(id.to_owned())
    } else {
        Err(format!("`{id}` is not a dotted rule id"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set<T: Ord + Clone>(items: &[T]) -> BTreeSet<T> {
        items.iter().cloned().collect()
    }

    fn strings(items: &[&str]) -> BTreeSet<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    const RULES: &str = "\
# Title

## Validation rules

Some text.

| Rule | Checks |
| --- | --- |
| `schema.valid` | Duplicate names |
| `refs.resolve` | Frontmatter refs |

| Rule | Default | Options | Checks |
| --- | --- | --- | --- |
| `body.links` | `error` | `ignore` | Links |

| Text | Checked |
| --- | --- |
| `` `WF-3` `` (inline code) | per `inlineCode` |

| Id | Reported when |
| --- | --- |
| `config.parse` | `config.json` cannot be parsed |

## Concurrency

| Rule | Checks |
| --- | --- |
| `not.in.the.section` | x |
";

    #[test]
    fn rule_ids_come_from_the_tables_headed_rule_or_id_in_the_rules_section_only() {
        assert_eq!(
            rule_ids(RULES).unwrap(),
            strings(&["schema.valid", "refs.resolve", "body.links", "config.parse"])
        );
    }

    #[test]
    fn the_two_tables_of_rules_are_told_apart_by_their_second_column() {
        assert_eq!(
            always_on_rule_ids(RULES).unwrap(),
            strings(&["schema.valid", "refs.resolve"])
        );
        assert_eq!(
            configurable_rule_ids(RULES).unwrap(),
            strings(&["body.links"])
        );
    }

    #[test]
    fn a_design_without_one_of_the_two_tables_is_an_error() {
        let only_always_on =
            "## Validation rules\n\n| Rule | Checks |\n| --- | --- |\n| `a.b` | x |\n";

        assert!(always_on_rule_ids(only_always_on).is_ok());
        assert!(configurable_rule_ids(only_always_on).is_err());
    }

    #[test]
    fn a_table_row_that_holds_no_single_id_is_an_error_and_not_skipped() {
        let design =
            "## Validation rules\n\n| Rule | Checks |\n| --- | --- |\n| `a.b` and `c.d` | x |\n";

        let error = rule_ids(design).unwrap_err();

        assert!(error.contains("`a.b` and `c.d`"), "{error}");
    }

    #[test]
    fn an_id_that_is_not_a_dotted_name_is_an_error() {
        let design =
            "## Validation rules\n\n| Rule | Checks |\n| --- | --- |\n| `Schema.valid` | x |\n";

        assert!(rule_ids(design).is_err());
    }

    #[test]
    fn a_design_with_no_rule_table_is_an_error_and_not_an_empty_set() {
        assert!(rule_ids("# Title\n\n## Validation rules\n\nNo table.\n").is_err());
        assert!(rule_ids("# Title\n").is_err());
    }

    #[test]
    fn a_table_inside_a_code_fence_is_not_read() {
        let design = "## Validation rules\n\n```\n| Rule | Checks |\n| --- | --- |\n| `in.fence` | x |\n```\n\n| Rule | Checks |\n| --- | --- |\n| `real.one` | x |\n";

        assert_eq!(rule_ids(design).unwrap(), strings(&["real.one"]));
    }

    const COMMANDS: &str = "\
## Commands

Text.

### typdoc new

```bash
# stdout: WF-3
### typdoc not-a-command
```

### typdoc get

## Validation rules

### typdoc elsewhere
";

    #[test]
    fn commands_are_the_headings_in_the_commands_section_outside_code_fences() {
        assert_eq!(command_names(COMMANDS).unwrap(), strings(&["new", "get"]));
    }

    #[test]
    fn a_design_with_no_command_heading_is_an_error() {
        assert!(command_names("## Commands\n\nNone.\n").is_err());
    }

    #[test]
    fn exit_codes_are_the_first_column_of_the_table_headed_code() {
        let design = "\
## Exit codes and errors

| Code | Meaning |
| --- | --- |
| 0 | Success |
| 5 | Not found |

## Worked examples

| Code | Meaning |
| --- | --- |
| 9 | elsewhere |
";

        assert_eq!(exit_codes(design).unwrap(), set(&[0u8, 5]));
    }

    #[test]
    fn an_exit_code_that_is_not_a_number_is_an_error() {
        let design = "## Exit codes and errors\n\n| Code | Meaning |\n| --- | --- |\n| x | y |\n";

        assert!(exit_codes(design).is_err());
    }

    #[test]
    fn the_real_design_names_what_it_is_known_to_name() {
        let design = crate::fixtures::design_text();

        let rules = rule_ids(&design).unwrap();
        for id in [
            "frontmatter.transitions",
            "refs.codedByPath",
            "state.missing",
            "body.links",
            "imports.absent",
            "config.parse",
            "config.config-dir",
        ] {
            assert!(rules.contains(id), "{id} is not read from the design");
        }
        assert_eq!(
            command_names(&design).unwrap(),
            strings(&[
                "new", "get", "list", "set", "toc", "refs", "mv", "pull", "validate"
            ])
        );
        assert_eq!(
            exit_codes(&design).unwrap(),
            set(&[0u8, 1, 2, 3, 4, 5, 6, 7])
        );
        assert!(
            always_on_rule_ids(&design)
                .unwrap()
                .contains("state.missing")
        );
        assert!(
            configurable_rule_ids(&design)
                .unwrap()
                .contains("body.links")
        );
        assert_eq!(
            frontmatter_losses(&design).unwrap(),
            vec![
                "Comments, anywhere in the block".to_owned(),
                "Blank lines between fields".to_owned(),
                "`tags: [a, b]`".to_owned(),
                "`title: 'Ship it'`, `status: \"no\"`".to_owned(),
                "`id:   WF-3`".to_owned(),
                "`&anchor` with `*alias`".to_owned(),
                "`!!str`, `!Ref`, any other tag".to_owned(),
            ]
        );
    }

    const LOSSES: &str = "\
# Title

## Document files

Some text.

**Rules**

- A write rewrites the whole block:

| Written in the file | After any write |
| --- | --- |
| Comments, anywhere in the block | Gone |
| `tags: [a, b]` | A block list |

## Refs

| Written in the file | After any write |
| --- | --- |
| `elsewhere` | not read |
";

    #[test]
    fn frontmatter_losses_come_from_the_table_in_document_files_only() {
        assert_eq!(
            frontmatter_losses(LOSSES).unwrap(),
            vec![
                "Comments, anywhere in the block".to_owned(),
                "`tags: [a, b]`".to_owned(),
            ]
        );
    }

    #[test]
    fn a_design_with_no_losses_table_is_an_error_and_not_an_empty_list() {
        assert!(frontmatter_losses("# Title\n\n## Document files\n\nNo table.\n").is_err());
        assert!(frontmatter_losses("# Title\n").is_err());
    }
}
