//! Every validation rule the catalog (`docs/design/catalog/rules.md`) names is in the registry
//! or in the list of the ones not built yet, and nothing listed is in the registry. The catalog
//! holds only the two rule tables (contract decision 1) -- `config.*` error ids are out of its
//! scope and are not checked here.

use std::collections::BTreeSet;

use serde::Deserialize;
use typdoc_core::rules::{ALWAYS_ON, CONFIGURABLE, UNIMPLEMENTED_RULES};
use typdoc_testkit::check::{Kind, acknowledged};

/// One entry of `docs/design/catalog/rules.md`'s single list -- `configurable` lives on the
/// entry itself rather than which of two tables it used to be in.
#[derive(Deserialize)]
struct RuleEntry {
    id: String,
    configurable: bool,
}

#[derive(Deserialize)]
struct RulesCatalog {
    rules: Vec<RuleEntry>,
}

fn ids(items: &[&str]) -> BTreeSet<String> {
    items.iter().map(|item| item.to_string()).collect()
}

fn rules_catalog() -> RulesCatalog {
    typdoc_testkit::fixtures::read_catalog("docs/design/catalog/rules.md")
}

/// `rules.md`'s own scope (contract decision 1): one entry per rule id from `ALWAYS_ON` and
/// `CONFIGURABLE` -- the two validation-rule tables -- not `RULES`, which also carries the
/// `config.*` error ids from a third table the catalog does not hold.
fn validation_rule_ids() -> BTreeSet<String> {
    let mut all = ids(ALWAYS_ON);
    all.extend(CONFIGURABLE.iter().map(|(rule, _)| rule.to_string()));
    all
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
