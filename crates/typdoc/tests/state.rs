//! `state.missing`, `state.malformed`, `state.behind` and `state.retired` through the built
//! binary, beyond the single trip each rule's `fixtures/broken/` folder shows through the
//! generic coverage harness: the cases the design and the contract name by name, and, for
//! `state.behind` and `state.retired`, the measurement decision 13 is built on (why these rules
//! report and never repair). `config.state-orphan` and `config.state-uncoded` are config
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

// --- `state.malformed`: present but unusable, told apart from `state.missing` (decision 13) ---

/// A `last` that is text is `state.malformed`, at `error`, not `state.missing`: the record is
/// there, and reporting it as absent would send a user to restore a file that is not lost.
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

/// `state.malformed` does not need a document of the collection to be present: the record is
/// wrong on its own account, unlike `state.missing`, which only fires once a document exists.
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

/// A valid `last` leaves `state.malformed` silent (the counterpart already shown for
/// `state.missing` by `a_recorded_last_makes_the_project_clean` covers this the same way).
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

// --- `state.behind`: `last` lower than the highest existing number, at `warn` (decision 13) ---

/// A recorded `last` lower than the highest existing number is `state.behind`, at `warn`:
/// allocation still gives the right number (the larger of the two), but the record itself is
/// telling the reader something untrue.
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

/// A `last` equal to the highest existing number leaves `state.behind` silent: it is not lower,
/// so nothing is wrong yet.
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

/// A `last` higher than every existing number (a gap kept on purpose, after a deletion) also
/// leaves `state.behind` silent: keeping the gap is decision 13's whole point, not a fault of
/// its own.
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

// --- `state.retired`: an entry for a collection the project no longer has (decision 13) ---

/// A state entry naming a collection this project no longer has at all is `state.retired`, at
/// `warn`, and is kept: the read still succeeds.
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

/// No stray entry, no finding: `state.retired` is not a rule that fires on everything.
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

/// The contrast decision 13 asks to be shown: an entry for a collection this project no longer
/// has used to be `config.state-uncoded`, a config error, which stops every command including a
/// plain read (exit 2, even for `get` on an unrelated file). Now it is `state.retired`, a
/// `validate` finding, and the same read succeeds: the two projects here differ only in whether
/// the state file's stray entry names a collection the project still has (uncoded, still a
/// config error) or one it does not have at all (retired, no longer one).
#[test]
fn state_retired_does_not_stop_a_read_where_its_predecessor_as_a_config_error_did() {
    let notes_collection = (
        ".typdoc/collections/notes.json",
        r#"{ "match": "*.md", "schema": "note.json" }"#,
    );
    let note_schema = ("note.json", r#"{ "name": "note", "fields": {} }"#);

    // Retired: `tickets` is not a collection of this project at all.
    let retired = Scratch::project(&[
        notes_collection,
        note_schema,
        (
            ".typdoc/state/default.json",
            r#"{ "tickets": { "last": 5 } }"#,
        ),
    ]);
    retired.file("note.md", "");

    // Uncoded: `notes` is a collection of this project, and its schema has no code — the case
    // `config.state-uncoded` still is (`fixtures/broken/config.state-uncoded` covers the trip
    // itself; this asserts the exit code contrast directly, beside `state.retired`'s).
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

// --- The measurement decision 13 is built on: why these rules report and never repair ---

/// Records the comparison decision 13 measures. A project has `WF-1` (linking to `WF-3.md`),
/// `WF-2` and `WF-3`, with `last: 3`. `WF-3` is deleted.
///
/// Keeping the gap (`last` stays `3`, the highest number *ever issued* rather than the highest
/// that still exists) gives a `body.links` finding: the link now names a file that is not
/// there, which is exactly what a reader needs to know.
///
/// Deriving `last` from the files instead would set it to `2`, the highest that *exists* after
/// the deletion — and the next `new` would issue `WF-3` again, since allocation is one past the
/// larger of the highest existing number and `last`. This is simulated here without `new`
/// itself (tickets 9's own command; not built by this ticket) by recreating `WF-3.md` as a
/// *different* document by hand, the way a freshly issued `WF-3` would be: the link is well
/// formed, the document exists, and `validate` reports nothing about it at all — the silent
/// wrong answer decision 13 is written to prevent.
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

    // Deriving `last` from the files: right after the deletion this would set it to `2`, the
    // highest that still exists, and the next `new` allocates one past the larger of that and
    // `last` — `3` again — records it as the new `last` (decision 13's own wording) and issues
    // `WF-3` to a document that is not the one `WF-1` was written about. `new` is not built by
    // this ticket, so both of its effects (the file and the state file) are put in place by
    // hand here, exactly as `new` would leave them: `last` ends at `3`, not `2`, which is the
    // point — the file says nothing is wrong any more, because as far as it knows nothing is.
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
