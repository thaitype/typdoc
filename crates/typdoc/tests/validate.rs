//! `validate`'s report, its scope, the merge of rule levels, and the three rules this ticket
//! builds: `frontmatter.parse`, `frontmatter.types` and `frontmatter.unknown`. The exact set
//! each rule's `broken/` fixture trips is checked by `coverage.rs`; this file covers the report
//! shape, scope, level merging and the arguments that choose them.

#[allow(dead_code, reason = "each test file uses part of the shared helper")]
mod common;

use common::{Ran, Scratch, Spawn, fixture};
use serde_json::{Value, json};

fn validate(args: &[&str], cwd: &std::path::Path) -> Ran {
    let mut all = vec!["validate"];
    all.extend_from_slice(args);
    all.push("--json");
    Spawn::args(all).cwd(cwd).run()
}

/// A project with one collection of `*.md` and a schema whose `title` is required, so a
/// missing or misfit field is easy to produce on purpose.
const REQUIRED_TITLE: [(&str, &str); 2] = [
    (
        ".typdoc/collections/notes.json",
        r#"{ "match": "*.md", "schema": "note.json" }"#,
    ),
    (
        "note.json",
        r#"{ "name": "note", "fields": { "title": { "type": "string", "required": true } } }"#,
    ),
];

#[test]
fn with_no_arguments_the_scope_is_the_whole_project_and_a_clean_project_has_no_findings() {
    let project = fixture("valid/several-namespaces");

    let ran = validate(&[], &project);

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    let object = ran.stdout_json();
    assert_eq!(object["summary"]["scope"], json!("all"));
    assert_eq!(object["summary"]["strict"], json!(false));
    assert_eq!(
        object["summary"]["checked"]["namespaces"],
        json!(["archive", "story-1", "story-2"])
    );
    assert_eq!(object["summary"]["checked"]["documents"], json!(6));
    assert!(
        object["summary"]["checked"].get("paths").is_none(),
        "scope all has no checked.paths: {object}"
    );
    assert_eq!(
        object["summary"]["findings"],
        json!({ "error": 0, "warn": 0, "info": 0 })
    );
    assert_eq!(object["findings"], json!([]));
}

#[test]
fn a_namespace_flag_narrows_scope_all_to_the_documents_of_that_namespace() {
    let project = fixture("valid/several-namespaces");

    let ran = validate(&["--namespace", "story-1"], &project);

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    let object = ran.stdout_json();
    assert_eq!(
        object["summary"]["checked"]["namespaces"],
        json!(["story-1"])
    );
    assert_eq!(object["summary"]["checked"]["documents"], json!(2));
}

