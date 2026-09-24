//! Every validation rule the catalog (`docs/design/catalog/rules.md`) names is in the registry
//! or in the list of the ones not built yet, and nothing listed is in the registry. The catalog
//! holds only the two rule tables (contract decision 1); the `config.*` error ids from the
//! third table are their own concept (ticket 26) and are checked separately below, against
//! `docs/design/catalog/config-errors.md`.

use std::collections::BTreeSet;

use serde::Deserialize;
use typdoc_core::rules::{ALWAYS_ON, CONFIGURABLE, RULES, UNIMPLEMENTED_RULES};
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

/// One entry of `docs/design/catalog/config-errors.md`'s single list -- `id` mirrors
/// `rules.md`'s own field of the same name; `reported_when` is the short text naming when that
/// config error fires (`design.md`'s old "Reported when" column).
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

/// `rules.md`'s own scope (contract decision 1): one entry per rule id from `ALWAYS_ON` and
/// `CONFIGURABLE` -- the two validation-rule tables -- not `RULES`, which also carries the
/// `config.*` error ids from a third table, held instead by `docs/design/catalog/config-errors.md`
/// and checked separately below (ticket 26).
fn validation_rule_ids() -> BTreeSet<String> {
    let mut all = ids(ALWAYS_ON);
    all.extend(CONFIGURABLE.iter().map(|(rule, _)| rule.to_string()));
    all
}

/// `RULES`'s `config.*` subset (ticket 26): the ids that are neither `ALWAYS_ON` nor
/// `CONFIGURABLE` -- `config-errors.md`'s own scope, the third table `rules.md` does not hold.
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

/// The `config.*` third table (ticket 26): every id `config-errors.md` names is in `RULES`'s
/// `config.*` subset or in the list of the ones not built yet, and nothing listed is in that
/// subset -- the same both-directions property `rules.md`'s own test above holds for the other
/// 23 ids, restoring the goal's "coverage continues unchanged" criterion for the full 43.
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
