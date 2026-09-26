//! Covers SPC-2, SPC-5, SPC-8, SPC-13.
//!
//! Every case that writes runs on a `Scratch` project, never on a fixture in the repository's
//! own tree.

#[allow(dead_code, reason = "each test file uses part of the shared helper")]
mod common;

use std::path::Path;

use common::{NOTES, Ran, Scratch, Spawn};
use serde_json::json;

fn run(project: &Path, args: &[&str]) -> Ran {
    Spawn::args(args.iter().copied()).cwd(project).run()
}

fn ok_json(ran: &Ran) -> serde_json::Value {
    assert_eq!(ran.code, 0, "stderr: {}", ran.stderr);
    assert_eq!(ran.stderr, "");
    ran.stdout_json()
}

const WF_SCHEMA: &str = r#"{
  "name": "ticket",
  "code": "WF",
  "fields": {
    "title": { "type": "string", "required": true },
    "status": { "type": "enum", "values": ["open", "claimed"], "default": "open" },
    "kind": { "type": "enum", "values": ["research", "task"], "required": true }
  }
}"#;

const WF_COLLECTION: [(&str, &str); 2] = [
    (
        ".typdoc/collections/tickets.json",
        r#"{ "match": "tickets/{key}.md", "schema": "wf.json" }"#,
    ),
    ("wf.json", WF_SCHEMA),
];

fn wf_project(state_last: Option<u64>, existing: &[&str]) -> Scratch {
    let mut files: Vec<(&str, &str)> = WF_COLLECTION.to_vec();
    let state_text;
    if let Some(last) = state_last {
        state_text = format!("{{ \"tickets\": {{ \"last\": {last} }} }}");
        files.push((".typdoc/state/default.json", &state_text));
    }
    let project = Scratch::project(&files);
    for path in existing {
        project.file(
            path,
            "---\ntitle: Existing\nstatus: open\nkind: research\n---\n",
        );
    }
    project
}

// --- Allocation ---

#[test]
fn new_allocates_one_past_the_larger_of_the_highest_existing_number_and_last() {
    let project = wf_project(Some(1), &["tickets/WF-1.md"]);

    let ran = run(
        project.path(),
        &["new", "WF", "Second ticket", "--set", "kind=task", "--json"],
    );

    let out = ok_json(&ran);
    assert_eq!(out["document"]["key"], json!("WF-2"));
    assert_eq!(out["document"]["path"], json!("tickets/WF-2.md"));
    assert_eq!(out["document"]["fields"]["status"], json!("open"));
    assert_eq!(out["document"]["fields"]["kind"], json!("task"));
    assert!(
        project.path().join("tickets/WF-2.md").is_file(),
        "the document must actually be on disk"
    );
    let state = std::fs::read_to_string(project.path().join(".typdoc/state/default.json"))
        .expect("a state file");
    assert!(state.contains("\"last\": 2"), "{state}");
}

#[test]
fn a_hand_made_file_with_a_higher_number_than_last_is_respected() {
    let project = wf_project(Some(1), &["tickets/WF-1.md", "tickets/WF-5.md"]);

    let ran = run(
        project.path(),
        &["new", "WF", "Next ticket", "--set", "kind=task", "--json"],
    );

    let out = ok_json(&ran);
    assert_eq!(out["document"]["key"], json!("WF-6"));
}

#[test]
fn new_never_reissues_a_number_whose_document_was_deleted() {
    let project = wf_project(Some(1), &["tickets/WF-1.md"]);

    let first = run(
        project.path(),
        &[
            "new",
            "WF",
            "Will be deleted",
            "--set",
            "kind=task",
            "--json",
        ],
    );
    let first_out = ok_json(&first);
    assert_eq!(first_out["document"]["key"], json!("WF-2"));

    std::fs::remove_file(project.path().join("tickets/WF-2.md"))
        .expect("the document made by the first new can be removed");
    assert!(
        !project.path().join("tickets/WF-2.md").exists(),
        "the deletion must have actually happened"
    );

    let second = run(
        project.path(),
        &["new", "WF", "Replacement", "--set", "kind=task", "--json"],
    );
    let second_out = ok_json(&second);

    assert_eq!(
        second_out["document"]["key"],
        json!("WF-3"),
        "a deleted document's number must not be handed out again"
    );
    assert!(
        !project.path().join("tickets/WF-2.md").exists(),
        "the second new must not have recreated the deleted document under its old number"
    );
}

