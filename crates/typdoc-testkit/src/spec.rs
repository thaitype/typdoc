//! What a broken fixture declares about itself, in `fixture.json` at the folder's root: the
//! command to run in it, the exact set of rules it is expected to trip, and any environment
//! variable the run needs declared (`env`, empty by default: most fixtures need none, since the
//! spawn helper already gives every run a fresh `HOME`; a rule that can only be tripped by
//! setting a variable on purpose, such as `config.config-dir`'s `TYPDOC_CONFIG_DIR`, is the
//! exception `env` exists for). The set of rules is written by hand from the design and never
//! taken from a run of the tool.
//!
//! Also whether the declared command is one that writes (`is_write`); where such a fixture then
//! runs is `crate::staging`.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde::Deserialize;

#[derive(Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FixtureSpec {
    /// The arguments of the run, after the program name.
    pub command: Vec<String>,
    /// The rules the run is expected to trip. The rule the folder is named for is one of them.
    pub trips: BTreeSet<String>,
    /// A variable the spawned run needs set, by name; empty when the fixture needs none (the
    /// ordinary case). `HOME` and `PATH` are the spawn helper's own and cannot be named here
    /// (the same rule `Spawn::var` already enforces for a test that builds one by hand).
    #[serde(default)]
    pub env: BTreeMap<String, String>,
}

impl FixtureSpec {
    /// Reads the spec of the fixture for `rule`, which is the folder's name.
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

    /// Whether the declared command changes a project's files. The first word of `command`
    /// names the command; a write is one of the three story 2 builds. None of the three is a
    /// command the binary has yet, so this reads only the declaration, never the binary.
    pub fn is_write(&self) -> bool {
        self.command
            .first()
            .is_some_and(|first| WRITE_COMMANDS.contains(&first.as_str()))
    }
}

/// The commands whose run changes a project's files. Every other command reads. Kept here
/// rather than derived from the registry, since a write command does not exist in the binary
/// yet for this to read from.
const WRITE_COMMANDS: &[&str] = &["new", "set", "mv"];

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
