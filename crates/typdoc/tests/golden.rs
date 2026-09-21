//! The `--json` output of each command against its golden and its assertions, and the
//! generator that writes a golden.

#[allow(dead_code, reason = "each test file uses part of the shared helper")]
mod common;

use common::{Ran, Spawn, fixture};
use serde_json::{Value, json};
use typdoc_testkit::fixtures;
use typdoc_testkit::golden::{self, Case, REGENERATE_VAR};

fn run(case: &Case) -> Ran {
    Spawn::args(&case.command).cwd(fixture(&case.project)).run()
}

/// The output of the case's run as JSON, when the run ended with 0 and printed nothing on
/// standard error.
fn output_of(case: &Case) -> Result<Value, String> {
    let ran = run(case);
    if ran.code != 0 || !ran.stderr.is_empty() {
        return Err(format!(
            "{}: the run ended with {} and printed {:?} on standard error",
            case.id, ran.code, ran.stderr
        ));
    }
    serde_json::from_str(&ran.stdout).map_err(|e| format!("{}: stdout is not JSON: {e}", case.id))
}

fn case_named(id: &str) -> Case {
    golden::discover_fixtures()
        .unwrap()
        .into_iter()
        .find(|case| case.id == id)
        .unwrap_or_else(|| panic!("the golden case {id} is missing"))
}

/// The output of the case's run, and its golden as JSON that a test may change.
fn output_and_golden(id: &str) -> (Value, Value) {
    let case = case_named(id);
    let golden = std::fs::read_to_string(case.golden_path()).unwrap();
    (
        output_of(&case).unwrap(),
        serde_json::from_str(&golden).unwrap(),
    )
}

#[test]
fn every_golden_case_matches_its_golden_and_its_assertions() {
    let cases = golden::discover_fixtures().unwrap();
    assert!(!cases.is_empty(), "fixtures/output/ holds no golden case");

    let mut problems = Vec::new();
    for case in &cases {
        match output_of(case) {
            Ok(actual) => {
                if let Err(problem) = case.check(&actual) {
                    problems.push(problem);
                }
            }
            Err(problem) => problems.push(problem),
        }
    }

    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn the_golden_of_get_turns_red_when_a_value_is_changed_on_purpose() {
    let (actual, mut changed) = output_and_golden("get/minimal");
    assert_eq!(golden::compare(&changed, &actual), Ok(()));

    changed["document"]["fields"]["title"] = json!("Another title");

    let error = golden::compare(&changed, &actual).unwrap_err();
    assert!(error.contains("/document/fields/title"), "{error}");
}

#[test]
fn the_golden_of_get_turns_red_when_an_array_is_in_the_wrong_order() {
    let (actual, mut changed) = output_and_golden("get/minimal");

    changed["document"]["fields"]["tags"] = json!(["beta", "alpha"]);

    let error = golden::compare(&changed, &actual).unwrap_err();
    assert!(error.contains("/document/fields/tags/0"), "{error}");
}

#[test]
fn a_read_command_prints_the_same_bytes_on_every_run() {
    for case in golden::discover_fixtures().unwrap() {
        let first = run(&case).stdout;
        let second = run(&case).stdout;

        assert_eq!(first, second, "{} prints something that changes", case.id);
    }
}

/// Regenerates the one golden that `TYPDOC_REGENERATE_GOLDEN` names, `<command>/<case>`.
/// It is not part of a plain run of the suite, so it shows as ignored, not as passed.
#[test]
#[ignore = "regenerates one golden: TYPDOC_REGENERATE_GOLDEN=<command>/<case> scripts/test.sh -p typdoc --test golden regenerate -- --ignored"]
fn regenerate() {
    let id = std::env::var(REGENERATE_VAR)
        .unwrap_or_else(|_| panic!("{REGENERATE_VAR} names the one golden to regenerate"));

    let written = golden::regenerate(&fixtures::path("output"), &id, output_of)
        .unwrap_or_else(|problem| panic!("{problem}"));

    println!("wrote {}", written.display());
}