// --- Exit 7: a destination that already exists ---

#[test]
fn a_path_given_on_the_command_line_that_exists_is_refused_at_exit_7_untouched() {
    let project = Scratch::project(&NOTES);
    project.file("a.md", "---\ntitle: Already here\n---\n");
    let before = std::fs::read(project.path().join("a.md")).unwrap();

    let ran = run(
        project.path(),
        &[
            "new",
            "a.md",
            "--set",
            "title=Trying to overwrite",
            "--json",
        ],
    );

    assert_eq!(ran.code, 7, "stdout: {} stderr: {}", ran.stdout, ran.stderr);
    assert_eq!(ran.stdout, "");
    let error = ran.stderr_json();
    assert_eq!(error["code"], json!(7));

    let after = std::fs::read(project.path().join("a.md")).unwrap();
    assert_eq!(
        before, after,
        "the file that was already there must be byte-identical"
    );
}

/// Allocation goes past every file the index holds, so a file can be in the way only when the
/// index does not see it: a second collection overlapping `tickets/WF-2.md` drops that path from
/// the index (`collections.overlap`) while the file stays on disk. The destination is checked
/// before `last` is raised, so the refusal leaves the state file's bytes as they were.
#[test]
fn a_name_a_template_produces_that_exists_is_refused_at_exit_7_with_last_unchanged() {
    let files = [
        (
            ".typdoc/collections/tickets.json",
            r#"{ "match": "tickets/{key}.md", "schema": "wf.json" }"#,
        ),
        ("wf.json", WF_SCHEMA),
        // `tickets/WF-2.md` is the next number: `max(1, 1) + 1`.
        (
            ".typdoc/collections/collides.json",
            r#"{ "match": "tickets/WF-2.md", "schema": "note.json" }"#,
        ),
        ("note.json", r#"{ "name": "note", "fields": {} }"#),
        (
            ".typdoc/state/default.json",
            r#"{ "tickets": { "last": 1 } }"#,
        ),
    ];
    let project = Scratch::project(&files);
    project.file(
        "tickets/WF-1.md",
        "---\ntitle: Existing\nstatus: open\nkind: research\n---\n",
    );
    project.file(
        "tickets/WF-2.md",
        "---\ntitle: Hand-made, in the way\n---\n",
    );
    let before_document = std::fs::read(project.path().join("tickets/WF-2.md")).unwrap();
    let before_state = std::fs::read(project.path().join(".typdoc/state/default.json")).unwrap();

    let ran = run(
        project.path(),
        &["new", "WF", "Collides", "--set", "kind=task", "--json"],
    );

    assert_eq!(ran.code, 7, "stdout: {} stderr: {}", ran.stdout, ran.stderr);
    assert_eq!(ran.stdout, "");

    let after_document = std::fs::read(project.path().join("tickets/WF-2.md")).unwrap();
    assert_eq!(
        before_document, after_document,
        "the hand-made file must be byte-identical"
    );
    let after_state = std::fs::read(project.path().join(".typdoc/state/default.json")).unwrap();
    assert_eq!(
        before_state, after_state,
        "a refused new must not burn the number it would have allocated"
    );
}

// --- `state.missing` and `state.malformed` ---

#[test]
fn state_missing_refuses_new_with_nothing_written() {
    let project = wf_project(None, &["tickets/WF-1.md"]);

    let ran = run(
        project.path(),
        &[
            "new",
            "WF",
            "Should not be created",
            "--set",
            "kind=task",
            "--json",
        ],
    );

    assert_eq!(ran.code, 2, "stdout: {} stderr: {}", ran.stdout, ran.stderr);
    let error = ran.stderr_json();
    let details = error["details"].as_array().expect("a details array");
    assert_eq!(details.len(), 1, "{details:?}");
    assert_eq!(details[0]["rule"], json!("state.missing"));

    assert!(
        !project.path().join(".typdoc/state/default.json").exists(),
        "a refused new must not create the state file"
    );
    let entries: Vec<_> = std::fs::read_dir(project.path().join("tickets"))
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(entries, vec![std::ffi::OsString::from("WF-1.md")]);
}

#[test]
fn state_malformed_refuses_new_with_the_state_file_untouched() {
    let files = [
        (
            ".typdoc/collections/tickets.json",
            r#"{ "match": "tickets/{key}.md", "schema": "wf.json" }"#,
        ),
        ("wf.json", WF_SCHEMA),
        (
            ".typdoc/state/default.json",
            r#"{ "tickets": { "last": "three" } }"#,
        ),
    ];
    let project = Scratch::project(&files);
    let before = std::fs::read(project.path().join(".typdoc/state/default.json")).unwrap();

    let ran = run(
        project.path(),
        &[
            "new",
            "WF",
            "Should not be created",
            "--set",
            "kind=task",
            "--json",
        ],
    );

    assert_eq!(ran.code, 2, "stdout: {} stderr: {}", ran.stdout, ran.stderr);
    let details = ran.stderr_json()["details"].as_array().unwrap().clone();
    assert_eq!(details[0]["rule"], json!("state.malformed"));

    let after = std::fs::read(project.path().join(".typdoc/state/default.json")).unwrap();
    assert_eq!(before, after);
    assert!(
        std::fs::read_dir(project.path().join("tickets")).is_err(),
        "no document must have been created, and the folder itself must not exist either"
    );
}

// --- Validation, before anything is written ---

#[test]
fn a_missing_required_field_refuses_new_and_burns_no_number() {
    let project = wf_project(Some(1), &["tickets/WF-1.md"]);
    let before_state = std::fs::read(project.path().join(".typdoc/state/default.json")).unwrap();

    // `kind` is required and not given.
    let ran = run(project.path(), &["new", "WF", "No kind", "--json"]);

    assert_eq!(ran.code, 2, "stdout: {} stderr: {}", ran.stdout, ran.stderr);
    let details = ran.stderr_json()["details"].as_array().unwrap().clone();
    assert!(
        details
            .iter()
            .any(|d| d["rule"] == json!("frontmatter.types")),
        "{details:?}"
    );

    assert!(!project.path().join("tickets/WF-2.md").exists());
    let after_state = std::fs::read(project.path().join(".typdoc/state/default.json")).unwrap();
    assert_eq!(before_state, after_state);
}

#[test]
fn writing_an_auto_field_directly_through_set_is_refused() {
    let files = [
        (
            ".typdoc/collections/tickets.json",
            r#"{ "match": "tickets/{key}.md", "schema": "wf.json" }"#,
        ),
        (
            "wf.json",
            r#"{ "name": "wf", "code": "WF", "fields": {
                "title": { "type": "string" },
                "created_at": { "type": "datetime", "auto": "create" }
            } }"#,
        ),
        (
            ".typdoc/state/default.json",
            r#"{ "tickets": { "last": 0 } }"#,
        ),
    ];
    let project = Scratch::project(&files);

    let ran = run(
        project.path(),
        &[
            "new",
            "WF",
            "A ticket",
            "--set",
            "created_at=2020-01-01T00:00:00+00:00",
            "--json",
        ],
    );

    assert_eq!(ran.code, 2, "stdout: {} stderr: {}", ran.stdout, ran.stderr);
    let details = ran.stderr_json()["details"].as_array().unwrap().clone();
    assert_eq!(details[0]["rule"], json!("frontmatter.types"));
    assert!(!project.path().join("tickets/WF-1.md").exists());
}

