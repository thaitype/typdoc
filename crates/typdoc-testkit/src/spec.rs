//! A broken fixture's `fixture.json`. Its `trips` are written by hand from the design and never
//! taken from a run of the tool.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde::Deserialize;

#[derive(Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FixtureSpec {
    /// The arguments of the run, after the program name.
    pub command: Vec<String>,
    pub trips: BTreeSet<String>,
    /// Empty for most fixtures, since the spawn helper already gives every run a fresh `HOME`;
    /// it is for a rule that can only be tripped by setting a variable on purpose, such as
    /// `config.config-dir`'s `TYPDOC_CONFIG_DIR`. `HOME` and `PATH` are the spawn helper's own
    /// and cannot be named here (the same rule `Spawn::var` enforces).
    #[serde(default)]
    pub env: BTreeMap<String, String>,
}

impl FixtureSpec {
    pub fn parse(rule: &str, text: &str) -> Result<FixtureSpec, String> {
        let spec: FixtureSpec = serde_json::from_str(text).map_err(|e| e.to_string())?;
        if spec.command.is_empty() {
            return Err("`command` is empty".into());
        }
        if !spec.trips.contains(rule) {
            return Err(format!(
                "`trips` does not hold {rule}, the rule the folder is named for"
            ));
        }
        Ok(spec)
    }

    pub fn load(dir: &Path, rule: &str) -> Result<FixtureSpec, String> {
        let file = dir.join("fixture.json");
        let text =
            std::fs::read_to_string(&file).map_err(|e| format!("{}: {e}", file.display()))?;
        FixtureSpec::parse(rule, &text).map_err(|e| format!("{}: {e}", file.display()))
    }

    pub fn is_write(&self) -> bool {
        is_write_command(&self.command)
    }
}

/// Every command not listed here reads. Kept here rather than derived from the registry.
const WRITE_COMMANDS: &[&str] = &["new", "set", "mv"];

pub fn is_write_command(command: &[String]) -> bool {
    command
        .first()
        .is_some_and(|first| WRITE_COMMANDS.contains(&first.as_str()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_spec_holds_the_command_and_the_rules_the_run_trips() {
        let spec = FixtureSpec::parse(
            "body.links",
            r#"{ "command": ["validate", "--json"], "trips": ["body.links", "body.anchors"] }"#,
        )
        .unwrap();

        assert_eq!(spec.command, ["validate", "--json"]);
        assert_eq!(
            spec.trips,
            BTreeSet::from(["body.anchors".to_string(), "body.links".to_string()])
        );
    }

    #[test]
    fn a_spec_that_does_not_trip_its_own_rule_is_refused() {
        let error = FixtureSpec::parse(
            "body.links",
            r#"{ "command": ["validate"], "trips": ["body.anchors"] }"#,
        )
        .unwrap_err();

        assert!(error.contains("body.links"), "{error}");
    }

    #[test]
    fn a_spec_with_no_command_or_an_unknown_key_or_broken_json_is_refused() {
        assert!(FixtureSpec::parse("a.b", r#"{ "command": [], "trips": ["a.b"] }"#).is_err());
        assert!(
            FixtureSpec::parse(
                "a.b",
                r#"{ "command": ["x"], "trips": ["a.b"], "setup": "rm" }"#
            )
            .is_err()
        );
        assert!(FixtureSpec::parse("a.b", "{").is_err());
    }

    #[test]
    fn a_fixture_with_no_spec_file_is_refused_with_the_file_named() {
        let dir = tempfile::tempdir().unwrap();

        let error = FixtureSpec::load(dir.path(), "a.b").unwrap_err();

        assert!(error.contains("fixture.json"), "{error}");
    }

    #[test]
    fn a_spec_whose_command_starts_with_new_set_or_mv_is_a_write() {
        for command in ["new", "set", "mv"] {
            let spec = FixtureSpec::parse(
                "a.b",
                &format!(r#"{{ "command": ["{command}", "x"], "trips": ["a.b"] }}"#),
            )
            .unwrap();

            assert!(spec.is_write(), "{command} is a write");
        }
    }

    #[test]
    fn a_spec_whose_command_starts_with_a_read_command_is_not_a_write() {
        for command in ["get", "list", "refs", "toc", "validate"] {
            let spec = FixtureSpec::parse(
                "a.b",
                &format!(r#"{{ "command": ["{command}"], "trips": ["a.b"] }}"#),
            )
            .unwrap();

            assert!(!spec.is_write(), "{command} is not a write");
        }
    }
}
