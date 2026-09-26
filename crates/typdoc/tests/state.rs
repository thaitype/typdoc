//! Covers SPC-8.
//!
//! The cases beyond the one trip each rule's `fixtures/broken/` folder gives through the
//! coverage harness. `config.state-orphan` and `config.state-uncoded` are config errors, covered
//! by their own `fixtures/broken/` folders and by `config.rs`.

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

/// The next test adds one document to the same project and turns it red, so this silence is the
/// rule's answer and not an absence of checking.
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

fn one_finding(ran: &Ran, rule: &str) -> serde_json::Value {
    assert_eq!(ran.code, 2, "{}", ran.stderr);
    let findings = ran.stdout_json()["findings"].as_array().unwrap().clone();
    assert_eq!(findings.len(), 1, "{findings:?}");
    assert_eq!(findings[0]["rule"], json!(rule), "{findings:?}");
    findings[0].clone()
}

fn warn_only(ran: &Ran, rule: &str) -> serde_json::Value {
    assert_eq!(ran.code, 0, "{}", ran.stderr);
    let findings = ran.stdout_json()["findings"].as_array().unwrap().clone();
    assert_eq!(findings.len(), 1, "{findings:?}");
    assert_eq!(findings[0]["rule"], json!(rule), "{findings:?}");
    assert_eq!(findings[0]["level"], json!("warn"), "{findings:?}");
    findings[0].clone()
}

// --- `state.malformed` ---