/// The binary runs on the real clock, so only the shape of the stamp can be checked, not its
/// instant.
#[test]
fn auto_create_is_stamped_with_a_value_that_looks_like_a_datetime() {
    let files = [
        (
            ".typdoc/collections/tickets.json",
            r#"{ "match": "tickets/{key}.md", "schema": "wf.json" }"#,
        ),
        (
            "wf.json",
            r#"{ "name": "wf", "code": "WF", "fields": {
                "title": { "type": "string" },
                "created_at": { "type": "datetime", "auto": "create" }
            } }"#,
        ),
        (
            ".typdoc/state/default.json",
            r#"{ "tickets": { "last": 0 } }"#,
        ),
    ];
    let project = Scratch::project(&files);

    let ran = run(project.path(), &["new", "WF", "A ticket", "--json"]);

    let out = ok_json(&ran);
    let stamped = out["document"]["fields"]["created_at"]
        .as_str()
        .expect("created_at is a string")
        .to_owned();
    assert!(stamped.contains('T'), "{stamped}");
    assert!(
        stamped.ends_with("+00:00") || stamped.contains('+') || stamped.contains('-'),
        "the stamped value should carry an offset: {stamped}"
    );
}

#[test]
fn a_scope_holding_more_than_one_namespace_exits_1_with_the_choices() {
    let project = Scratch::empty();
    project.file(
        ".typdoc/config.json",
        r#"{ "version": 1, "namespaces": ["story-1", "story-2"] }"#,
    );
    project.file(
        ".typdoc/collections/tickets.json",
        r#"{ "match": "tickets/{key}.md", "schema": "wf.json" }"#,
    );
    project.file("wf.json", WF_SCHEMA);
    project.file("story-1/README.md", "---\ntitle: One\n---\n");
    project.file("story-2/README.md", "---\ntitle: Two\n---\n");

    let ran = run(
        project.path(),
        &["new", "WF", "Ambiguous", "--set", "kind=task", "--json"],
    );

    assert_eq!(ran.code, 1, "stdout: {} stderr: {}", ran.stdout, ran.stderr);
    let error = ran.stderr_json();
    assert_eq!(error["code"], json!(1));
    let candidates = error["candidates"].as_array().expect("a candidates array");
    let names: Vec<&str> = candidates.iter().map(|v| v.as_str().unwrap()).collect();
    assert_eq!(names, vec!["story-1", "story-2"]);

    // `--namespace` names exactly one, and the write goes through.
    let ran = run(
        project.path(),
        &[
            "new",
            "WF",
            "Not ambiguous",
            "--set",
            "kind=task",
            "--namespace",
            "story-1",
            "--json",
        ],
    );
    let out = ok_json(&ran);
    assert_eq!(out["document"]["namespace"], json!("story-1"));
    assert_eq!(out["document"]["path"], json!("story-1/tickets/WF-1.md"));
}

