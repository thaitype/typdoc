//! Frontmatter values read by the type their schema gives them, and the config errors that
//! need a schema. The expected values are written by hand from the design.

#[allow(dead_code, reason = "each test file uses part of the shared helper")]
mod common;

use common::{Ran, Scratch, Spawn, fixture};
use serde_json::{Value, json};

fn get(project: &std::path::Path, path: &str) -> Ran {
    Spawn::args(["get", path, "--json"]).cwd(project).run()
}

/// The `fields` of a document of the fixture `valid/field-types`.
fn fields_of(path: &str) -> Value {
    let ran = get(&fixture("valid/field-types"), path);
    assert_eq!(ran.code, 0, "{path}: {}", ran.stderr);
    ran.stdout_json()["document"]["fields"].clone()
}

#[test]
fn each_field_type_gives_its_value_the_type_the_schema_names() {
    assert_eq!(
        fields_of("records/typed.md"),
        json!({
            "title": "Typed",
            "count": 3,
            "ratio": 1000.0,
            "done": true,
            "due": "2026-09-19",
            "at": "2026-09-19T14:30:00+07:00",
            "kind": "b",
            "tags": ["x", "y"],
            "parent": "WF-1",
            "blockers": ["WF-1", "WF-2"]
        })
    );
}

#[test]
fn a_number_is_a_number_in_json_and_not_text() {
    let fields = fields_of("records/typed.md");

    assert!(fields["count"].is_i64() || fields["count"].is_u64());
    assert!(fields["ratio"].is_f64());
}

#[test]
fn the_words_no_yes_on_and_off_and_a_version_stay_text_in_a_string_field() {
    assert_eq!(
        fields_of("records/words.md"),
        json!({
            "title": "no",
            "note": "yes",
            "owner": "on",
            "label": "off",
            "version": "1.10"
        })
    );
}

#[test]
fn the_schema_decides_and_quotes_in_the_file_do_not() {
    assert_eq!(
        fields_of("records/quoted.md"),
        json!({ "title": "Quoted", "count": 3, "done": false, "due": "2026-09-19" })
    );
}

#[test]
fn a_value_that_does_not_fit_its_type_is_shown_as_written_and_never_guessed() {
    assert_eq!(
        fields_of("records/misfit.md"),
        json!({
            "title": ["a", "b"],
            "count": "abc",
            "ratio": "0755",
            "done": "no",
            "due": "2026-02-30",
            "at": "2026-09-19",
            "kind": "z",
            "tags": "single",
            "parent": ["WF-1"],
            "blockers": "WF-1"
        })
    );
}

#[test]
fn a_field_the_schema_does_not_name_keeps_the_text_as_written() {
    assert_eq!(
        fields_of("records/unknown.md"),
        json!({
            "title": "Unknown",
            "extra": "1e3",
            "flag": "true",
            "year": "2026",
            "empty": "",
            "nothing": "~",
            "items": ["1", "2.50"]
        })
    );
}

#[test]
fn the_types_of_a_parent_schema_apply_and_a_child_field_replaces_the_parent_one() {
    assert_eq!(
        fields_of("tickets/WF-1.md"),
        json!({
            "status": "open",
            "estimate": 5,
            "weight": "1e3",
            "created_at": "2026-09-19T14:30:00+07:00",
            "blocked_by": ["WF-2"]
        })
    );
    assert_eq!(
        fields_of("tickets/WF-2.md"),
        json!({ "status": "resolved", "estimate": 0.5, "weight": "7" })
    );
}

fn bool_project(value: &str) -> Scratch {
    let project = Scratch::project(&[]);
    project.file(
        ".typdoc/collections/notes.json",
        r#"{ "match": "*.md", "schema": "note.json" }"#,
    );
    project.file(
        "note.json",
        r#"{ "name": "note", "fields": { "done": { "type": "bool" } } }"#,
    );
    project.file("a.md", &format!("---\ndone: {value}\n---\n"));
    project
}