#[test]
fn a_last_that_is_text_is_state_malformed_not_state_missing() {
    let files = [
        (
            ".typdoc/collections/tickets.json",
            r#"{ "match": "tickets/{key}.md", "schema": "wf.json" }"#,
        ),
        ("wf.json", r#"{ "name": "wf", "code": "WF", "fields": {} }"#),
        (
            ".typdoc/state/default.json",
            r#"{ "tickets": { "last": "three" } }"#,
        ),
    ];
    let project = Scratch::project(&files);
    project.file("tickets/WF-1.md", "");

    let finding = one_finding(&validate(project.path()), "state.malformed");

    assert_eq!(finding["collection"], json!("tickets"));
    assert_eq!(finding["namespace"], json!("default"));
    assert_eq!(finding["level"], json!("error"));
}

/// Unlike `state.missing`, the record is wrong on its own account.
#[test]
fn a_malformed_last_is_reported_with_no_document_of_the_collection_at_all() {
    let files = [
        (
            ".typdoc/collections/tickets.json",
            r#"{ "match": "tickets/{key}.md", "schema": "wf.json" }"#,
        ),
        ("wf.json", r#"{ "name": "wf", "code": "WF", "fields": {} }"#),
        (
            ".typdoc/state/default.json",
            r#"{ "tickets": { "last": null } }"#,
        ),
    ];
    let project = Scratch::project(&files);

    let finding = one_finding(&validate(project.path()), "state.malformed");

    assert_eq!(finding["collection"], json!("tickets"));
}

#[test]
fn a_valid_last_leaves_state_malformed_silent() {
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

// --- `state.behind` ---

#[test]
fn a_last_lower_than_the_highest_existing_number_is_state_behind() {
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
    project.file("tickets/WF-2.md", "");
    project.file("tickets/WF-3.md", "");

    let finding = warn_only(&validate(project.path()), "state.behind");

    assert_eq!(finding["collection"], json!("tickets"));
    assert_eq!(finding["namespace"], json!("default"));
    assert!(
        finding["message"].as_str().unwrap().contains('1'),
        "{finding}"
    );
    assert!(
        finding["message"].as_str().unwrap().contains('3'),
        "{finding}"
    );
}

#[test]
fn a_last_equal_to_the_highest_existing_number_leaves_state_behind_silent() {
    let files = [
        (
            ".typdoc/collections/tickets.json",
            r#"{ "match": "tickets/{key}.md", "schema": "wf.json" }"#,
        ),
        ("wf.json", r#"{ "name": "wf", "code": "WF", "fields": {} }"#),
        (
            ".typdoc/state/default.json",
            r#"{ "tickets": { "last": 3 } }"#,
        ),
    ];
    let project = Scratch::project(&files);
    project.file("tickets/WF-1.md", "");
    project.file("tickets/WF-2.md", "");
    project.file("tickets/WF-3.md", "");

    let ran = validate(project.path());

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    assert_eq!(ran.stdout_json()["findings"], json!([]));
}

/// A gap left by a deletion is kept on purpose, so it is not a fault.
#[test]
fn a_last_higher_than_every_existing_number_leaves_state_behind_silent() {
    let files = [
        (
            ".typdoc/collections/tickets.json",
            r#"{ "match": "tickets/{key}.md", "schema": "wf.json" }"#,
        ),
        ("wf.json", r#"{ "name": "wf", "code": "WF", "fields": {} }"#),
        (
            ".typdoc/state/default.json",
            r#"{ "tickets": { "last": 3 } }"#,
        ),
    ];
    let project = Scratch::project(&files);
    project.file("tickets/WF-1.md", "");
    project.file("tickets/WF-2.md", "");

    let ran = validate(project.path());

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    assert_eq!(ran.stdout_json()["findings"], json!([]));
}

// --- `state.retired` ---

#[test]
fn an_entry_for_a_collection_the_project_no_longer_has_is_state_retired() {
    let files = [
        (
            ".typdoc/collections/notes.json",
            r#"{ "match": "*.md", "schema": "note.json" }"#,
        ),
        ("note.json", r#"{ "name": "note", "fields": {} }"#),
        (
            ".typdoc/state/default.json",
            r#"{ "tickets": { "last": 5 } }"#,
        ),
    ];
    let project = Scratch::project(&files);
    project.file("note.md", "");

    let finding = warn_only(&validate(project.path()), "state.retired");

    assert_eq!(finding["collection"], json!("tickets"));
}

#[test]
fn no_stray_entry_leaves_state_retired_silent() {
    let files = [
        (
            ".typdoc/collections/notes.json",
            r#"{ "match": "*.md", "schema": "note.json" }"#,
        ),
        ("note.json", r#"{ "name": "note", "fields": {} }"#),
    ];
    let project = Scratch::project(&files);
    project.file("note.md", "");

    let ran = validate(project.path());

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    assert_eq!(ran.stdout_json()["findings"], json!([]));
}

/// The two projects differ only in whether the entry names a collection the project has, with
/// no code (`config.state-uncoded`, a config error that stops every command, a read included), or
/// none at all (`state.retired`, a finding that stops nothing).
#[test]
fn state_retired_does_not_stop_a_read_where_its_predecessor_as_a_config_error_did() {
    let notes_collection = (
        ".typdoc/collections/notes.json",
        r#"{ "match": "*.md", "schema": "note.json" }"#,
    );
    let note_schema = ("note.json", r#"{ "name": "note", "fields": {} }"#);

    let retired = Scratch::project(&[
        notes_collection,
        note_schema,
        (
            ".typdoc/state/default.json",
            r#"{ "tickets": { "last": 5 } }"#,
        ),
    ]);
    retired.file("note.md", "");

    let uncoded = Scratch::project(&[
        notes_collection,
        note_schema,
        (
            ".typdoc/state/default.json",
            r#"{ "notes": { "last": 5 } }"#,
        ),
    ]);
    uncoded.file("note.md", "");

    let get =
        |project: &std::path::Path| Spawn::args(["get", "note.md", "--json"]).cwd(project).run();

    let retired_read = get(retired.path());
    let uncoded_read = get(uncoded.path());

    assert_eq!(retired_read.code, 0, "{}", retired_read.stderr);
    assert_eq!(uncoded_read.code, 2, "{}", uncoded_read.stderr);
    assert_eq!(
        uncoded_read.stderr_json()["details"][0]["rule"],
        json!("config.state-uncoded"),
        "{}",
        uncoded_read.stderr
    );
}

// --- Why these rules report and never repair ---

/// `WF-3` is deleted from a project whose `WF-1` links to it. Keeping the gap (`last` stays `3`)
/// leaves a `body.links` finding that says where to look. Deriving `last` from the files would set
/// it to `2`, and the next `new` would issue `WF-3` again to a different document, after which
/// `validate` reports nothing. `new.rs`'s `new_never_reissues_a_number_whose_document_was_deleted`
/// runs `new` itself.
#[test]
fn the_measurement_behind_decision_13_deriving_last_reissues_a_retired_key_silently() {
    let schema = r#"{ "name": "wf", "code": "WF", "fields": { "title": { "type": "string" } } }"#;
    let files = [
        (
            ".typdoc/collections/tickets.json",
            r#"{ "match": "tickets/{key}.md", "schema": "wf.json" }"#,
        ),
        ("wf.json", schema),
        (
            ".typdoc/state/default.json",
            r#"{ "tickets": { "last": 3 } }"#,
        ),
    ];

    // Keeping the gap: `WF-3` deleted, `last` left at `3`.
    let keeping_the_gap = Scratch::project(&files);
    keeping_the_gap.file(
        "tickets/WF-1.md",
        "---\ntitle: One\n---\n\n[the third](WF-3.md)\n",
    );
    keeping_the_gap.file("tickets/WF-2.md", "");
    // `WF-3.md` is never created here: this is the project after its deletion.

    let ran = validate(keeping_the_gap.path());

    assert_eq!(ran.code, 2, "{}", ran.stderr);
    let findings = ran.stdout_json()["findings"].as_array().unwrap().clone();
    let target_missing: Vec<&serde_json::Value> = findings
        .iter()
        .filter(|f| f["rule"] == json!("body.links"))
        .collect();
    assert_eq!(target_missing.len(), 1, "{findings:?}");
    assert!(
        target_missing[0]["message"]
            .as_str()
            .unwrap()
            .contains("WF-3.md"),
        "{:?}",
        target_missing[0]
    );

    // Deriving `last` from the files: the next `new` issues `WF-3` again to another document and
    // records `last: 3`. Both effects are put in place by hand, as `new` would leave them.
    let derived_from_files = Scratch::project(&[
        (
            ".typdoc/collections/tickets.json",
            r#"{ "match": "tickets/{key}.md", "schema": "wf.json" }"#,
        ),
        ("wf.json", schema),
        (
            ".typdoc/state/default.json",
            r#"{ "tickets": { "last": 3 } }"#,
        ),
    ]);
    derived_from_files.file(
        "tickets/WF-1.md",
        "---\ntitle: One\n---\n\n[the third](WF-3.md)\n",
    );
    derived_from_files.file("tickets/WF-2.md", "");
    derived_from_files.file(
        "tickets/WF-3.md",
        "---\ntitle: An unrelated document\n---\n",
    );

    let ran = validate(derived_from_files.path());

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    assert_eq!(
        ran.stdout_json()["findings"],
        json!([]),
        "the reissued key is a well-formed link to a well-formed document, so nothing reports \
         that it is the wrong one"
    );
}
