//! The checks that compare the design, the code and the fixtures. Each takes plain sets, so
//! a test can hand it a fault and see it refuse.

use std::collections::BTreeSet;
use std::fmt::Display;

/// What a list of differences is about, as the messages name it.
pub struct Kind<'a> {
    /// What the items are: `command`, `rule`, `exit code`.
    pub thing: &'a str,
    /// Where the items that exist are found: `the registry`, `the codes a test produces`.
    pub present: &'a str,
    /// The name of the list in code.
    pub list: &'a str,
}

/// The two checks of a list of differences between the design and the binary, for anything the
/// design names.
///
/// `present` is what the code has (the commands the CLI has, the rules the binary reports, the
/// exit codes a test produces) and `listed` is the list. Everything the design names is present
/// or listed. A listed entry is not present, and is still named by the design: an entry that
/// outlives its work is red at once, so a list cannot be a way out.
pub fn acknowledged<T: Ord + Display>(
    kind: &Kind,
    named_by_design: &BTreeSet<T>,
    present: &BTreeSet<T>,
    listed: &BTreeSet<T>,
) -> Result<(), String> {
    let Kind {
        thing,
        present: place,
        list,
    } = kind;
    let mut problems = Vec::new();
    for name in named_by_design {
        if !present.contains(name) && !listed.contains(name) {
            problems.push(format!(
                "the design names {thing} {name}, and it is in neither {place} nor {list}"
            ));
        }
    }
    for name in listed {
        if present.contains(name) {
            problems.push(format!(
                "{thing} {name} is in {list} and is also in {place}: take it out of the list"
            ));
        } else if !named_by_design.contains(name) {
            problems.push(format!(
                "{thing} {name} is in {list} and the design does not name it"
            ));
        }
    }
    for name in present {
        if !named_by_design.contains(name) {
            problems.push(format!(
                "{thing} {name} is in {place} and the design does not name it"
            ));
        }
    }
    finish(problems)
}

/// Every entry of `fixtures/broken/` names a rule that exists, and every rule that exists has
/// an entry. A rule that is listed as not built exists in neither sense, so it needs no fixture
/// and a fixture for it is refused.
pub fn broken_coverage(
    entries: &[String],
    built: &BTreeSet<String>,
    listed: &BTreeSet<String>,
) -> Result<(), String> {
    let mut problems = Vec::new();
    for entry in entries {
        if built.contains(entry) {
            continue;
        }
        if listed.contains(entry) {
            problems.push(format!(
                "fixtures/broken/{entry} is for a rule that is not built: it is in the list of unimplemented rules"
            ));
        } else {
            problems.push(format!("fixtures/broken/{entry} names no rule"));
        }
    }
    for rule in built {
        if !entries.contains(rule) {
            problems.push(format!(
                "the rule {rule} exists and has no fixture in fixtures/broken/"
            ));
        }
    }
    finish(problems)
}

/// The rules a fixture trips are exactly the rules it declares: no more, no fewer.
pub fn exact_set(
    fixture: &str,
    expected: &BTreeSet<String>,
    tripped: &BTreeSet<String>,
) -> Result<(), String> {
    let mut problems = Vec::new();
    for rule in expected.difference(tripped) {
        problems.push(format!("fixture {fixture} did not trip {rule}"));
    }
    for rule in tripped.difference(expected) {
        problems.push(format!(
            "fixture {fixture} also tripped {rule}, which it does not declare"
        ));
    }
    finish(problems)
}

/// The rule ids in what a command printed: `findings[].rule` on standard output and
/// `details[].rule` in the error object on standard error.
pub fn tripped_rules(stdout: &str, stderr: &str) -> Result<BTreeSet<String>, String> {
    let mut rules = BTreeSet::new();
    for (stream, text, list) in [
        ("stdout", stdout, "findings"),
        ("stderr", stderr, "details"),
    ] {
        if text.trim().is_empty() {
            continue;
        }
        let printed: serde_json::Value = serde_json::from_str(text)
            .map_err(|e| format!("{stream} is not JSON ({e}): {text:?}"))?;
        let Some(items) = printed.get(list) else {
            continue;
        };
        let items = items
            .as_array()
            .ok_or_else(|| format!("`{list}` on {stream} is not an array"))?;
        for item in items {
            let rule = item
                .get("rule")
                .and_then(|rule| rule.as_str())
                .ok_or_else(|| format!("an item of `{list}` on {stream} has no `rule`: {item}"))?;
            rules.insert(rule.to_owned());
        }
    }
    Ok(rules)
}