#[test]
fn a_bool_is_true_or_false_and_no_other_spelling_is_read_as_one() {
    for (written, read) in [
        ("true", json!(true)),
        ("false", json!(false)),
        ("yes", json!("yes")),
        ("no", json!("no")),
        ("on", json!("on")),
        ("off", json!("off")),
        ("y", json!("y")),
        ("True", json!("True")),
        ("FALSE", json!("FALSE")),
        ("1", json!("1")),
        ("0", json!("0")),
    ] {
        let ran = get(bool_project(written).path(), "a.md");

        assert_eq!(ran.code, 0, "{written}: {}", ran.stderr);
        assert_eq!(
            ran.stdout_json()["document"]["fields"]["done"],
            read,
            "{written}"
        );
    }
}

fn note_project(schema: &str, document: &str) -> Scratch {
    let project = Scratch::project(&[]);
    project.file(
        ".typdoc/collections/notes.json",
        r#"{ "match": "*.md", "schema": "note.json" }"#,
    );
    project.file("note.json", schema);
    project.file("a.md", document);
    project
}

#[test]
fn a_schema_type_the_format_does_not_name_reads_its_value_as_written() {
    let project = note_project(
        r#"{ "name": "note", "fields": { "n": { "type": "integer" } } }"#,
        "---\nn: 3\n---\n",
    );

    let ran = get(project.path(), "a.md");

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    assert_eq!(ran.stdout_json()["document"]["fields"]["n"], json!("3"));
}

#[test]
fn a_value_with_no_text_form_exits_2_and_names_the_field() {
    for (field, document) in [
        ("meta", "---\nmeta: { a: 1 }\n---\n"),
        ("tags", "---\ntags: [[a], b]\n---\n"),
        ("tags", "---\ntags: [{ a: 1 }]\n---\n"),
    ] {
        let project = note_project(
            r#"{ "name": "note", "fields": { "tags": { "type": "list" } } }"#,
            document,
        );

        let ran = get(project.path(), "a.md");

        assert_eq!(ran.code, 2, "{document:?}: {}", ran.stderr);
        let object = ran.stderr_json();
        assert!(
            object["error"].as_str().unwrap().contains(field),
            "{document:?}: {object}"
        );
    }
}

#[test]
fn a_document_with_no_block_has_no_fields_under_a_schema_with_types() {
    let project = note_project(
        r#"{ "name": "note", "fields": { "n": { "type": "number", "required": true } } }"#,
        "# Only a heading\n",
    );

    let ran = get(project.path(), "a.md");

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    assert_eq!(ran.stdout_json()["document"]["fields"], json!({}));
}

/// `(rule, path)` of each detail of the error object of a run that ended with 2.
fn details(ran: &Ran) -> Vec<(String, String)> {
    assert_eq!(ran.code, 2, "stderr: {}", ran.stderr);
    assert_eq!(ran.stdout, "");
    let object = ran.stderr_json();
    assert_eq!(object["complete"], json!(true), "{object}");
    object["details"]
        .as_array()
        .expect("details")
        .iter()
        .map(|d| {
            assert_eq!(d["level"], json!("error"), "{d}");
            assert!(d["message"].as_str().is_some_and(|m| !m.is_empty()), "{d}");
            (
                d["rule"].as_str().expect("rule").to_owned(),
                d["path"].as_str().expect("path").to_owned(),
            )
        })
        .collect()
}

fn pair(rule: &str, path: &str) -> (String, String) {
    (rule.to_owned(), path.to_owned())
}

fn collection(project: &Scratch, name: &str, pattern: &str, schema: &str) {
    project.file(
        &format!(".typdoc/collections/{name}.json"),
        &json!({ "match": pattern, "schema": schema }).to_string(),
    );
}

#[test]
fn a_collection_whose_schema_does_not_exist_is_config_collection_schema() {
    let project = Scratch::project(&[]);
    collection(&project, "notes", "*.md", "absent.json");

    let ran = get(project.path(), "a.md");

    assert_eq!(
        details(&ran),
        [pair(
            "config.collection-schema",
            ".typdoc/collections/notes.json"
        )]
    );
    assert!(ran.stderr.contains("absent.json"), "{}", ran.stderr);
}

