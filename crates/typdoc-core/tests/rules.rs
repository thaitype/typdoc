//! Covers SPC-1, SPC-6.
//!
//! Every id the catalogs name is built or listed as not built, and nothing listed is built. The
//! rules catalog holds the validation rules only; the `config.*` ids are checked against their
//! own catalog.

use std::collections::BTreeSet;

use serde::Deserialize;
use typdoc_core::rules::{ALWAYS_ON, CONFIGURABLE, RULES, UNIMPLEMENTED_RULES};
use typdoc_testkit::check::{Kind, acknowledged};

#[derive(Deserialize)]
struct RuleEntry {
    id: String,
    configurable: bool,
}

#[derive(Deserialize)]
struct RulesCatalog {
    rules: Vec<RuleEntry>,
}

#[derive(Deserialize)]
struct ConfigErrorEntry {
    id: String,
    #[expect(dead_code, reason = "read for completeness; only `id` is checked here")]
    reported_when: String,
}

#[derive(Deserialize)]
struct ConfigErrorsCatalog {
    errors: Vec<ConfigErrorEntry>,
}

fn ids(items: &[&str]) -> BTreeSet<String> {
    items.iter().map(|item| item.to_string()).collect()
}

fn rules_catalog() -> RulesCatalog {
    typdoc_testkit::fixtures::read_catalog("docs/design/catalog/rules.md")
}

fn config_errors_catalog() -> ConfigErrorsCatalog {
    typdoc_testkit::fixtures::read_catalog("docs/design/catalog/config-errors.md")
}

/// The ids of the rules catalog: not `RULES`, which also holds the `config.*` ids.
fn validation_rule_ids() -> BTreeSet<String> {
    let mut all = ids(ALWAYS_ON);
    all.extend(CONFIGURABLE.iter().map(|(rule, _)| rule.to_string()));
    all
}

fn config_error_ids() -> BTreeSet<String> {
    let validation = validation_rule_ids();
    RULES
        .iter()
        .map(|rule| rule.to_string())
        .filter(|rule| !validation.contains(rule))
        .collect()
}

#[test]
fn every_rule_the_design_names_is_built_or_listed_and_nothing_listed_is_built() {
    let catalog = rules_catalog();
    let named: BTreeSet<String> = catalog.rules.iter().map(|entry| entry.id.clone()).collect();

    let result = acknowledged(
        &Kind {
            thing: "rule",
            present: "the registry",
            list: "unimplemented_rules",
        },
        &named,
        &validation_rule_ids(),
        &ids(UNIMPLEMENTED_RULES),
    );

    assert_eq!(result, Ok(()));
}

#[test]
fn the_rules_a_config_may_name_are_split_by_the_catalogs_configurable_field() {
    let catalog = rules_catalog();
    let always_on: BTreeSet<String> = catalog
        .rules
        .iter()
        .filter(|entry| !entry.configurable)
        .map(|entry| entry.id.clone())
        .collect();
    let configurable_ids: BTreeSet<String> = catalog
        .rules
        .iter()
        .filter(|entry| entry.configurable)
        .map(|entry| entry.id.clone())
        .collect();
    let configurable: Vec<&str> = CONFIGURABLE.iter().map(|(rule, _)| *rule).collect();

    assert_eq!(ids(ALWAYS_ON), always_on);
    assert_eq!(ids(&configurable), configurable_ids);
}

#[test]
fn every_config_error_the_design_names_is_built_or_listed_and_nothing_listed_is_built() {
    let catalog = config_errors_catalog();
    let named: BTreeSet<String> = catalog
        .errors
        .iter()
        .map(|entry| entry.id.clone())
        .collect();

    let result = acknowledged(
        &Kind {
            thing: "config error",
            present: "RULES's config.* subset",
            list: "unimplemented_rules",
        },
        &named,
        &config_error_ids(),
        &ids(UNIMPLEMENTED_RULES),
    );

    assert_eq!(result, Ok(()));
}