fn finish(problems: Vec<String>) -> Result<(), String> {
    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const COMMANDS: Kind = Kind {
        thing: "command",
        present: "the registry",
        list: "unimplemented_commands",
    };

    const CODES: Kind = Kind {
        thing: "exit code",
        present: "the codes a test produces",
        list: "unproduced_exit_codes",
    };

    fn set(items: &[&str]) -> BTreeSet<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    fn acknowledged_strings(
        design: &[&str],
        present: &[&str],
        listed: &[&str],
    ) -> Result<(), String> {
        acknowledged(&COMMANDS, &set(design), &set(present), &set(listed))
    }

    #[test]
    fn a_design_that_is_covered_by_the_registry_and_the_list_passes() {
        assert_eq!(
            acknowledged_strings(&["get", "new", "set"], &["get"], &["new", "set"]),
            Ok(())
        );
    }

    #[test]
    fn a_name_in_the_design_that_is_in_neither_place_is_refused() {
        let error = acknowledged_strings(&["get", "new"], &["get"], &[]).unwrap_err();

        assert!(error.contains("new"), "{error}");
        assert!(error.contains("neither"), "{error}");
    }

    #[test]
    fn an_entry_that_is_built_and_left_in_the_list_is_refused() {
        let error = acknowledged_strings(&["get", "new"], &["get", "new"], &["new"]).unwrap_err();

        assert!(error.contains("new"), "{error}");
        assert!(error.contains("unimplemented_commands"), "{error}");
        assert!(error.contains("take it out"), "{error}");
    }

    #[test]
    fn an_entry_the_design_no_longer_names_is_refused() {
        let error = acknowledged_strings(&["get"], &["get"], &["gone"]).unwrap_err();

        assert!(error.contains("gone"), "{error}");
    }

    #[test]
    fn something_present_that_the_design_does_not_name_is_refused() {
        let error = acknowledged_strings(&["get"], &["get", "extra"], &[]).unwrap_err();

        assert!(error.contains("extra"), "{error}");
    }

    #[test]
    fn every_problem_is_reported_and_not_only_the_first() {
        let error = acknowledged_strings(&["a", "b"], &[], &["c"]).unwrap_err();

        assert!(
            error.contains('a') && error.contains('b') && error.contains('c'),
            "{error}"
        );
    }

    #[test]
    fn numbers_are_checked_the_same_way() {
        let design: BTreeSet<u8> = [0, 3, 4].into();
        let present: BTreeSet<u8> = [0, 3].into();
        let listed: BTreeSet<u8> = [3, 4].into();

        let error = acknowledged(&CODES, &design, &present, &listed).unwrap_err();

        assert!(error.contains("exit code 3"), "{error}");
        assert!(!error.contains("exit code 4"), "{error}");
        assert!(error.contains("the codes a test produces"), "{error}");
    }

    #[test]
    fn a_rule_built_has_a_fixture_and_a_rule_not_built_has_none() {
        let entries = vec!["body.links".to_string()];

        assert_eq!(
            broken_coverage(
                &entries,
                &set(&["body.links"]),
                &set(&["frontmatter.transitions"])
            ),
            Ok(())
        );
    }

    #[test]
    fn a_folder_that_names_no_rule_is_refused() {
        let entries = vec!["body.links".to_string(), "no.such-rule".to_string()];

        let error = broken_coverage(&entries, &set(&["body.links"]), &set(&[])).unwrap_err();

        assert!(error.contains("no.such-rule"), "{error}");
        assert!(error.contains("names no rule"), "{error}");
    }

    #[test]
    fn a_rule_that_exists_and_has_no_fixture_is_refused() {
        let error = broken_coverage(&[], &set(&["body.links"]), &set(&[])).unwrap_err();

        assert!(error.contains("body.links"), "{error}");
        assert!(error.contains("no fixture"), "{error}");
    }

    #[test]
    fn a_folder_for_a_rule_that_is_listed_as_not_built_is_refused() {
        let entries = vec!["frontmatter.transitions".to_string()];

        let error =
            broken_coverage(&entries, &set(&[]), &set(&["frontmatter.transitions"])).unwrap_err();

        assert!(error.contains("frontmatter.transitions"), "{error}");
        assert!(error.contains("not built"), "{error}");
    }

    #[test]
    fn a_fixture_that_trips_a_second_rule_is_refused() {
        let error = exact_set(
            "body.links",
            &set(&["body.links"]),
            &set(&["body.links", "body.anchors"]),
        )
        .unwrap_err();

        assert!(error.contains("body.anchors"), "{error}");
        assert!(error.contains("body.links"), "{error}");
    }

    #[test]
    fn a_fixture_whose_rule_does_not_fire_is_refused() {
        let error = exact_set("body.links", &set(&["body.links"]), &set(&[])).unwrap_err();

        assert!(error.contains("did not trip"), "{error}");
    }

    #[test]
    fn a_fixture_that_trips_exactly_what_it_declares_passes() {
        assert_eq!(
            exact_set("a.b", &set(&["a.b", "c.d"]), &set(&["c.d", "a.b"])),
            Ok(())
        );
    }

    #[test]
    fn rules_are_read_from_findings_on_stdout_and_details_on_stderr() {
        let stdout = r#"{ "summary": {}, "findings": [
            { "level": "error", "rule": "body.links", "message": "m" },
            { "level": "warn", "rule": "body.links", "message": "n" } ] }"#;
        let stderr = r#"{ "error": "e", "code": 2, "details": [
            { "level": "error", "rule": "config.parse", "message": "m" } ] }"#;

        assert_eq!(
            tripped_rules(stdout, stderr).unwrap(),
            set(&["body.links", "config.parse"])
        );
    }

    #[test]
    fn a_run_that_printed_no_rule_trips_none() {
        assert_eq!(
            tripped_rules(r#"{ "document": {} }"#, "").unwrap(),
            set(&[])
        );
        assert_eq!(
            tripped_rules("", r#"{ "error": "e", "code": 5, "details": [] }"#).unwrap(),
            set(&[])
        );
    }

    #[test]
    fn output_that_is_not_json_or_a_finding_with_no_rule_is_an_error() {
        assert!(tripped_rules("text", "").is_err());
        assert!(tripped_rules("", "typdoc: message").is_err());
        assert!(tripped_rules(r#"{ "findings": [ { "level": "error" } ] }"#, "").is_err());
    }
}