#[test]
fn a_parent_schema_that_does_not_exist_is_config_collection_schema_and_names_the_parent() {
    let project = Scratch::project(&[]);
    collection(&project, "notes", "*.md", "note.json");
    project.file(
        "note.json",
        r#"{ "name": "note", "extends": "./gone.json", "fields": {} }"#,
    );

    let ran = get(project.path(), "a.md");

    assert_eq!(
        details(&ran),
        [pair(
            "config.collection-schema",
            ".typdoc/collections/notes.json"
        )]
    );
    assert!(ran.stderr.contains("gone.json"), "{}", ran.stderr);
}

#[test]
fn a_parent_is_found_from_the_folder_of_the_schema_that_extends_it() {
    let project = Scratch::project(&[]);
    collection(&project, "notes", "*.md", "schemas/note.json");
    project.file(
        "schemas/note.json",
        r#"{ "name": "note", "extends": "../base/base.json", "fields": {} }"#,
    );
    project.file(
        "base/base.json",
        r#"{ "name": "base", "fields": { "n": { "type": "number" } } }"#,
    );
    project.file("a.md", "---\nn: 4\n---\n");

    let ran = get(project.path(), "a.md");

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    assert_eq!(ran.stdout_json()["document"]["fields"]["n"], json!(4));
}

#[test]
fn schemas_that_extend_each_other_are_read_once_and_do_not_loop() {
    let project = Scratch::project(&[]);
    collection(&project, "notes", "*.md", "a.json");
    project.file(
        "a.json",
        r#"{ "name": "a", "extends": "b.json", "fields": { "n": { "type": "number" } } }"#,
    );
    project.file(
        "b.json",
        r#"{ "name": "b", "extends": "a.json", "fields": { "m": { "type": "number" } } }"#,
    );
    project.file("a.md", "---\nn: 1\nm: 2\n---\n");

    let ran = get(project.path(), "a.md");

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    assert_eq!(
        ran.stdout_json()["document"]["fields"],
        json!({ "n": 1, "m": 2 })
    );
}

#[test]
fn a_schema_url_with_a_scheme_other_than_http_or_https_is_config_schema_url() {
    for url in [
        "ftp://example.invalid/note.json",
        "file:///etc/note.json",
        "HTTPS2://example.invalid/note.json",
    ] {
        let project = Scratch::project(&[]);
        collection(&project, "notes", "*.md", url);

        let ran = get(project.path(), "a.md");

        assert_eq!(
            details(&ran),
            [pair("config.schema-url", ".typdoc/collections/notes.json")],
            "{url}"
        );
        assert!(ran.stderr.contains(url), "{url}: {}", ran.stderr);
    }
}

#[test]
fn an_extends_url_with_another_scheme_is_config_schema_url_at_the_schema_file() {
    let project = Scratch::project(&[]);
    collection(&project, "notes", "*.md", "schemas/note.json");
    project.file(
        "schemas/note.json",
        r#"{ "name": "note", "extends": "ftp://example.invalid/base.json", "fields": {} }"#,
    );

    let ran = get(project.path(), "a.md");

    assert_eq!(
        details(&ran),
        [pair("config.schema-url", "schemas/note.json")]
    );
}

#[test]
fn an_http_or_https_url_is_not_fetched_and_still_stops_the_run() {
    for url in [
        "http://example.invalid/n.json",
        "https://example.invalid/n.json",
    ] {
        let project = Scratch::project(&[]);
        collection(&project, "notes", "*.md", url);

        let ran = get(project.path(), "a.md");

        assert_eq!(ran.code, 2, "{url}: {}", ran.stderr);
        assert!(ran.stderr.contains(url), "{url}: {}", ran.stderr);
    }
}