#[test]
fn several_document_arguments_give_scope_paths_and_checked_paths_matches_the_findings() {
    let project = fixture("valid/several-namespaces");

    let ran = validate(&["story-1:WF-1", "story-2:WF-9"], &project);

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    let object = ran.stdout_json();
    assert_eq!(object["summary"]["scope"], json!("paths"));
    assert_eq!(
        object["summary"]["checked"]["paths"],
        json!(["story-1/tickets/WF-1.md", "story-2/tickets/WF-9.md"])
    );
    assert_eq!(object["summary"]["checked"]["documents"], json!(2));
    assert_eq!(
        object["summary"]["checked"]["namespaces"],
        json!(["story-1", "story-2"])
    );
    let paths: Vec<&str> = object["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["path"].as_str().unwrap())
        .collect();
    let checked: Vec<&str> = object["summary"]["checked"]["paths"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p.as_str().unwrap())
        .collect();
    assert!(
        paths.iter().all(|p| checked.contains(p)),
        "every finding's path is one of checked.paths: {object}"
    );
}

#[test]
fn the_same_document_named_twice_is_checked_once() {
    let project = fixture("valid/several-namespaces");

    let ran = validate(&["story-1:WF-1", "story-1/tickets/WF-1.md"], &project);

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    let object = ran.stdout_json();
    assert_eq!(object["summary"]["checked"]["documents"], json!(1));
    assert_eq!(
        object["summary"]["checked"]["paths"],
        json!(["story-1/tickets/WF-1.md"])
    );
}

#[test]
fn schemas_or_audit_together_with_arguments_is_bad_arguments() {
    let project = fixture("valid/several-namespaces");

    for args in [
        vec!["--schemas", "story-1:WF-1"],
        vec!["--audit", "story-1:WF-1"],
    ] {
        let ran = validate(&args, &project);

        assert_eq!(ran.code, 1, "{args:?}: {}", ran.stdout);
        assert_eq!(ran.stdout, "", "{args:?}");
        let object = ran.stderr_json();
        assert_eq!(object["code"], json!(1));
    }
}

/// Audit mode's own report (`summary.audit`, `summary.unreported`, the `audit` object, rules
/// at `off` shown as `info`) is not built yet (ticket 19); `--audit` alone is refused rather
/// than silently answering a different question than the one it names, the same choice already
/// made for `--json`'s absence and for a `project::` prefix.
#[test]
fn audit_alone_is_not_built_yet() {
    let project = fixture("valid/several-namespaces");

    let ran = validate(&["--audit"], &project);

    assert_eq!(ran.code, 1, "{}", ran.stdout);
    assert_eq!(ran.stdout, "");
    let object = ran.stderr_json();
    assert_eq!(object["code"], json!(1));
    assert!(
        object["error"].as_str().unwrap().contains("--audit"),
        "{object}"
    );
}

#[test]
fn schemas_alone_describes_the_whole_project_and_checks_no_document_yet() {
    let project = fixture("valid/several-namespaces");

    let ran = validate(&["--schemas"], &project);

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    let object = ran.stdout_json();
    assert_eq!(object["summary"]["scope"], json!("schemas"));
    assert_eq!(object["summary"]["checked"]["documents"], json!(0));
    assert_eq!(object["findings"], json!([]));
}

#[test]
fn an_argument_that_names_no_document_stops_before_any_report_and_is_not_a_finding() {
    let project = fixture("valid/several-namespaces");

    let ran = validate(&["story-1:NOSUCH-1"], &project);

    assert_eq!(ran.code, 5, "{}", ran.stderr);
    assert_eq!(ran.stdout, "");
    let object = ran.stderr_json();
    assert!(object.get("summary").is_none(), "{object}");
}

#[test]
fn validate_without_json_exits_1_as_not_built_yet() {
    let project = fixture("valid/minimal");

    let ran = Spawn::args(["validate"]).cwd(&project).run();

    assert_eq!(ran.code, 1);
}

fn missing_title(project: &Scratch) -> Ran {
    validate(&[], project.path())
}

#[test]
fn an_empty_block_and_a_file_with_no_block_both_reach_frontmatter_types() {
    let project = Scratch::project(&REQUIRED_TITLE);
    project.file("empty.md", "---\n---\n\nBody.\n");
    project.file("none.md", "Just a plain file, no frontmatter at all.\n");

    let ran = missing_title(&project);

    assert_eq!(ran.code, 2, "{}", ran.stderr);
    let object = ran.stdout_json();
    let findings = object["findings"].as_array().unwrap();
    assert_eq!(findings.len(), 2, "{object}");
    for finding in findings {
        assert_eq!(finding["rule"], json!("frontmatter.types"));
        assert_eq!(finding["level"], json!("error"));
        assert_eq!(finding["field"], json!("title"));
        assert!(finding.get("line").is_none(), "{finding}");
        assert!(finding.get("col").is_none(), "{finding}");
    }
}

#[test]
fn a_value_that_does_not_fit_its_type_is_frontmatter_types_at_error() {
    let files = [
        (
            ".typdoc/collections/notes.json",
            r#"{ "match": "*.md", "schema": "note.json" }"#,
        ),
        (
            "note.json",
            r#"{ "name": "note", "fields": { "count": { "type": "number" } } }"#,
        ),
    ];
    let project = Scratch::project(&files);
    project.file("a.md", "---\ncount: abc\n---\n");

    let ran = missing_title(&project);

    assert_eq!(ran.code, 2, "{}", ran.stderr);
    let object = ran.stdout_json();
    assert_eq!(object["findings"].as_array().unwrap().len(), 1);
    assert_eq!(object["findings"][0]["rule"], json!("frontmatter.types"));
    assert_eq!(object["findings"][0]["field"], json!("count"));
}

#[test]
fn a_field_the_schema_does_not_name_is_frontmatter_unknown_at_the_default_warn_level() {
    let project = Scratch::project(&REQUIRED_TITLE);
    project.file("a.md", "---\ntitle: ok\nextra: surprise\n---\n");

    let ran = missing_title(&project);

    assert_eq!(ran.code, 0, "warn does not fail the run: {}", ran.stderr);
    let object = ran.stdout_json();
    assert_eq!(object["findings"].as_array().unwrap().len(), 1);
    assert_eq!(object["findings"][0]["rule"], json!("frontmatter.unknown"));
    assert_eq!(object["findings"][0]["level"], json!("warn"));
}

#[test]
fn strict_raises_a_remaining_warn_to_error_and_the_exit_code_follows_it() {
    let project = Scratch::project(&REQUIRED_TITLE);
    project.file("a.md", "---\ntitle: ok\nextra: surprise\n---\n");

    let plain = missing_title(&project);
    let strict = validate(&["--strict"], project.path());

    assert_eq!(plain.code, 0, "{}", plain.stderr);
    assert_eq!(strict.code, 2, "{}", strict.stderr);
    let object = strict.stdout_json();
    assert_eq!(object["summary"]["strict"], json!(true));
    assert_eq!(object["findings"][0]["level"], json!("error"));
    assert_eq!(
        object["summary"]["findings"],
        json!({ "error": 1, "warn": 0, "info": 0 })
    );
}

#[test]
fn a_project_wide_setting_is_overridden_by_the_collections_own_setting() {
    let files = [
        (
            ".typdoc/collections/notes.json",
            r#"{ "match": "*.md", "schema": "note.json",
                "validation": { "frontmatter.unknown": { "level": "off" } } }"#,
        ),
        ("note.json", r#"{ "name": "note", "fields": {} }"#),
    ];
    let project = Scratch::project(&files);
    project.file(
        ".typdoc/config.json",
        r#"{ "version": 1, "validation": { "global": {
            "frontmatter.unknown": { "level": "error" } } } }"#,
    );
    project.file("a.md", "---\nextra: surprise\n---\n");

    let ran = missing_title(&project);

    assert_eq!(ran.code, 0, "the collection turns it off: {}", ran.stderr);
    assert_eq!(ran.stdout_json()["findings"], json!([]));
}

