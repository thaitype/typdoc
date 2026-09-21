//! `examples/`: the small, real project the top of the repository ships for a reader to copy.
//! Not a fixture — nothing here is generated, and nothing in `typdoc_testkit::fixtures` reaches
//! it, so it is read the way a user's own copy would be: `typdoc_testkit::fixtures::root()`
//! plus `examples`, run through the same spawn helper as every other command test.
//!
//! Values are written by hand from the files under `examples/`, never copied from a run.

#[allow(dead_code, reason = "each test file uses part of the shared helper")]
mod common;

use common::{Ran, Spawn};
use serde_json::json;
use typdoc_testkit::fixtures;

fn examples_dir() -> std::path::PathBuf {
    let dir = fixtures::root().join("examples");
    assert!(
        dir.join(".typdoc/config.json").is_file(),
        "{}",
        dir.display()
    );
    dir
}

fn run(args: &[&str]) -> Ran {
    Spawn::args(args.iter().copied()).cwd(examples_dir()).run()
}

#[test]
fn validate_reports_every_ticket_checked_and_finds_nothing_wrong() {
    let ran = run(&["validate", "--json"]);

    assert_eq!(ran.code, 0, "stderr: {}", ran.stderr);
    let report = ran.stdout_json();
    assert_eq!(
        report["summary"]["checked"],
        json!({ "namespaces": ["default"], "documents": 3 })
    );
    assert_eq!(
        report["summary"]["findings"],
        json!({ "error": 0, "warn": 0, "info": 0 })
    );
    assert_eq!(report["findings"], json!([]));
}

#[test]
fn get_reads_a_ticket_by_its_key() {
    let ran = run(&["get", "WF-1", "--json"]);

    assert_eq!(ran.code, 0, "stderr: {}", ran.stderr);
    let document = ran.stdout_json()["document"].clone();
    assert_eq!(document["path"], json!("tickets/WF-1.md"));
    assert_eq!(
        document["fields"]["title"],
        json!("Set up the example project")
    );
    assert_eq!(document["fields"]["status"], json!("resolved"));
}

#[test]
fn list_finds_all_three_tickets() {
    let ran = run(&["list", "--json"]);

    assert_eq!(ran.code, 0, "stderr: {}", ran.stderr);
    let out = ran.stdout_json();
    assert_eq!(out["total"], json!(3));
    assert_eq!(out["truncated"], json!(false));
    let keys: Vec<String> = out["documents"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["key"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(keys, ["WF-1", "WF-2", "WF-3"]);
}

#[test]
fn toc_reads_the_headings_of_a_ticket() {
    let ran = run(&["toc", "WF-1", "--json"]);

    assert_eq!(ran.code, 0, "stderr: {}", ran.stderr);
    let headings = ran.stdout_json()["headings"].clone();
    let texts: Vec<String> = headings
        .as_array()
        .unwrap()
        .iter()
        .map(|h| h["text"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(texts, ["Set up the example project", "Question", "Answer"]);
}

#[test]
fn refs_follows_blocked_by_from_wf_2_to_wf_1() {
    let ran = run(&["refs", "WF-2", "--json"]);

    assert_eq!(ran.code, 0, "stderr: {}", ran.stderr);
    let refs = ran.stdout_json()["refs"].clone();
    let refs = refs.as_array().unwrap();
    assert_eq!(refs.len(), 1, "{refs:?}");
    assert_eq!(refs[0]["path"], json!("tickets/WF-1.md"));
    assert_eq!(refs[0]["field"], json!("blocked_by"));
}