// --- The path form ---

#[test]
fn the_path_form_creates_at_the_given_path_with_defaults_filled() {
    let project = Scratch::project(&NOTES);

    let ran = run(
        project.path(),
        &[
            "new",
            "a-new-note.md",
            "--set",
            "title=A new note",
            "--json",
        ],
    );

    let out = ok_json(&ran);
    assert_eq!(out["document"]["path"], json!("a-new-note.md"));
    assert!(out["document"]["key"].is_null() || out["document"].get("key").is_none());
    assert_eq!(out["document"]["code"], json!(null));
    assert!(project.path().join("a-new-note.md").is_file());
}

#[test]
fn a_path_matching_no_collection_is_bad_arguments() {
    let project = Scratch::project(&NOTES);

    let ran = run(
        project.path(),
        &["new", "elsewhere/note.txt", "--set", "title=X", "--json"],
    );

    assert_eq!(ran.code, 1, "stdout: {} stderr: {}", ran.stdout, ran.stderr);
}

#[test]
fn a_path_matching_a_coded_collection_names_the_coded_form_instead() {
    let project = wf_project(Some(0), &[]);

    let ran = run(
        project.path(),
        &["new", "tickets/WF-9.md", "--set", "title=X", "--json"],
    );

    assert_eq!(ran.code, 1, "stdout: {} stderr: {}", ran.stdout, ran.stderr);
    let error = ran.stderr_json();
    assert!(
        error["error"].as_str().unwrap().contains("coded"),
        "{error}"
    );
}

// --- Text output ---

