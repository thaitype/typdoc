//! Covers SPC-2, SPC-4, SPC-5, SPC-12, SPC-13.
//!
//! Every case that writes runs on a copy (a `Scratch` project, or a fixture staged with
//! `typdoc_testkit::staging::stage`), never on a fixture in the repository's own tree.

#[allow(dead_code, reason = "each test file uses part of the shared helper")]
mod common;

use std::path::Path;

use common::{NOTES, Ran, Scratch, Spawn, fixture};
use serde_json::{Value, json};

fn run(project: &Path, args: &[&str]) -> Ran {
    Spawn::args(args.iter().copied()).cwd(project).run()
}

fn ok_json(ran: &Ran) -> Value {
    assert_eq!(ran.code, 0, "stderr: {}", ran.stderr);
    assert_eq!(ran.stderr, "");
    ran.stdout_json()
}

const STATUS_SCHEMA: &str = r#"{
  "name": "ticket",
  "fields": {
    "title": { "type": "string", "required": true },
    "status": {
      "type": "enum",
      "values": ["open", "claimed", "resolved", "closed"],
      "transitions": { "open": ["claimed", "closed"], "claimed": ["resolved", "open", "closed"] }
    }
  }
}"#;

const STATUS_COLLECTION: &str = r#"{ "match": "*.md", "schema": "ticket.json" }"#;

#[test]
fn the_transitions_fixture_exits_2_names_the_finding_and_leaves_the_document_unchanged() {
    let dir = fixture("broken/frontmatter.transitions");
    let spec = typdoc_testkit::spec::FixtureSpec::load(&dir, "frontmatter.transitions")
        .expect("the fixture's own spec");
    assert!(
        spec.is_write(),
        "a `set` fixture must be classified as a write"
    );
    let staged = typdoc_testkit::staging::stage(&dir, &spec).expect("staged outside the repo");
    let before = std::fs::read(dir.join("ticket.md")).unwrap();

    let ran = Spawn::args(&spec.command).cwd(staged.dir()).run();

    assert_eq!(ran.code, 2, "stdout: {} stderr: {}", ran.stdout, ran.stderr);
    assert_eq!(ran.stdout, "");
    let error = ran.stderr_json();
    assert_eq!(error["code"], json!(2));
    let details = error["details"].as_array().expect("a details array");
    assert_eq!(details.len(), 1, "{details:?}");
    assert_eq!(details[0]["rule"], json!("frontmatter.transitions"));
    assert_eq!(
        details[0]["message"],
        json!("transition not allowed: open -> resolved")
    );

    let after = std::fs::read(staged.dir().join("ticket.md")).unwrap();
    assert_eq!(
        before, after,
        "a refused write must leave the document's bytes untouched"
    );
    let repo_copy = std::fs::read(dir.join("ticket.md")).unwrap();
    assert_eq!(
        before, repo_copy,
        "the repository's own fixture must be untouched too"
    );
}

#[test]
fn a_false_if_writes_nothing_and_exits_3_naming_the_condition() {
    let project = Scratch::project(&[
        (".typdoc/collections/tickets.json", STATUS_COLLECTION),
        ("ticket.json", STATUS_SCHEMA),
        ("a.md", "---\ntitle: A ticket\nstatus: open\n---\n\nBody.\n"),
    ]);
    let before = std::fs::read(project.path().join("a.md")).unwrap();

    let ran = run(
        project.path(),
        &[
            "set",
            "a.md",
            "status=claimed",
            "--if",
            "status=closed",
            "--json",
        ],
    );

    assert_eq!(ran.code, 3, "stdout: {} stderr: {}", ran.stdout, ran.stderr);
    assert_eq!(ran.stdout, "");
    let error = ran.stderr_json();
    assert_eq!(error["code"], json!(3));
    let details = error["details"].as_array().expect("a details array");
    assert_eq!(details.len(), 1, "{details:?}");
    assert_eq!(details[0]["message"], json!("`status=closed` is false"));

    let after = std::fs::read(project.path().join("a.md")).unwrap();
    assert_eq!(before, after);
}

