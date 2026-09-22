//! The design, the code and the fixtures agree: everything the design names is built or listed
//! as a difference, nothing listed is built, and each broken fixture trips what it declares.

#[allow(dead_code, reason = "each test file uses part of the shared helper")]
mod common;

use std::collections::BTreeSet;
use std::path::Path;

use common::{NOTES, Ran, Scratch, Spawn, fixture};
use typdoc::registry;
use typdoc_core::rules::{RULES, UNIMPLEMENTED_RULES};
use typdoc_testkit::check::{Kind, acknowledged, broken_coverage, exact_set, tripped_rules};
use typdoc_testkit::design::{command_names, exit_codes};
use typdoc_testkit::fixtures::{broken_entries, design_text};
use typdoc_testkit::golden;

fn set<T: Ord + Clone>(items: &[T]) -> BTreeSet<T> {
    items.iter().cloned().collect()
}

fn names(items: &[&str]) -> BTreeSet<String> {
    items.iter().map(|item| item.to_string()).collect()
}

#[test]
fn every_command_the_design_names_is_built_or_listed_and_nothing_listed_is_built() {
    let built: BTreeSet<String> = registry::commands().into_iter().collect();

    let result = acknowledged(
        &Kind {
            thing: "command",
            present: "the registry",
            list: "unimplemented_commands",
        },
        &command_names(&design_text()).unwrap(),
        &built,
        &names(registry::UNIMPLEMENTED_COMMANDS),
    );

    assert_eq!(result, Ok(()));
}