/// Fields print in the order `--set` gave them, which is the order they were written in.
#[test]
fn new_path_without_json_prints_the_labeled_block() {
    let project = Scratch::project(&NOTES);

    let ran = run(
        project.path(),
        &[
            "new",
            "a-new-note.md",
            "--set",
            "title=A new note",
            "--set",
            "tags=a,b",
        ],
    );

    assert_eq!(ran.code, 0, "stderr: {}", ran.stderr);
    assert_eq!(ran.stderr, "");
    assert_eq!(
        ran.stdout,
        "path: a-new-note.md\n\
         collection: notes\n\
         schema: note\n\
         namespace: default\n\
         title: A new note\n\
         tags: a,b\n"
    );
}

/// `status` prints first because defaults are filled before `--set`, and `title` comes before
/// every `--set` field; `fixtures/output/new/coded/golden/stdout.json` pins the same order.
#[test]
fn new_coded_without_json_prints_the_labeled_block_not_the_bare_key() {
    let project = wf_project(Some(1), &["tickets/WF-1.md"]);

    let ran = run(
        project.path(),
        &["new", "WF", "Second ticket", "--set", "kind=task"],
    );

    assert_eq!(ran.code, 0, "stderr: {}", ran.stderr);
    assert_eq!(ran.stderr, "");
    assert_eq!(
        ran.stdout,
        "path: tickets/WF-2.md\n\
         collection: tickets\n\
         schema: ticket\n\
         namespace: default\n\
         key: WF-2\n\
         status: open\n\
         title: Second ticket\n\
         kind: task\n"
    );
}

// --- `--set` escaping: `new` and `set` share one parser (`apply_ops`), so `set.rs` holds the
// full set of cases ---

#[test]
fn news_own_set_unescapes_a_backslash_star_to_a_literal_star() {
    let project = Scratch::project(&NOTES);

    let ran = run(
        project.path(),
        &["new", "a.md", "--set", r"title=a\*b", "--json"],
    );

    let out = ok_json(&ran);
    assert_eq!(out["document"]["fields"]["title"], json!("a*b"));
}

#[test]
fn news_own_set_refuses_a_bare_unescaped_star_and_creates_nothing() {
    let project = Scratch::project(&NOTES);

    let ran = run(
        project.path(),
        &["new", "a.md", "--set", "title=x*y", "--json"],
    );

    assert_eq!(ran.code, 1, "stdout: {} stderr: {}", ran.stdout, ran.stderr);
    assert!(
        !project.path().join("a.md").exists(),
        "a refused new must create nothing"
    );
}

#[test]
fn a_bad_target_without_json_prints_a_plain_text_error_not_the_json_object() {
    let project = Scratch::project(&NOTES);

    let ran = run(project.path(), &["new", "not a code and not a path"]);

    assert_eq!(ran.code, 1, "stdout: {} stderr: {}", ran.stdout, ran.stderr);
    assert_eq!(ran.stdout, "");
    assert_eq!(
        ran.stderr,
        "typdoc: `not a code and not a path` is neither a schema's code (`[A-Z][A-Z0-9]*`) nor a \
         path, which ends in `.md`\n"
    );
}

#[test]
fn a_malformed_set_argument_without_json_prints_a_plain_text_error() {
    let project = wf_project(Some(0), &[]);

    let ran = run(
        project.path(),
        &["new", "WF", "A ticket", "--set", "no-equals-sign"],
    );

    assert_eq!(ran.code, 1, "stdout: {} stderr: {}", ran.stdout, ran.stderr);
    assert_eq!(ran.stdout, "");
    assert!(
        ran.stderr.starts_with("typdoc: ") && !ran.stderr.starts_with("typdoc: {"),
        "{}",
        ran.stderr
    );
}

