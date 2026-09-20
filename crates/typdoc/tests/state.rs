//! `state.missing` through the built binary: the three cases the design and the contract name
//! by name (a file absent while documents exist, a file without the entry, and a new collection
//! that reports nothing), beyond the single trip `fixtures/broken/state.missing` shows through
//! the generic coverage harness. `config.state-orphan` and `config.state-uncoded` are config
//! errors: `fixtures/broken/config.state-orphan` and `fixtures/broken/config.state-uncoded`
//! cover them through that same harness, since every config error's shape (exit 2, the error
//! object) is already checked generically in `config.rs`.

#[allow(dead_code, reason = "each test file uses part of the shared helper")]
mod common;

use common::{Ran, Scratch, Spawn};
use serde_json::json;

const TICKETS_SCHEMA: [(&str, &str); 2] = [
    (
        ".typdoc/collections/tickets.json",
        r#"{ "match": "tickets/{key}.md", "schema": "wf.json" }"#,
    ),
    ("wf.json", r#"{ "name": "wf", "code": "WF", "fields": {} }"#),
];

fn validate(project: &std::path::Path) -> Ran {
    Spawn::args(["validate", "--json"]).cwd(project).run()
}

fn state_missing(ran: &Ran) -> serde_json::Value {
    assert_eq!(ran.code, 2, "{}", ran.stderr);
    let findings = ran.stdout_json()["findings"].clone();
    let findings = findings.as_array().unwrap().clone();
    assert_eq!(findings.len(), 1, "{findings:?}");
    assert_eq!(findings[0]["rule"], json!("state.missing"));
    findings[0].clone()
}

/// The state file's own file, missing entirely, while the collection has a document: the
/// simplest of the three cases, and the one `fixtures/broken/state.missing` also shows.
#[test]
fn no_state_file_at_all_while_documents_exist_is_state_missing() {
    let project = Scratch::project(&TICKETS_SCHEMA);
    project.file("tickets/WF-1.md", "");

    let finding = state_missing(&validate(project.path()));

    assert_eq!(finding["path"], json!(".typdoc/state/default.json"));
    assert_eq!(finding["namespace"], json!("default"));
    assert_eq!(finding["collection"], json!("tickets"));
    assert!(finding.get("key").is_none(), "{finding}");
}

/// The state file exists, and holds entries for other collections, but not this one: still
/// `state.missing`, so a state file that is merely incomplete is not read as "recorded".
#[test]
fn a_state_file_that_exists_without_this_collections_entry_is_state_missing() {
    let files = [
        (
            ".typdoc/collections/tickets.json",
            r#"{ "match": "tickets/{key}.md", "schema": "wf.json" }"#,
        ),
        ("wf.json", r#"{ "name": "wf", "code": "WF", "fields": {} }"#),
        (
            ".typdoc/collections/rfcs.json",
            r#"{ "match": "rfcs/{key}.md", "schema": "rfc.json" }"#,
        ),
        (
            "rfc.json",
            r#"{ "name": "rfc", "code": "RFC", "fields": {} }"#,
        ),
        (".typdoc/state/default.json", r#"{ "rfcs": { "last": 4 } }"#),
    ];
    let project = Scratch::project(&files);
    project.file("tickets/WF-1.md", "");

    let finding = state_missing(&validate(project.path()));

    assert_eq!(finding["collection"], json!("tickets"));
}

/// A coded collection with no documents at all in the namespace, and no record of it either, is
/// new: nothing is reported (design, State: "A collection with no coded documents in the
/// namespace and no record is new, and nothing is reported"). Proof this is read, not merely
/// unwritten: a document of the collection is added in the next test and the same project turns
/// red.
#[test]
fn a_coded_collection_with_no_documents_and_no_record_reports_nothing() {
    let project = Scratch::project(&TICKETS_SCHEMA);

    let ran = validate(project.path());

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    assert_eq!(ran.stdout_json()["findings"], json!([]));
}

#[test]
fn the_same_project_with_a_document_added_turns_red() {
    let project = Scratch::project(&TICKETS_SCHEMA);
    project.file("tickets/WF-1.md", "");

    let ran = validate(project.path());

    assert_eq!(ran.code, 2, "{}", ran.stderr);
}

/// A `last` recorded for the collection clears the finding.
#[test]
fn a_recorded_last_makes_the_project_clean() {
    let files = [
        (
            ".typdoc/collections/tickets.json",
            r#"{ "match": "tickets/{key}.md", "schema": "wf.json" }"#,
        ),
        ("wf.json", r#"{ "name": "wf", "code": "WF", "fields": {} }"#),
        (
            ".typdoc/state/default.json",
            r#"{ "tickets": { "last": 1 } }"#,
        ),
    ];
    let project = Scratch::project(&files);
    project.file("tickets/WF-1.md", "");

    let ran = validate(project.path());

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    assert_eq!(ran.stdout_json()["findings"], json!([]));
}
