//! Every rule and config error the design names is in the registry or in the list of the ones
//! not built yet, and nothing listed is in the registry.

use std::collections::BTreeSet;

use typdoc_core::rules::{RULES, UNIMPLEMENTED_RULES};
use typdoc_testkit::check::{Kind, acknowledged};
use typdoc_testkit::design::rule_ids;
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