#[test]
fn a_duplicate_target_without_json_prints_a_plain_text_error() {
    let project = Scratch::project(&NOTES);
    project.file("a.md", "---\ntitle: Already here\n---\n");
    let before = std::fs::read(project.path().join("a.md")).unwrap();

    let ran = run(
        project.path(),
        &["new", "a.md", "--set", "title=Trying to overwrite"],
    );

    assert_eq!(ran.code, 7, "stdout: {} stderr: {}", ran.stdout, ran.stderr);
    assert_eq!(ran.stdout, "");
    assert!(
        ran.stderr.starts_with("typdoc: ") && !ran.stderr.starts_with("typdoc: {"),
        "{}",
        ran.stderr
    );

    let after = std::fs::read(project.path().join("a.md")).unwrap();
    assert_eq!(before, after);
}

#[test]
fn the_key_from_json_can_be_captured_and_fed_back_to_get() {
    let project = wf_project(Some(0), &[]);

    let ran = run(
        project.path(),
        &["new", "WF", "Round trip", "--set", "kind=task", "--json"],
    );

    let out = ok_json(&ran);
    let key = out["document"]["key"].as_str().expect("a key").to_owned();

    let got = run(project.path(), &["get", &key, "--json"]);
    let out = ok_json(&got);
    assert_eq!(out["document"]["key"], json!(key));
    assert_eq!(out["document"]["fields"]["title"], json!("Round trip"));
}

// --- `refs.acyclic` at write time ---

const WF_ACYCLIC_SCHEMA: &str = r#"{
  "name": "ticket",
  "code": "WF",
  "fields": {
    "title": { "type": "string", "required": true },
    "blocked_by": { "type": "ref[]", "target": "*", "acyclic": true }
  }
}"#;

/// `WF-1` holds a dangling `blocked_by: [WF-2]`, and `WF-2` is the number `new` allocates next,
/// so the new document closes `WF-1 -> WF-2 -> WF-1` as it is created.
#[test]
fn new_refuses_a_write_that_would_close_a_cycle_with_an_existing_document() {
    let files = [
        (
            ".typdoc/collections/tickets.json",
            r#"{ "match": "tickets/{key}.md", "schema": "wf.json" }"#,
        ),
        ("wf.json", WF_ACYCLIC_SCHEMA),
        (
            ".typdoc/state/default.json",
            r#"{ "tickets": { "last": 1 } }"#,
        ),
    ];
    let project = Scratch::project(&files);
    project.file(
        "tickets/WF-1.md",
        "---\ntitle: One\nblocked_by: [WF-2]\n---\n",
    );
    let state_before = std::fs::read(project.path().join(".typdoc/state/default.json")).unwrap();

    let ran = run(
        project.path(),
        &["new", "WF", "Two", "--set", "blocked_by=WF-1", "--json"],
    );

    assert_eq!(ran.code, 2, "stdout: {} stderr: {}", ran.stdout, ran.stderr);
    let error = ran.stderr_json();
    let details = error["details"].as_array().expect("a details array");
    assert!(
        details.iter().any(|f| f["rule"] == json!("refs.acyclic")),
        "{details:?}"
    );

    assert!(
        !project.path().join("tickets/WF-2.md").exists(),
        "a refused new must not create the document that would have closed the cycle"
    );
    let state_after = std::fs::read(project.path().join(".typdoc/state/default.json")).unwrap();
    assert_eq!(
        state_before, state_after,
        "a refused new must not burn the number it would have allocated"
    );
}

#[test]
fn new_is_not_refused_by_an_unrelated_pre_existing_cycle_elsewhere() {
    let files = [
        (
            ".typdoc/collections/tickets.json",
            r#"{ "match": "tickets/{key}.md", "schema": "wf.json" }"#,
        ),
        ("wf.json", WF_ACYCLIC_SCHEMA),
        (
            ".typdoc/state/default.json",
            r#"{ "tickets": { "last": 11 } }"#,
        ),
    ];
    let project = Scratch::project(&files);
    project.file(
        "tickets/WF-10.md",
        "---\ntitle: Ten\nblocked_by: [WF-11]\n---\n",
    );
    project.file(
        "tickets/WF-11.md",
        "---\ntitle: Eleven\nblocked_by: [WF-10]\n---\n",
    );

    let ran = run(project.path(), &["new", "WF", "Unrelated", "--json"]);

    let out = ok_json(&ran);
    assert_eq!(out["document"]["fields"]["title"], json!("Unrelated"));
}
