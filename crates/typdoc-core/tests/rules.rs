//! Every rule and config error the design names is in the registry or in the list of the ones
//! not built yet, and nothing listed is in the registry.

use std::collections::BTreeSet;

use typdoc_core::rules::{ALWAYS_ON, CONFIGURABLE, RULES, UNIMPLEMENTED_RULES};
use typdoc_testkit::check::{Kind, acknowledged};
use typdoc_testkit::design::{always_on_rule_ids, configurable_rule_ids, rule_ids};
use typdoc_testkit::fixtures::design_text;

fn ids(items: &[&str]) -> BTreeSet<String> {
    items.iter().map(|item| item.to_string()).collect()
}

#[test]
fn every_rule_the_design_names_is_built_or_listed_and_nothing_listed_is_built() {
    let result = acknowledged(
        &Kind {
            thing: "rule",
            present: "the registry",
            list: "unimplemented_rules",
        },
        &rule_ids(&design_text()).unwrap(),
        &ids(RULES),
        &ids(UNIMPLEMENTED_RULES),
    );

    assert_eq!(result, Ok(()));
}

#[test]
fn the_rules_a_config_may_name_are_the_two_tables_of_the_design() {
    let design = design_text();
    let configurable: Vec<&str> = CONFIGURABLE.iter().map(|(rule, _)| *rule).collect();

    assert_eq!(ids(ALWAYS_ON), always_on_rule_ids(&design).unwrap());
    assert_eq!(ids(&configurable), configurable_rule_ids(&design).unwrap());
}