#[test]
fn findings_across_several_files_are_ordered_by_path() {
    let project = Scratch::project(&REQUIRED_TITLE);
    project.file("b.md", "---\nother: 1\n---\n");
    project.file("a.md", "---\nother: 1\n---\n");

    let ran = missing_title(&project);

    assert_eq!(ran.code, 2, "{}", ran.stderr);
    let object = ran.stdout_json();
    let paths: Vec<&str> = object["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["path"].as_str().unwrap())
        .collect();
    // Each file is missing `title` (frontmatter.types, error) and has an unknown field
    // `other` (frontmatter.unknown, warn): two findings per file, `a.md` before `b.md`.
    assert_eq!(paths, ["a.md", "a.md", "b.md", "b.md"]);
}

#[test]
fn an_on_disk_argument_after_the_first_is_read_against_the_root_the_first_argument_found() {
    let project = Scratch::project(&REQUIRED_TITLE);
    project.file("a.md", "---\ntitle: A\n---\n");
    project.file("b.md", "---\ntitle: B\n---\n");

    // The first argument is a project-relative path (found the usual way, via `TYPDOC_DIR`
    // and the current directory); the second is on disk (`./b.md`) and must be read against
    // that same root rather than walking up from its own folder again.
    let ran = Spawn::args(["validate", "a.md", "./b.md", "--json"])
        .cwd(project.path())
        .run();

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    let object = ran.stdout_json();
    assert_eq!(
        object["summary"]["checked"]["paths"],
        json!(["a.md", "b.md"])
    );
}

fn broken(rule: &str) -> Value {
    let dir = fixture("broken").join(rule);
    let ran = Spawn::args(["validate", "--json"]).cwd(&dir).run();
    ran.stdout_json()
}

#[test]
fn the_frontmatter_parse_fixture_reports_no_position() {
    let object = broken("frontmatter.parse");

    let findings = object["findings"].as_array().unwrap();
    // The fixture holds one document with invalid YAML in a closed block and one whose block
    // holds a second YAML document: every one of them is `frontmatter.parse` with no position.
    assert_eq!(findings.len(), 2, "{findings:?}");
    for finding in findings {
        assert_eq!(finding["rule"], json!("frontmatter.parse"));
        assert_eq!(finding["level"], json!("error"));
        assert!(finding.get("line").is_none(), "{finding}");
        assert!(finding.get("col").is_none(), "{finding}");
        assert!(finding.get("field").is_none(), "{finding}");
    }
}
