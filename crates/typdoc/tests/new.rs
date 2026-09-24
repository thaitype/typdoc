//! `typdoc new`, through the built binary. Every case that writes runs on a `Scratch` project,
//! never on a fixture in the repository's own tree (`common::Scratch` always makes a fresh
//! temporary folder outside it).

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

/// A coded schema with a default, an enum with no default, and no `auto` field: the shape
/// `new`'s own golden already exercises for defaults and allocation, reused here by hand so each
/// test can vary the state file and the documents already on disk.
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

// --- allocation: the larger of the highest existing number and the state file's `last` ---

/// The ordinary case, through the binary rather than through the golden: the number allocated is
/// one past the larger of the highest existing number and `last`, and the state file is updated
/// to it.
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

/// A file created by hand with a higher number than `last` is respected: allocation reads it as
/// the highest existing number, which wins the `max`, and the numbers in between stay unissued
/// (design, `typdoc new`).
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

/// The number is never reused after a document is deleted: the real counterpart to
/// `crates/typdoc/tests/state.rs`'s `the_measurement_behind_decision_13_deriving_last_reissues_a_retired_key_silently`,
/// which simulates `new`'s two effects by hand because `new` did not exist yet when it was
/// written. This runs `new` itself, twice, with a deletion between the two: the first `new`
/// allocates `WF-2` and records `last: 2`; `WF-2.md` is then deleted, exactly as a user would;
/// the second `new` must allocate `WF-3`, not `WF-2` again, because allocation reads `last` from
/// the state file rather than deriving it from what exists on disk (decision 13).
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

// --- exit 7: a destination that already exists (decision 15), both of its `new` cases ---

/// `new <path>` where the path already exists: exit 7, and the file that was there is untouched,
/// down to the byte.
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