/// Every command the registry has is also a folder under `fixtures/output/`, and every folder
/// there names a command of the registry: a golden for a command that has left the registry, or
/// a command with no golden at all, is caught here rather than by noticing the gap by eye.
#[test]
fn every_command_of_the_registry_has_a_golden_and_every_golden_names_a_command_of_the_registry() {
    let built: BTreeSet<String> = registry::commands().into_iter().collect();
    let golden: BTreeSet<String> = golden::discover_fixtures()
        .unwrap()
        .iter()
        .map(|case| {
            case.id
                .split_once('/')
                .unwrap_or_else(|| panic!("golden id `{}` has no `/`", case.id))
                .0
                .to_owned()
        })
        .collect();

    let mut problems = Vec::new();
    for command in &built {
        if !golden.contains(command) {
            problems.push(format!(
                "the command {command} is in the registry and has no golden under fixtures/output/"
            ));
        }
    }
    for command in &golden {
        if !built.contains(command) {
            problems.push(format!(
                "fixtures/output/{command}/ holds a golden for a command that is not in the registry"
            ));
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn every_broken_fixture_names_a_rule_that_exists_and_every_rule_that_exists_has_one() {
    let result = broken_coverage(
        &broken_entries(),
        &names(RULES),
        &names(UNIMPLEMENTED_RULES),
    );

    assert_eq!(result, Ok(()));
}

/// Runs the fixture for `rule` in `dir` as its spec says, and compares the rules it trips. A
/// write fixture runs on a copy of `dir`, never on `dir` itself: `common::spawn_fixture` is
/// what decides that, so this function does not have to.
fn check_fixture(dir: &Path, rule: &str) -> Result<(), String> {
    let (spec, ran) = common::spawn_fixture(dir, rule)?;
    let tripped = tripped_rules(&ran.stdout, &ran.stderr)?;
    exact_set(rule, &spec.trips, &tripped)
}

#[test]
fn every_broken_fixture_trips_exactly_the_rules_it_declares() {
    let mut problems = Vec::new();
    for rule in broken_entries() {
        let dir = fixture("broken").join(&rule);
        if let Err(problem) = check_fixture(&dir, &rule) {
            problems.push(problem);
        }
    }

    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn a_fixture_whose_run_trips_nothing_is_red_through_the_built_binary() {
    let project = Scratch::project(&NOTES);
    project.file("note.md", "---\ntitle: Fine\n---\n");
    project.file(
        "fixture.json",
        r#"{ "command": ["get", "note.md", "--json"], "trips": ["frontmatter.types"] }"#,
    );

    let error = check_fixture(project.path(), "frontmatter.types").unwrap_err();

    assert_eq!(
        error,
        "fixture frontmatter.types did not trip frontmatter.types"
    );
}

/// Each exit code a test makes the binary end with, shown by a run that ends with it and, for
/// a failure, prints the error object with that code.
fn produced_exit_codes() -> BTreeSet<u8> {
    let unclosed = Scratch::project(&NOTES);
    unclosed.file("bad.md", "---\ntitle: never closed\n");
    let unreadable = Scratch::project(&NOTES);
    unreadable.file(".typdoc/collections/folder.json/inside", "");
    // A scratch project, never the repository's own `fixtures/valid/minimal`: `new` is a write
    // (`typdoc_testkit::spec::WRITE_COMMANDS`), and even a run refused at exit 7 acquires and
    // releases the namespace's lock first, which is a write to `.typdoc/locks/` that the
    // repository's own tree must never see (contract item 3). The same is true of `mv`, which
    // also produces exit 7 for a destination that already exists (decision 15); either command
    // demonstrates the code, and `new`'s single-file setup is the smaller of the two.
    let exists = Scratch::project(&NOTES);
    exists.file("a.md", "---\ntitle: Already here\n---\n");
    // A lock file nobody owns, made by hand rather than by a real contending process: exit 4
    // only needs a run that meets one and gives up before the timeout, not two processes racing
    // (ticket 4's own "done when" (c); the window this cannot close, the instant inside
    // `acquire`'s own creating call, is left untested on purpose, as decision 6 records).
    let locked = Scratch::project(&NOTES);
    locked.file("a.md", "---\ntitle: Locked out\n---\n");
    locked.file(
        ".typdoc/locks/default.lock",
        r#"{"pid":999999999,"host":"nobody-here","timestamp":"1990-01-01T00:00:00+00:00"}"#,
    );

    let runs: [(u8, Ran); 7] = [
        (
            0,
            Spawn::args(["get", "note.md", "--json"])
                .cwd(fixture("valid/minimal"))
                .run(),
        ),
        (
            1,
            Spawn::args(["get", "note", "--json"])
                .cwd(fixture("valid/minimal"))
                .run(),
        ),
        (
            2,
            Spawn::args(["get", "bad.md", "--json"])
                .cwd(unclosed.path())
                .run(),
        ),
        (
            4,
            Spawn::args(["set", "a.md", "title=x", "--lock-timeout", "1", "--json"])
                .cwd(locked.path())
                .run(),
        ),
        (
            5,
            Spawn::args(["get", "absent.md", "--json"])
                .cwd(fixture("valid/minimal"))
                .run(),
        ),
        (
            6,
            Spawn::args(["get", "a.md", "--json"])
                .cwd(unreadable.path())
                .run(),
        ),
        (
            7,
            Spawn::args(["new", "a.md", "--json"])
                .cwd(exists.path())
                .run(),
        ),
    ];

    let mut produced = BTreeSet::new();
    for (code, ran) in runs {
        assert_eq!(ran.code, i32::from(code), "stderr: {}", ran.stderr);
        if code != 0 {
            assert_eq!(ran.stderr_json()["code"], serde_json::json!(code));
        }
        produced.insert(code);
    }
    produced
}

#[test]
fn every_exit_code_of_the_design_is_produced_or_listed_and_nothing_listed_is_produced() {
    let result = acknowledged(
        &Kind {
            thing: "exit code",
            present: "the codes a test produces",
            list: "unproduced_exit_codes",
        },
        &exit_codes(&design_text()).unwrap(),
        &produced_exit_codes(),
        &set(registry::UNPRODUCED_EXIT_CODES),
    );

    assert_eq!(result, Ok(()));
}