#[test]
fn a_true_if_lets_the_write_through() {
    let project = Scratch::project(&[
        (".typdoc/collections/tickets.json", STATUS_COLLECTION),
        ("ticket.json", STATUS_SCHEMA),
        ("a.md", "---\ntitle: A ticket\nstatus: open\n---\n\nBody.\n"),
    ]);

    let ran = run(
        project.path(),
        &[
            "set",
            "a.md",
            "status=claimed",
            "--if",
            "status=open",
            "--json",
        ],
    );

    let out = ok_json(&ran);
    assert_eq!(out["document"]["fields"]["status"], json!("claimed"));
}

#[test]
fn a_document_outside_every_namespace_is_an_ordinary_write_with_no_namespace_in_json() {
    let project = Scratch::empty();
    project.file(
        ".typdoc/config.json",
        r#"{ "version": 1, "namespaces": ["story-1", "story-2"] }"#,
    );
    project.file(
        ".typdoc/collections/notes.json",
        r#"{ "match": "*.md", "schema": "schemas/n.json" }"#,
    );
    project.file("schemas/n.json", r#"{ "name": "n", "fields": {} }"#);
    project.file("README.md", "---\ntitle: Root\n---\n\nBody.\n");
    project.file("story-1/a.md", "---\ntitle: A\n---\n\nBody.\n");
    project.file("story-2/b.md", "---\ntitle: B\n---\n\nBody.\n");

    let ran = run(
        project.path(),
        &["set", "README.md", "title=Changed", "--json"],
    );

    let out = ok_json(&ran);
    assert_eq!(out["document"]["path"], json!("README.md"));
    assert_eq!(out["document"]["fields"]["title"], json!("Changed"));
    assert!(out["document"].get("namespace").is_none(), "{out}");

    let after = std::fs::read_to_string(project.path().join("README.md")).unwrap();
    assert_eq!(after, "---\ntitle: Changed\n---\n\nBody.\n");
}

/// Checked on the bytes of standard output: parsed JSON cannot tell `99999999999999999999` from
/// what a primitive rounds it to.
#[test]
fn a_number_no_primitive_holds_on_an_untouched_field_is_written_back_and_printed_unchanged() {
    let project = Scratch::project(&[
        (
            ".typdoc/collections/notes.json",
            r#"{ "match": "*.md", "schema": "note.json" }"#,
        ),
        (
            "note.json",
            r#"{ "name": "note", "fields": { "title": { "type": "string" }, "count": { "type": "number" } } }"#,
        ),
        (
            "a.md",
            "---\ntitle: Before\ncount: 99999999999999999999\n---\n\nBody.\n",
        ),
    ]);

    let ran = run(project.path(), &["set", "a.md", "title=After", "--json"]);

    assert_eq!(ran.code, 0, "stderr: {}", ran.stderr);
    assert!(
        ran.stdout.contains(r#""count":99999999999999999999"#),
        "the digits must appear unconverted on stdout: {}",
        ran.stdout
    );

    // The writer may quote the scalar (`yaml_serde` quotes one it would otherwise read as
    // something else), and a read gives the same text either way, so what is checked is that the
    // digits survive: on disk, and through `get`.
    let after = std::fs::read_to_string(project.path().join("a.md")).unwrap();
    assert!(
        after.contains("99999999999999999999"),
        "the digits must survive a write that never touched this field: {after:?}"
    );
    let reread = run(project.path(), &["get", "a.md", "--json"]);
    assert_eq!(reread.code, 0, "stderr: {}", reread.stderr);
    assert!(
        reread.stdout.contains(r#""count":99999999999999999999"#),
        "a later read must still print the same unconverted digits: {}",
        reread.stdout
    );
}

/// `set` changes only `title`, so the block keeps the file's order: `title`, then `tags`.
#[test]
fn set_without_json_prints_the_labeled_block_after_the_write() {
    let project = Scratch::project(&NOTES);
    project.file(
        "note.md",
        "---\ntitle: A minimal note\ntags: [alpha, beta]\n---\n\nBody.\n",
    );

    let ran = run(project.path(), &["set", "note.md", "title=A changed note"]);

    assert_eq!(ran.code, 0, "stderr: {}", ran.stderr);
    assert_eq!(ran.stderr, "");
    assert_eq!(
        ran.stdout,
        "path: note.md\n\
         collection: notes\n\
         schema: note\n\
         namespace: default\n\
         title: A changed note\n\
         tags: alpha,beta\n"
    );
}

#[test]
fn set_without_json_prints_a_plain_text_error_on_a_validation_failure() {
    let project = Scratch::project(&[
        (".typdoc/collections/tickets.json", STATUS_COLLECTION),
        ("ticket.json", STATUS_SCHEMA),
        ("a.md", "---\ntitle: A ticket\nstatus: open\n---\n\nBody.\n"),
    ]);
    let before = std::fs::read(project.path().join("a.md")).unwrap();

    let ran = run(project.path(), &["set", "a.md", "status=resolved"]);

    assert_eq!(ran.code, 2, "stdout: {} stderr: {}", ran.stdout, ran.stderr);
    assert_eq!(ran.stdout, "");
    assert!(
        ran.stderr.starts_with("typdoc: ") && !ran.stderr.starts_with("typdoc: {"),
        "{}",
        ran.stderr
    );

    let after = std::fs::read(project.path().join("a.md")).unwrap();
    assert_eq!(
        before, after,
        "a refused write must leave the file untouched"
    );
}

#[test]
fn k_with_nothing_after_the_equals_removes_the_field() {
    let project = Scratch::project(&NOTES);
    project.file("a.md", "---\ntitle: Keep\nowner: someone\n---\n\nBody.\n");

    let ran = run(project.path(), &["set", "a.md", "owner=", "--json"]);

    let out = ok_json(&ran);
    assert!(out["document"]["fields"].get("owner").is_none(), "{out}");

    let after = std::fs::read_to_string(project.path().join("a.md")).unwrap();
    assert!(!after.contains("owner"), "{after:?}");
}

#[test]
fn a_comma_separated_value_replaces_a_list_field_rather_than_appending_to_it() {
    let project = Scratch::project(&[
        (
            ".typdoc/collections/notes.json",
            r#"{ "match": "*.md", "schema": "note.json" }"#,
        ),
        (
            "note.json",
            r#"{ "name": "note", "fields": { "tags": { "type": "list" } } }"#,
        ),
        ("a.md", "---\ntags: [alpha, beta]\n---\n\nBody.\n"),
    ]);

    let ran = run(
        project.path(),
        &["set", "a.md", "tags=gamma,delta", "--json"],
    );

    let out = ok_json(&ran);
    assert_eq!(out["document"]["fields"]["tags"], json!(["gamma", "delta"]));
}

#[test]
fn writing_an_auto_field_directly_is_refused_and_writes_nothing() {
    let project = Scratch::project(&[
        (
            ".typdoc/collections/notes.json",
            r#"{ "match": "*.md", "schema": "note.json" }"#,
        ),
        (
            "note.json",
            r#"{ "name": "note", "fields": { "title": { "type": "string" }, "updated_at": { "type": "datetime", "auto": "update" } } }"#,
        ),
        (
            "a.md",
            "---\ntitle: Before\nupdated_at: 2026-01-01T00:00:00+00:00\n---\n\nBody.\n",
        ),
    ]);
    let before = std::fs::read(project.path().join("a.md")).unwrap();

    let ran = run(
        project.path(),
        &[
            "set",
            "a.md",
            "updated_at=2020-01-01T00:00:00+00:00",
            "--json",
        ],
    );

    assert_eq!(ran.code, 2, "stdout: {} stderr: {}", ran.stdout, ran.stderr);
    let error = ran.stderr_json();
    let details = error["details"].as_array().expect("a details array");
    assert_eq!(details[0]["rule"], json!("frontmatter.types"));

    let after = std::fs::read(project.path().join("a.md")).unwrap();
    assert_eq!(before, after);
}

#[test]
fn auto_update_is_stamped_only_when_a_value_actually_changes() {
    let schema = r#"{ "name": "note", "fields": {
        "title": { "type": "string" },
        "updated_at": { "type": "datetime", "auto": "update" }
    } }"#;
    let doc = "---\ntitle: Same\nupdated_at: 2026-01-01T00:00:00+00:00\n---\n\nBody.\n";

    let unchanged = Scratch::project(&[
        (
            ".typdoc/collections/notes.json",
            r#"{ "match": "*.md", "schema": "note.json" }"#,
        ),
        ("note.json", schema),
        ("a.md", doc),
    ]);
    let ran = run(unchanged.path(), &["set", "a.md", "title=Same", "--json"]);
    let out = ok_json(&ran);
    assert_eq!(
        out["document"]["fields"]["updated_at"],
        json!("2026-01-01T00:00:00+00:00"),
        "setting a field to the value it already holds is not a change"
    );

    let changed = Scratch::project(&[
        (
            ".typdoc/collections/notes.json",
            r#"{ "match": "*.md", "schema": "note.json" }"#,
        ),
        ("note.json", schema),
        ("a.md", doc),
    ]);
    let ran = run(
        changed.path(),
        &["set", "a.md", "title=Different", "--json"],
    );
    let out = ok_json(&ran);
    assert_ne!(
        out["document"]["fields"]["updated_at"],
        json!("2026-01-01T00:00:00+00:00"),
        "a real change must stamp `auto: update`"
    );
}

// --- `field=value` escaping ---

#[test]
fn a_backslash_star_in_a_scalar_value_is_stored_as_a_literal_star() {
    let project = Scratch::project(&NOTES);
    project.file("a.md", "---\ntitle: Before\n---\n\nBody.\n");

    let ran = run(project.path(), &["set", "a.md", r"title=a\*b", "--json"]);

    let out = ok_json(&ran);
    assert_eq!(out["document"]["fields"]["title"], json!("a*b"));
    let after = std::fs::read_to_string(project.path().join("a.md")).unwrap();
    assert!(after.contains("a*b"), "{after:?}");
    assert!(!after.contains(r"a\*b"), "{after:?}");
}

/// Unlike `--where` and `--if`, a written value gives `*` no wildcard meaning, so a bare one is an
/// error.
#[test]
fn a_bare_unescaped_star_in_a_set_value_is_refused_and_writes_nothing() {
    let project = Scratch::project(&NOTES);
    project.file("a.md", "---\ntitle: Before\n---\n\nBody.\n");
    let before = std::fs::read(project.path().join("a.md")).unwrap();

    let ran = run(project.path(), &["set", "a.md", "title=x*y", "--json"]);

    assert_eq!(ran.code, 1, "stdout: {} stderr: {}", ran.stdout, ran.stderr);
    let after = std::fs::read(project.path().join("a.md")).unwrap();
    assert_eq!(before, after, "a refused set must write nothing");
}

#[test]
fn a_backslash_comma_in_a_scalar_value_is_stored_as_a_literal_comma() {
    let project = Scratch::project(&NOTES);
    project.file("a.md", "---\ntitle: Before\n---\n\nBody.\n");

    let ran = run(project.path(), &["set", "a.md", r"title=a\,b", "--json"]);

    let out = ok_json(&ran);
    assert_eq!(out["document"]["fields"]["title"], json!("a,b"));
}

#[test]
fn a_list_field_splits_on_unescaped_commas_but_not_on_an_escaped_one() {
    let project = Scratch::project(&[
        (
            ".typdoc/collections/notes.json",
            r#"{ "match": "*.md", "schema": "note.json" }"#,
        ),
        (
            "note.json",
            r#"{ "name": "note", "fields": { "tags": { "type": "list" } } }"#,
        ),
        ("a.md", "---\ntags: [alpha]\n---\n\nBody.\n"),
    ]);

    let ran = run(project.path(), &["set", "a.md", r"tags=a\,b,c", "--json"]);

    let out = ok_json(&ran);
    assert_eq!(out["document"]["fields"]["tags"], json!(["a,b", "c"]));
}

#[test]
fn a_double_backslash_in_a_value_is_stored_as_a_single_backslash() {
    let project = Scratch::project(&NOTES);
    project.file("a.md", "---\ntitle: Before\n---\n\nBody.\n");

    let ran = run(project.path(), &["set", "a.md", r"title=a\\b", "--json"]);

    let out = ok_json(&ran);
    assert_eq!(out["document"]["fields"]["title"], json!(r"a\b"));
}

#[test]
fn an_unrecognized_escape_in_a_set_value_is_refused_and_writes_nothing() {
    let project = Scratch::project(&NOTES);
    project.file("a.md", "---\ntitle: Before\n---\n\nBody.\n");
    let before = std::fs::read(project.path().join("a.md")).unwrap();

    let ran = run(project.path(), &["set", "a.md", r"title=a\qb", "--json"]);

    assert_eq!(ran.code, 1, "stdout: {} stderr: {}", ran.stdout, ran.stderr);
    let error = ran.stderr_json();
    assert!(
        error["error"].as_str().unwrap_or_default().contains(r"\q"),
        "{error}"
    );
    let after = std::fs::read(project.path().join("a.md")).unwrap();
    assert_eq!(before, after, "a refused set must write nothing");
}

#[test]
fn a_value_ending_in_a_lone_backslash_is_refused_and_writes_nothing() {
    let project = Scratch::project(&NOTES);
    project.file("a.md", "---\ntitle: Before\n---\n\nBody.\n");
    let before = std::fs::read(project.path().join("a.md")).unwrap();

    let ran = run(project.path(), &["set", "a.md", r"title=a\", "--json"]);

    assert_eq!(ran.code, 1, "stdout: {} stderr: {}", ran.stdout, ran.stderr);
    let after = std::fs::read(project.path().join("a.md")).unwrap();
    assert_eq!(before, after, "a refused set must write nothing");
}

/// `--if` does not evaluate `ref.*` or `refby.*`, and refuses them rather than answer wrongly.
#[test]
fn an_if_with_a_ref_condition_is_refused_plainly() {
    let project = Scratch::project(&NOTES);
    project.file("a.md", "---\ntitle: A\n---\n\nBody.\n");

    let ran = run(
        project.path(),
        &[
            "set",
            "a.md",
            "title=B",
            "--if",
            "ref.any(x).path=y",
            "--json",
        ],
    );

    assert_eq!(ran.code, 1, "stdout: {} stderr: {}", ran.stdout, ran.stderr);
}

// --- `refs.acyclic` at write time ---

const WF_ACYCLIC_SCHEMA: &str = r#"{
  "name": "ticket",
  "code": "WF",
  "fields": {
    "title": { "type": "string" },
    "blocked_by": { "type": "ref[]", "target": "*", "acyclic": true }
  }
}"#;

const WF_ACYCLIC_COLLECTION: [(&str, &str); 2] = [
    (
        ".typdoc/collections/tickets.json",
        r#"{ "match": "tickets/{key}.md", "schema": "wf.json" }"#,
    ),
    ("wf.json", WF_ACYCLIC_SCHEMA),
];

#[test]
fn set_refuses_a_write_that_closes_an_immediate_cycle_on_an_acyclic_field() {
    let project = Scratch::project(&WF_ACYCLIC_COLLECTION);
    project.file(
        "tickets/WF-1.md",
        "---\ntitle: One\nblocked_by: [WF-5]\n---\n",
    );
    project.file("tickets/WF-5.md", "---\ntitle: Five\n---\n");
    let before = std::fs::read(project.path().join("tickets/WF-5.md")).unwrap();

    let ran = run(
        project.path(),
        &["set", "WF-5", "blocked_by=WF-1", "--json"],
    );

    assert_eq!(ran.code, 2, "stdout: {} stderr: {}", ran.stdout, ran.stderr);
    assert_eq!(ran.stdout, "");
    let error = ran.stderr_json();
    let details = error["details"].as_array().expect("a details array");
    assert!(
        details.iter().any(|f| f["rule"] == json!("refs.acyclic")),
        "{details:?}"
    );

    let after = std::fs::read(project.path().join("tickets/WF-5.md")).unwrap();
    assert_eq!(
        before, after,
        "a refused write must leave the document's bytes untouched"
    );
}

#[test]
fn set_refuses_a_write_that_closes_a_longer_chain_into_a_cycle() {
    let project = Scratch::project(&WF_ACYCLIC_COLLECTION);
    project.file(
        "tickets/WF-1.md",
        "---\ntitle: One\nblocked_by: [WF-2]\n---\n",
    );
    project.file(
        "tickets/WF-2.md",
        "---\ntitle: Two\nblocked_by: [WF-3]\n---\n",
    );
    project.file("tickets/WF-3.md", "---\ntitle: Three\n---\n");
    let before = std::fs::read(project.path().join("tickets/WF-3.md")).unwrap();

    let ran = run(
        project.path(),
        &["set", "WF-3", "blocked_by=WF-1", "--json"],
    );

    assert_eq!(ran.code, 2, "stdout: {} stderr: {}", ran.stdout, ran.stderr);
    let details = ran.stderr_json()["details"].as_array().unwrap().clone();
    assert!(
        details.iter().any(|f| f["rule"] == json!("refs.acyclic")),
        "{details:?}"
    );
    let after = std::fs::read(project.path().join("tickets/WF-3.md")).unwrap();
    assert_eq!(before, after);
}

/// The scan before a write still finds the unrelated cycle; it must not become this write's
/// refusal.
#[test]
fn set_is_not_refused_by_an_unrelated_pre_existing_cycle_elsewhere() {
    let project = Scratch::project(&WF_ACYCLIC_COLLECTION);
    project.file(
        "tickets/WF-10.md",
        "---\ntitle: Ten\nblocked_by: [WF-11]\n---\n",
    );
    project.file(
        "tickets/WF-11.md",
        "---\ntitle: Eleven\nblocked_by: [WF-10]\n---\n",
    );
    project.file("tickets/WF-1.md", "---\ntitle: One\n---\n");

    let ran = run(project.path(), &["set", "WF-1", "title=Updated", "--json"]);

    let out = ok_json(&ran);
    assert_eq!(out["document"]["fields"]["title"], json!("Updated"));
}

// --- A cycle refuses a write only when the write forms it ---

/// The cycle is written straight to disk, since `set` would refuse to create it. `validate` still
/// reports it after the write.
#[test]
fn set_untouched_by_its_own_acyclic_field_succeeds_despite_a_pre_existing_cycle() {
    let project = Scratch::project(&WF_ACYCLIC_COLLECTION);
    project.file(
        "tickets/WF-1.md",
        "---\ntitle: One\nblocked_by: [WF-5]\n---\n",
    );
    project.file(
        "tickets/WF-5.md",
        "---\ntitle: Five\nblocked_by: [WF-1]\n---\n",
    );

    let ran = run(project.path(), &["set", "WF-1", "title=Updated", "--json"]);

    let out = ok_json(&ran);
    assert_eq!(out["document"]["fields"]["title"], json!("Updated"));

    let validated = run(project.path(), &["validate", "--json"]);
    assert_eq!(validated.code, 2, "{}", validated.stderr);
    let findings = validated.stdout_json()["findings"]
        .as_array()
        .cloned()
        .unwrap();
    assert!(
        findings.iter().any(|f| f["rule"] == json!("refs.acyclic")),
        "{findings:?}"
    );
}

#[test]
fn set_on_an_unrelated_field_succeeds_when_no_cycle_exists_at_all() {
    let project = Scratch::project(&WF_ACYCLIC_COLLECTION);
    project.file(
        "tickets/WF-1.md",
        "---\ntitle: One\nblocked_by: [WF-5]\n---\n",
    );
    project.file("tickets/WF-5.md", "---\ntitle: Five\n---\n");

    let ran = run(project.path(), &["set", "WF-1", "title=Updated", "--json"]);

    let out = ok_json(&ran);
    assert_eq!(out["document"]["fields"]["title"], json!("Updated"));
}

/// `WF-1` and `WF-5` already form a cycle; the write forms a second one,
/// `WF-1 -> WF-5 -> WF-10 -> WF-1`, and the cycle already on disk does not excuse it.
#[test]
fn set_refuses_a_write_that_closes_a_new_cycle_through_a_different_pair_on_the_same_field() {
    let project = Scratch::project(&WF_ACYCLIC_COLLECTION);
    project.file(
        "tickets/WF-1.md",
        "---\ntitle: One\nblocked_by: [WF-5]\n---\n",
    );
    project.file(
        "tickets/WF-5.md",
        "---\ntitle: Five\nblocked_by: [WF-1, WF-10]\n---\n",
    );
    project.file("tickets/WF-10.md", "---\ntitle: Ten\n---\n");
    let before = std::fs::read(project.path().join("tickets/WF-10.md")).unwrap();

    let ran = run(
        project.path(),
        &["set", "WF-10", "blocked_by=WF-1", "--json"],
    );

    assert_eq!(ran.code, 2, "stdout: {} stderr: {}", ran.stdout, ran.stderr);
    let details = ran.stderr_json()["details"].as_array().unwrap().clone();
    assert!(
        details.iter().any(|f| f["rule"] == json!("refs.acyclic")),
        "{details:?}"
    );
    let after = std::fs::read(project.path().join("tickets/WF-10.md")).unwrap();
    assert_eq!(before, after);
}