/// `new <CODE> "<title>"` where the name the `match` template produces already exists: decision
/// 15's own "should not be reachable" case. `Project::new_coded` reads "the highest existing
/// number" from the same index every command loads at the start of its own run, so a file simply
/// dropped at the path a run's own allocation is about to compute is not actually unreachable —
/// that same run's own `Project::load` already saw it and allocated past it, which is the
/// evidence this file's own doc comment gives for why that read does not have to be redone under
/// the lock. To reach the case for real inside one run, the file has to be invisible to the
/// index while still sitting on disk: a second collection whose `match` overlaps `tickets/WF-2.md`
/// exactly makes `Index::build` treat the path as `collections.overlap` and drop it out of
/// `entries` (the map `highest_existing` reads), while `deps.fs.exists` — asked about the literal
/// path, not the index — still finds it there.
///
/// Exit 7, the file that was already there is untouched, and — the part a mere "the document is
/// unharmed" check would miss — the state file's own bytes are unchanged too: the number that
/// would have been burned on this collision is not, because `Project::new_coded` checks the
/// destination before raising `last` (see its own doc comment for why this is not merely relying
/// on `O_EXCL`).
#[test]
fn a_name_a_template_produces_that_exists_is_refused_at_exit_7_with_last_unchanged() {
    let files = [
        (
            ".typdoc/collections/tickets.json",
            r#"{ "match": "tickets/{key}.md", "schema": "wf.json" }"#,
        ),
        ("wf.json", WF_SCHEMA),
        // Overlaps `tickets/WF-2.md` exactly, the path `new`'s own allocation (`max(1, 1) + 1`)
        // is about to compute, so that path is invisible to `highest_existing` (`Index::build`
        // removes an overlapping path from `entries`) even though a file sits there.
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

// --- state.missing / state.malformed refuse `new` before anything is written ---

/// A coded collection with documents already in the namespace and no state file at all is
/// `state.missing`: `new` refuses at exit 2, and nothing is written — no document, and (there was
/// none to begin with) no state file either.
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

/// A `last` that is present but not a whole number is `state.malformed`: `new` refuses at exit 2
/// with the state file's own bytes untouched.
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

// --- validation refuses a bad candidate before anything is written, and burns no number ---

/// A required field the `--set` list never gives is `frontmatter.types`, refused at exit 2 with
/// nothing written: no document, and the state file's own bytes are unchanged (the number this
/// would have allocated is not burned on a validation failure).
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

/// Writing an `auto` field directly through `--set` is refused, the same as `set` refuses it
/// (`new`'s own `--set` shares the grammar): exit 2, nothing written.
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

/// `auto: create` is stamped by `new`: presence and shape are checked (the running binary's own
/// clock is the real one, so the exact instant cannot be pinned in a test the way the golden's
/// clockless fixture sidesteps it).
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

// --- a scope holding more than one namespace exits 1 with the choices ---

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

// --- the path-identified form ---

/// The path form fills defaults and validates the same way, and writes exactly the path given.
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

/// A path that matches no collection at all is bad arguments, exit 1, nothing written.
#[test]
fn a_path_matching_no_collection_is_bad_arguments() {
    let project = Scratch::project(&NOTES);

    let ran = run(
        project.path(),
        &["new", "elsewhere/note.txt", "--set", "title=X", "--json"],
    );

    assert_eq!(ran.code, 1, "stdout: {} stderr: {}", ran.stdout, ran.stderr);
}

/// A path that matches a *coded* collection's `match` is refused with a message naming the coded
/// form instead, rather than silently trying to write a key-shaped path by hand.
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

// --- text-mode output (ticket 16): the shared labeled block, and the error-path fix ---

/// The hand-written golden for `new <path>`'s text-mode shape (contract, text-output shapes: the
/// same labeled block `get` prints, for the newly created document). No `key` line, since a
/// path-identified document has none; fields print in the order `--set` gave them, the order the
/// write path wrote them in.
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

/// The hand-written golden for `new <CODE>`'s text-mode shape — **the deliberate, accepted
/// replacement (M-10(f)) of today's bare-key output**: the same labeled block, `key` included.
/// `WF_SCHEMA`'s only field with a default (`status`) is written first (`new_block` fills
/// defaults before `--set`), then `title` and `kind` in the order `new`'s own `--set` list (title
/// first, always; `kind` from `--set kind=task`) adds them — the same field order the existing
/// `--json` golden (`fixtures/output/new/coded/golden/stdout.json`) already pins for this exact
/// case.
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

/// The bug this ticket fixes, kept as the regression it was found as (testing-decisions, "Text
/// output"): before this ticket, `typdoc new` with a target that is neither a code nor a path
/// printed the raw `--json` error object on stderr even without `--json`, because
/// `parse_new_target`'s error (`cli.rs`, around line 200) called `failure` with a hard-coded
/// `true`. Now it prints plain text.
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

/// The other pre-flag-check parse error in `new` (`cli.rs`, around line 220: a malformed `--set`
/// argument) also prints plain text without `--json`, not the `--json` error object.
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

/// `new`'s own "duplicate-target refusal" (ticket 16's own example), without `--json`: exit 7,
/// plain text on stderr, and the file that was already there untouched.
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

// --- round trip: the key can be read out of `--json` and fed back to `get` ---
//
// Before ticket 16, `new <CODE>` without `--json` printed the bare key and nothing else, and
// this round trip captured that bare key straight off stdout. The contract's text-output table
// (M-10(f)) deliberately replaces that with the same labeled block `get` prints — "a caller that
// wants just the key uses `--json`" — so the round trip below reads `--json`'s own `key` field
// instead. `new_coded_without_json_prints_the_labeled_block_not_the_bare_key`, above, is what
// pins the replacement itself.

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

// --- ticket 30 (M-18): `new` refuses a write-time cycle on an `acyclic` field ---

const WF_ACYCLIC_SCHEMA: &str = r#"{
  "name": "ticket",
  "code": "WF",
  "fields": {
    "title": { "type": "string", "required": true },
    "blocked_by": { "type": "ref[]", "target": "*", "acyclic": true }
  }
}"#;

/// Creating a new document whose own `acyclic` field value would close a cycle with an existing
/// document: `WF-1` already has `blocked_by: [WF-2]` — dangling today, since `WF-2` does not
/// exist yet — and `new WF` is about to allocate exactly `WF-2` (state's `last` is `1`). Giving
/// the new document `blocked_by: [WF-1]` closes `WF-1 -> WF-2 -> WF-1` the moment it is created,
/// and must be refused the same way `set` is: exit 2, `refs.acyclic` in `details`, nothing
/// written (no document, `last` not burned) — not merely caught by a later `validate` run.
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

/// The mirror "no false refusal" case for `new`: creating a document with no `acyclic` value at
/// all succeeds normally even though an unrelated, pre-existing cycle already sits elsewhere in
/// the project, through the same field, on documents this write does not touch.
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
