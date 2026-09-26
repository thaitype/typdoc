//! Covers SPC-2, SPC-3.

#[allow(dead_code, reason = "each test file uses part of the shared helper")]
mod common;

use std::collections::BTreeSet;
use std::path::Path;

use common::{NOTES, Ran, Scratch, Spawn, fixture};
use typdoc::registry;
use typdoc_core::rules::{RULES, UNIMPLEMENTED_RULES};
use typdoc_testkit::check::{Kind, acknowledged, broken_coverage, exact_set, tripped_rules};
use typdoc_testkit::fixtures::broken_entries;
use typdoc_testkit::golden;

fn set<T: Ord + Clone>(items: &[T]) -> BTreeSet<T> {
    items.iter().cloned().collect()
}

fn names(items: &[&str]) -> BTreeSet<String> {
    items.iter().map(|item| item.to_string()).collect()
}

/// Read as `serde_json::Value`s: `typdoc` has no `serde` dependency of its own to derive a typed
/// struct with.
fn catalog_array(relative: &str, field: &str) -> Vec<serde_json::Value> {
    let body: serde_json::Value = typdoc_testkit::fixtures::read_catalog(relative);
    body[field]
        .as_array()
        .unwrap_or_else(|| panic!("{relative}: `{field}` is not an array"))
        .clone()
}

fn design_commands() -> BTreeSet<String> {
    catalog_array("docs/design/catalog/commands.md", "commands")
        .into_iter()
        .map(|value| {
            value
                .as_str()
                .unwrap_or_else(|| panic!("a command entry is not a string: {value}"))
                .to_owned()
        })
        .collect()
}

fn design_exit_codes() -> BTreeSet<u8> {
    catalog_array("docs/design/catalog/exit-codes.md", "codes")
        .into_iter()
        .map(|value| {
            let code = value
                .as_u64()
                .unwrap_or_else(|| panic!("an exit code entry is not a number: {value}"));
            u8::try_from(code).unwrap_or_else(|_| panic!("exit code {code} does not fit in u8"))
        })
        .collect()
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
        &design_commands(),
        &built,
        &names(registry::UNIMPLEMENTED_COMMANDS),
    );

    assert_eq!(result, Ok(()));
}

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

/// `common::spawn_fixture` runs a write fixture on a copy, so nothing here writes to `dir`.
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

fn produced_exit_codes() -> BTreeSet<u8> {
    let unclosed = Scratch::project(&NOTES);
    unclosed.file("bad.md", "---\ntitle: never closed\n");
    let unreadable = Scratch::project(&NOTES);
    unreadable.file(".typdoc/collections/folder.json/inside", "");
    // A scratch project, not `fixtures/valid/minimal`: even a `new` refused at exit 7 takes and
    // releases the namespace's lock first, a write to `.typdoc/locks/` that the repository's own
    // tree must never see.
    let exists = Scratch::project(&NOTES);
    exists.file("a.md", "---\ntitle: Already here\n---\n");
    // A lock file nobody owns, made by hand: exit 4 needs only a run that meets one and gives
    // up, not two processes racing.
    let locked = Scratch::project(&NOTES);
    locked.file("a.md", "---\ntitle: Locked out\n---\n");
    locked.file(
        ".typdoc/locks/default.lock",
        r#"{"pid":999999999,"host":"nobody-here","timestamp":"1990-01-01T00:00:00+00:00"}"#,
    );
    // `--if` needs a schema that declares the field, and `NOTES`'s declares none.
    let if_false = Scratch::project(&[
        (
            ".typdoc/collections/notes.json",
            r#"{ "match": "*.md", "schema": "note.json" }"#,
        ),
        (
            "note.json",
            r#"{ "name": "note", "fields": { "title": { "type": "string", "required": true } } }"#,
        ),
    ]);
    if_false.file("a.md", "---\ntitle: Before\n---\n");

    let runs: [(u8, Ran); 8] = [
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
            3,
            Spawn::args([
                "set",
                "a.md",
                "title=After",
                "--if",
                "title=Nonexistent",
                "--json",
            ])
            .cwd(if_false.path())
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
        &design_exit_codes(),
        &produced_exit_codes(),
        &set(registry::UNPRODUCED_EXIT_CODES),
    );

    assert_eq!(result, Ok(()));
}