#[test]
fn a_match_template_that_breaks_the_placeholder_rules_is_config_match_template() {
    for pattern in [
        "",
        "/a.md",
        "a//b.md",
        "../a.md",
        "a**b.md",
        "{other}.md",
        "a{b.md",
        "{key}.md",
        "{key}/{key}.md",
    ] {
        let project = Scratch::project(&[]);
        collection(&project, "notes", pattern, "note.json");
        project.file("note.json", r#"{ "name": "note", "fields": {} }"#);

        let ran = get(project.path(), "a.md");

        assert_eq!(
            details(&ran),
            [pair(
                "config.match-template",
                ".typdoc/collections/notes.json"
            )],
            "{pattern:?}"
        );
    }
}

#[test]
fn a_coded_schema_with_a_wildcard_or_without_one_key_is_config_match_template() {
    for pattern in [
        "*.md",
        "**/{key}.md",
        "WF-1.md",
        "{key}/{key}.md",
        "{key}*.md",
    ] {
        let project = Scratch::project(&[]);
        collection(&project, "tickets", pattern, "ticket.json");
        project.file(
            "ticket.json",
            r#"{ "name": "ticket", "code": "WF", "fields": {} }"#,
        );

        let ran = get(project.path(), "WF-1.md");

        assert_eq!(
            details(&ran),
            [pair(
                "config.match-template",
                ".typdoc/collections/tickets.json"
            )],
            "{pattern:?}"
        );
    }
}

#[test]
fn two_collections_that_name_the_same_coded_schema_are_config_coded_schema_shared() {
    for second in ["ticket.json", "./ticket.json", "sub/../ticket.json"] {
        let project = Scratch::project(&[]);
        collection(&project, "drafts", "drafts/{key}.md", second);
        collection(&project, "tickets", "tickets/{key}.md", "ticket.json");
        project.file(
            "ticket.json",
            r#"{ "name": "ticket", "code": "WF", "fields": {} }"#,
        );

        let ran = get(project.path(), "tickets/WF-1.md");

        assert_eq!(
            details(&ran),
            [pair(
                "config.coded-schema-shared",
                ".typdoc/collections/tickets.json"
            )],
            "{second}"
        );
    }
}

#[test]
fn several_collections_that_share_an_uncoded_schema_are_fine() {
    let project = Scratch::project(&[]);
    collection(&project, "notes", "notes/*.md", "note.json");
    collection(&project, "drafts", "drafts/*.md", "note.json");
    project.file("note.json", r#"{ "name": "note", "fields": {} }"#);
    project.file("notes/a.md", "");

    let ran = get(project.path(), "notes/a.md");

    assert_eq!(ran.code, 0, "{}", ran.stderr);
}

#[test]
fn every_config_error_that_can_be_determined_is_reported_together() {
    let project = Scratch::project(&[]);
    collection(&project, "absent", "*.md", "absent.json");
    collection(&project, "badmatch", "{key}.md", "note.json");
    collection(&project, "url", "u/*.md", "ftp://example.invalid/n.json");
    collection(&project, "one", "one/{key}.md", "ticket.json");
    collection(&project, "two", "two/{key}.md", "ticket.json");
    project.file("note.json", r#"{ "name": "note", "fields": {} }"#);
    project.file(
        "ticket.json",
        r#"{ "name": "ticket", "code": "WF", "fields": {} }"#,
    );

    let ran = get(project.path(), "a.md");

    assert_eq!(
        details(&ran),
        [
            pair(
                "config.collection-schema",
                ".typdoc/collections/absent.json"
            ),
            pair("config.match-template", ".typdoc/collections/badmatch.json"),
            pair("config.coded-schema-shared", ".typdoc/collections/two.json"),
            pair("config.schema-url", ".typdoc/collections/url.json"),
        ]
    );
}

#[test]
fn a_config_error_from_a_schema_and_one_from_the_config_come_in_one_list() {
    let project = Scratch::project(&[]);
    project.file(".typdoc/config.json", r#"{ "version": 1, "name": "x" }"#);
    collection(&project, "notes", "*.md", "absent.json");

    let ran = get(project.path(), "a.md");

    assert_eq!(
        details(&ran),
        [
            pair("config.collection-schema", ".typdoc/collections/notes.json"),
            pair("config.unknown-key", ".typdoc/config.json"),
        ]
    );
}

#[test]
fn a_fault_with_no_id_does_not_hide_the_config_errors_that_have_one() {
    let project = Scratch::project(&[]);
    project.file(".typdoc/config.json", r#"{ "version": 1, "name": "x" }"#);
    collection(&project, "notes", "*.md", "https://example.invalid/n.json");

    let ran = get(project.path(), "a.md");

    assert_eq!(
        details(&ran),
        [pair("config.unknown-key", ".typdoc/config.json")]
    );
}
