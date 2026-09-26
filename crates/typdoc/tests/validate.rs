//! Covers SPC-1, SPC-2, SPC-4, SPC-5, SPC-7, SPC-10, SPC-12, SPC-14, SPC-15, SPC-17.
//!
//! The exact set each rule's `broken/` fixture trips is checked by `coverage.rs`.

#[allow(dead_code, reason = "each test file uses part of the shared helper")]
mod common;

use common::{Ran, Scratch, Spawn, fixture};
use serde_json::{Value, json};
use typdoc_core::{TEMP_PREFIX, is_temp_name};

fn validate(args: &[&str], cwd: &std::path::Path) -> Ran {
    let mut all = vec!["validate"];
    all.extend_from_slice(args);
    all.push("--json");
    Spawn::args(all).cwd(cwd).run()
}

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

/// Without this refusal, `--schemas` would answer alone and drop `--audit`'s report without a
/// word.
#[test]
fn schemas_and_audit_together_is_bad_arguments_even_with_no_document_arguments() {
    let project = fixture("valid/several-namespaces");

    let ran = validate(&["--schemas", "--audit"], &project);

    assert_eq!(ran.code, 1, "{}", ran.stdout);
    assert_eq!(ran.stdout, "");
    let object = ran.stderr_json();
    assert_eq!(object["code"], json!(1));
    assert!(
        object["error"].as_str().unwrap().contains("--schemas")
            && object["error"].as_str().unwrap().contains("--audit"),
        "{object}"
    );
}

#[test]
fn audit_alone_builds_a_report_with_summary_audit_and_unreported() {
    let project = fixture("valid/several-namespaces");

    let ran = validate(&["--audit"], &project);

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    let object = ran.stdout_json();
    assert_eq!(object["summary"]["audit"], json!(true));
    assert!(object["summary"]["unreported"].is_object(), "{object}");
    assert!(object["summary"]["overlapping"].is_u64(), "{object}");
    assert!(object["audit"]["collections"].is_array(), "{object}");
    assert!(object["audit"]["uncollected"].is_array(), "{object}");
    assert!(object["audit"]["no_frontmatter"].is_array(), "{object}");
    assert!(object["audit"]["overlapping"].is_array(), "{object}");
}

const AUDIT_PROJECT: [(&str, &str); 2] = [
    (
        ".typdoc/collections/notes.json",
        r#"{ "match": "notes/*.md", "schema": "note.json",
            "validation": { "frontmatter.unknown": { "level": "off" } } }"#,
    ),
    (
        "note.json",
        r#"{ "name": "note", "fields": { "title": { "type": "string", "required": true } } }"#,
    ),
];

fn audit_project() -> Scratch {
    let project = Scratch::project(&AUDIT_PROJECT);
    project.file("notes/a.md", "---\nextra: x\n---\n");
    project.file("notes/b.md", "no frontmatter here\n");
    project.file("README.md", "");
    project
}

#[test]
fn audit_reports_an_off_rule_as_info_and_still_exits_0_despite_an_error_finding() {
    let project = audit_project();

    let ran = validate(&["--audit"], project.path());

    assert_eq!(
        ran.code, 0,
        "an error-level finding still exits 0 under --audit: {}",
        ran.stderr
    );
    let object = ran.stdout_json();
    let findings = object["findings"].as_array().unwrap();
    let unknown = findings
        .iter()
        .find(|f| f["rule"] == json!("frontmatter.unknown"))
        .unwrap_or_else(|| panic!("no frontmatter.unknown finding: {findings:?}"));
    assert_eq!(unknown["level"], json!("info"), "{unknown}");
    let types = findings
        .iter()
        .find(|f| f["rule"] == json!("frontmatter.types"))
        .unwrap_or_else(|| panic!("no frontmatter.types finding: {findings:?}"));
    assert_eq!(types["level"], json!("error"), "{types}");
    assert_eq!(
        object["summary"]["findings"],
        json!({ "error": 1, "warn": 0, "info": 1 })
    );
}

/// A rule turned `off` has no `warn` for `--strict` to raise, so it stays `info`.
#[test]
fn strict_still_raises_warn_to_error_under_audit_while_an_off_rule_stays_info() {
    let project = audit_project();
    project.file(
        ".typdoc/collections/plain.json",
        r#"{ "match": "plain/*.md", "schema": "plain.json" }"#,
    );
    project.file("plain.json", r#"{ "name": "plain", "fields": {} }"#);
    project.file("plain/c.md", "---\nextra: y\n---\n");

    let ran = validate(&["--audit", "--strict"], project.path());

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    let object = ran.stdout_json();
    assert_eq!(object["summary"]["strict"], json!(true));
    let findings = object["findings"].as_array().unwrap();
    let raised = findings
        .iter()
        .find(|f| f["path"] == json!("plain/c.md") && f["rule"] == json!("frontmatter.unknown"))
        .unwrap_or_else(|| panic!("no finding for plain/c.md: {findings:?}"));
    assert_eq!(
        raised["level"],
        json!("error"),
        "--strict still raises warn to error under --audit: {raised}"
    );
    let off_stayed = findings
        .iter()
        .find(|f| f["path"] == json!("notes/a.md") && f["rule"] == json!("frontmatter.unknown"))
        .unwrap_or_else(|| panic!("no finding for notes/a.md: {findings:?}"));
    assert_eq!(
        off_stayed["level"],
        json!("info"),
        "off has no warn for --strict to raise: {off_stayed}"
    );
}

#[test]
fn audit_lists_uncollected_and_no_frontmatter_and_evaluates_neither() {
    let project = audit_project();

    let ran = validate(&["--audit"], project.path());

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    let object = ran.stdout_json();
    assert_eq!(object["audit"]["uncollected"], json!(["README.md"]));
    assert_eq!(object["audit"]["no_frontmatter"], json!(["notes/b.md"]));
    assert_eq!(
        object["summary"]["unreported"],
        json!({ "uncollected": 1, "no_frontmatter": 1 })
    );
    assert_eq!(object["summary"]["checked"]["documents"], json!(1));
    let findings = object["findings"].as_array().unwrap();
    assert!(
        findings.iter().all(|f| f["path"] != json!("notes/b.md")),
        "{findings:?}"
    );
    assert!(
        findings.iter().all(|f| f["path"] != json!("README.md")),
        "{findings:?}"
    );
}

/// `notes` holds `b.md` though it is not evaluated: a count `findings` alone could not give.
#[test]
fn audit_counts_a_no_frontmatter_document_as_one_the_collection_holds() {
    let project = audit_project();

    let ran = validate(&["--audit"], project.path());

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    let object = ran.stdout_json();
    assert_eq!(
        object["audit"]["collections"],
        json!([{ "name": "notes", "documents": 2 }])
    );
}

#[test]
fn validate_and_audit_treat_a_file_with_no_frontmatter_differently_and_the_lists_never_overlap() {
    let project = audit_project();

    let plain = validate(&[], project.path());
    let audit = validate(&["--audit"], project.path());

    assert_eq!(plain.code, 2, "{}", plain.stderr);
    let plain_findings = plain.stdout_json()["findings"].clone();
    let plain_findings = plain_findings.as_array().unwrap();
    assert!(
        plain_findings
            .iter()
            .any(|f| f["path"] == json!("notes/b.md") && f["rule"] == json!("frontmatter.types")),
        "plain validate evaluates a file with no frontmatter like any document: {plain_findings:?}"
    );

    assert_eq!(audit.code, 0, "{}", audit.stderr);
    let object = audit.stdout_json();
    let uncollected: Vec<&str> = object["audit"]["uncollected"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    let no_frontmatter: Vec<&str> = object["audit"]["no_frontmatter"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert!(no_frontmatter.contains(&"notes/b.md"), "{no_frontmatter:?}");
    assert!(uncollected.contains(&"README.md"), "{uncollected:?}");
    assert!(!uncollected.contains(&"notes/b.md"), "{uncollected:?}");
    assert!(!no_frontmatter.contains(&"README.md"), "{no_frontmatter:?}");
}

#[test]
fn the_text_form_of_audit_prints_the_summary_and_the_two_lists() {
    let project = audit_project();

    let ran = Spawn::args(["validate", "--audit"])
        .cwd(project.path())
        .run();

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    let expected = "typdoc audit: 1 collections, 3 files (1 in no collection)\n\
        \n\
        notes  2 files   frontmatter.types 1 error \u{b7} frontmatter.unknown 1 info\n\
        \n\
        in no collection: README.md (1)\n\
        \n\
        no frontmatter: notes/b.md (1)\n";
    assert_eq!(ran.stdout, expected);
}

/// The header's total counts the overlapping file, and the file gets a line of its own, so the
/// text gives the same total as the JSON summary.
#[test]
fn the_text_form_of_audit_accounts_for_an_overlapping_file() {
    let project = overlap_and_a_clean_sibling();

    let ran = Spawn::args(["validate", "--audit"])
        .cwd(project.path())
        .run();

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    let expected = "typdoc audit: 2 collections, 2 files (0 in no collection)\n\
        \n\
        one  1 files   frontmatter.unknown 1 warn\n\
        two  0 files   clean\n\
        \n\
        matched by more than one collection: a.md (one, two) (1)\n";
    assert_eq!(ran.stdout, expected);
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

/// `error.md`'s finding has a position and `warn.md`'s has none. `info` cannot occur here: only
/// `--audit` reports at `info`.
#[test]
fn plain_validate_without_json_prints_one_line_per_finding_at_warn_and_error() {
    let project = Scratch::project(&[
        (
            ".typdoc/collections/notes.json",
            r#"{ "match": "*.md", "schema": "note.json" }"#,
        ),
        ("note.json", r#"{ "name": "note", "fields": {} }"#),
    ]);
    project.file("warn.md", "---\nextra: surprise\n---\n");
    project.file("error.md", "See [broken](./nope.md).\n");

    let ran = Spawn::args(["validate"]).cwd(project.path()).run();

    assert_eq!(ran.code, 2, "{}", ran.stderr);
    assert_eq!(ran.stderr, "");
    let expected = "path          level  rule                 message\n\
        error.md:1:5  error  body.links           link target missing: ./nope.md\n\
        warn.md       warn   frontmatter.unknown  the field `extra` is not a field of the schema\n";
    assert_eq!(ran.stdout, expected);
}

#[test]
fn a_clean_project_without_json_prints_nothing_and_exits_0() {
    let project = fixture("valid/minimal");

    let ran = Spawn::args(["validate"]).cwd(&project).run();

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    assert_eq!(ran.stdout, "");
    assert_eq!(ran.stderr, "");
}

#[test]
fn schemas_alone_without_json_prints_the_same_one_line_per_finding_shape() {
    let project = Scratch::project(&[(
        ".typdoc/config.json",
        r#"{ "version": 1, "imports": { "https": "../elsewhere" } }"#,
    )]);
    project.file("x.md", "");

    let ran = Spawn::args(["validate", "--schemas"])
        .cwd(project.path())
        .run();

    assert_eq!(ran.code, 2, "{}", ran.stderr);
    assert_eq!(ran.stderr, "");
    let expected = "path                 level  rule          message\n\
        .typdoc/config.json  error  schema.valid  the import name `https` is a URL scheme \
        (`http`, `https`, `mailto` and `file` are reserved), and the two would be told apart \
        wrongly\n";
    assert_eq!(ran.stdout, expected);
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

// `schema.valid`, `collections.overlap`, `keys.unique` and `filename.pattern`

#[test]
fn a_schema_valid_finding_carries_no_namespace_collection_or_key() {
    let object = broken("schema.valid");

    let findings = object["findings"].as_array().unwrap();
    assert_eq!(findings.len(), 1, "{findings:?}");
    let finding = &findings[0];
    assert_eq!(finding["rule"], json!("schema.valid"));
    assert!(finding["path"].is_string(), "{finding}");
    assert!(finding.get("namespace").is_none(), "{finding}");
    assert!(finding.get("collection").is_none(), "{finding}");
    assert!(finding.get("key").is_none(), "{finding}");
}

/// An overlap is never settled by precedence, so which collection is the file's is exactly what
/// is unknown: `collection` and `key` are left out.
#[test]
fn a_collections_overlap_finding_carries_a_namespace_and_no_collection_or_key() {
    let object = broken("collections.overlap");

    let findings: Vec<&Value> = object["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["rule"] == json!("collections.overlap"))
        .collect();
    assert_eq!(findings.len(), 1, "{findings:?}");
    let finding = findings[0];
    assert_eq!(finding["path"], json!("a.md"));
    assert_eq!(finding["namespace"], json!("default"));
    assert!(finding.get("collection").is_none(), "{finding}");
    assert!(finding.get("key").is_none(), "{finding}");
    assert!(
        finding["message"].as_str().unwrap().contains("one")
            && finding["message"].as_str().unwrap().contains("two"),
        "{finding}"
    );
}

/// The shape of `fixtures/broken/collections.overlap`, built here so a test can name its own
/// arguments.
fn overlap_and_a_clean_sibling() -> Scratch {
    let project = Scratch::project(&[
        (
            ".typdoc/collections/one.json",
            r#"{ "match": "*.md", "schema": "schemas/note.json" }"#,
        ),
        (
            ".typdoc/collections/two.json",
            r#"{ "match": "a.md", "schema": "schemas/note.json" }"#,
        ),
        ("schemas/note.json", r#"{ "name": "note", "fields": {} }"#),
    ]);
    project.file("a.md", "");
    project.file("b.md", "---\nextra: surprise\n---\n");
    project
}

/// An overlapping path was never checked against a schema, so it is in neither
/// `checked.documents` nor `checked.paths`, though its finding is in `findings`: the one finding
/// whose `path` is not in `checked.paths`.
#[test]
fn validate_on_an_overlapping_path_argument_reports_it_instead_of_aborting() {
    let project = overlap_and_a_clean_sibling();

    let alone = Spawn::args(["validate", "a.md", "--json"])
        .cwd(project.path())
        .run();
    assert_eq!(alone.code, 2, "{}", alone.stderr);
    assert_eq!(
        alone.stderr, "",
        "the report goes to stdout, not an aborted error object"
    );
    let object = alone.stdout_json();
    let findings = object["findings"].as_array().unwrap();
    assert_eq!(findings.len(), 1, "{findings:?}");
    assert_eq!(findings[0]["rule"], json!("collections.overlap"));
    assert_eq!(findings[0]["path"], json!("a.md"));
    assert_eq!(object["summary"]["checked"]["documents"], json!(0));
    assert_eq!(object["summary"]["checked"]["paths"], json!([]));

    let both = Spawn::args(["validate", "a.md", "b.md", "--json"])
        .cwd(project.path())
        .run();
    assert_eq!(both.code, 2, "{}", both.stderr);
    let object = both.stdout_json();
    let findings = object["findings"].as_array().unwrap();
    assert_eq!(findings.len(), 2, "{findings:?}");
    let rules: Vec<&str> = findings
        .iter()
        .map(|f| f["rule"].as_str().unwrap())
        .collect();
    assert!(rules.contains(&"collections.overlap"), "{findings:?}");
    assert!(rules.contains(&"frontmatter.unknown"), "{findings:?}");
    assert_eq!(object["summary"]["checked"]["documents"], json!(1));
    assert_eq!(object["summary"]["checked"]["paths"], json!(["b.md"]));
}

#[test]
fn validate_on_the_whole_project_does_not_count_an_overlapping_document_as_checked() {
    let project = overlap_and_a_clean_sibling();

    let ran = Spawn::args(["validate", "--json"])
        .cwd(project.path())
        .run();

    assert_eq!(ran.code, 2, "{}", ran.stderr);
    let object = ran.stdout_json();
    let findings = object["findings"].as_array().unwrap();
    let rules: Vec<&str> = findings
        .iter()
        .map(|f| f["rule"].as_str().unwrap())
        .collect();
    assert!(rules.contains(&"collections.overlap"), "{findings:?}");
    assert!(rules.contains(&"frontmatter.unknown"), "{findings:?}");
    assert_eq!(object["summary"]["checked"]["documents"], json!(1));
}

#[test]
fn audit_counts_an_overlapping_file_beside_unreported_not_inside_it() {
    let project = overlap_and_a_clean_sibling();

    let ran = Spawn::args(["validate", "--audit", "--json"])
        .cwd(project.path())
        .run();

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    let object = ran.stdout_json();
    assert_eq!(object["summary"]["checked"]["documents"], json!(1));
    assert_eq!(
        object["summary"]["unreported"],
        json!({ "uncollected": 0, "no_frontmatter": 0 })
    );
    assert_eq!(object["summary"]["overlapping"], json!(1));
    assert_eq!(object["audit"]["uncollected"], json!([]));
    assert_eq!(object["audit"]["no_frontmatter"], json!([]));
    assert_eq!(
        object["audit"]["overlapping"],
        json!([{ "path": "a.md", "collections": ["one", "two"] }])
    );
    let findings = object["findings"].as_array().unwrap();
    assert!(
        findings
            .iter()
            .any(|f| f["rule"] == json!("collections.overlap") && f["path"] == json!("a.md")),
        "an overlapping file is still reported under collections.overlap in audit mode: \
         {findings:?}"
    );
}

fn project_of_collections(collections: &[(&str, &str)]) -> Scratch {
    let mut files: Vec<(String, String)> = collections
        .iter()
        .map(|(name, matches)| {
            (
                format!(".typdoc/collections/{name}.json"),
                format!(r#"{{ "match": "{matches}", "schema": "note.json" }}"#),
            )
        })
        .collect();
    files.push((
        "note.json".to_owned(),
        r#"{ "name": "note", "fields": {} }"#.to_owned(),
    ));
    let borrowed: Vec<(&str, &str)> = files
        .iter()
        .map(|(path, text)| (path.as_str(), text.as_str()))
        .collect();
    Scratch::project(&borrowed)
}

fn audit_object(project: &Scratch, extra: &[&str]) -> Value {
    let mut args = vec!["validate", "--audit", "--json"];
    args.extend_from_slice(extra);
    let ran = Spawn::args(args).cwd(project.path()).run();
    assert_eq!(ran.code, 0, "{}", ran.stderr);
    ran.stdout_json()
}

/// `skills` holds 0, since its one file is matched twice; a listing built from the collections
/// that hold a document would leave it out.
#[test]
fn audit_lists_a_collection_whose_every_file_is_matched_twice_with_0_and_names_the_overlap() {
    let project = project_of_collections(&[("notes", "*.md"), ("skills", "a.md")]);
    project.file("a.md", "");
    project.file("b.md", "");

    let object = audit_object(&project, &[]);

    assert_eq!(
        object["audit"]["collections"],
        json!([
            { "name": "notes", "documents": 1 },
            { "name": "skills", "documents": 0 },
        ])
    );
    assert_eq!(
        object["audit"]["overlapping"],
        json!([{ "path": "a.md", "collections": ["notes", "skills"] }])
    );
}

/// Named to sort before and after the collection that holds the document, so the order by name
/// is read from both sides.
#[test]
fn audit_lists_a_collection_that_matches_nothing_with_0() {
    let project = project_of_collections(&[
        ("aaa", "nothing-here/*.md"),
        ("mmm", "*.md"),
        ("zzz", "also-nothing/*.md"),
    ]);
    project.file("a.md", "");

    let object = audit_object(&project, &[]);

    assert_eq!(
        object["audit"]["collections"],
        json!([
            { "name": "aaa", "documents": 0 },
            { "name": "mmm", "documents": 1 },
            { "name": "zzz", "documents": 0 },
        ])
    );
    assert_eq!(object["audit"]["overlapping"], json!([]));
}

/// Every entry carries its own collections, so one that carried only the first two, or another
/// file's, would differ from what is written here.
#[test]
fn audit_names_every_collection_of_an_overlap_sorted_by_name_and_the_list_by_path() {
    let project = project_of_collections(&[
        ("zeta", "*.md"),
        ("mid", "*.md"),
        ("alpha", "a.md"),
        ("solo", "solo.md"),
    ]);
    project.file("a.md", "");
    project.file("b.md", "");

    let object = audit_object(&project, &[]);

    assert_eq!(
        object["audit"]["overlapping"],
        json!([
            { "path": "a.md", "collections": ["alpha", "mid", "zeta"] },
            { "path": "b.md", "collections": ["mid", "zeta"] },
        ])
    );
    assert_eq!(
        object["audit"]["collections"],
        json!([
            { "name": "alpha", "documents": 0 },
            { "name": "mid", "documents": 0 },
            { "name": "solo", "documents": 0 },
            { "name": "zeta", "documents": 0 },
        ])
    );
    assert_eq!(object["summary"]["overlapping"], json!(2));
}

/// `a.md` is matched by `notes` and `skills`, `b.md` and `n.md` (no frontmatter) by `notes`
/// alone, and `deep/u.md` by none. Counting `a.md` in each collection would give 3 and 1.
/// `checked.documents` is 1: `n.md` is not evaluated and `a.md` has no one schema.
#[test]
fn a_file_matched_twice_is_in_the_number_of_no_collection_and_the_summary_numbers_hold() {
    let project = project_of_collections(&[("notes", "*.md"), ("skills", "a.md")]);
    project.file("a.md", "---\ntitle: x\n---\n");
    project.file("b.md", "---\ntitle: y\n---\n");
    project.file("n.md", "no frontmatter here\n");
    project.file("deep/u.md", "");

    let object = audit_object(&project, &[]);

    assert_eq!(
        object["audit"]["collections"],
        json!([
            { "name": "notes", "documents": 2 },
            { "name": "skills", "documents": 0 },
        ])
    );
    assert_eq!(object["summary"]["checked"]["documents"], json!(1));
    assert_eq!(
        object["summary"]["unreported"],
        json!({ "uncollected": 1, "no_frontmatter": 1 })
    );
    assert_eq!(object["summary"]["overlapping"], json!(1));
    assert_eq!(object["audit"]["uncollected"], json!(["deep/u.md"]));
    assert_eq!(object["audit"]["no_frontmatter"], json!(["n.md"]));
}

#[test]
fn the_text_form_of_audit_lists_every_collection_and_names_the_collections_of_each_overlap() {
    let project = project_of_collections(&[("notes", "*.md"), ("skills", "a.md")]);
    project.file("a.md", "");
    project.file("b.md", "---\n---\n");

    let ran = Spawn::args(["validate", "--audit"])
        .cwd(project.path())
        .run();

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    let expected = "typdoc audit: 2 collections, 2 files (0 in no collection)\n\
        \n\
        notes   1 files   clean\n\
        skills  0 files   clean\n\
        \n\
        matched by more than one collection: a.md (notes, skills) (1)\n";
    assert_eq!(ran.stdout, expected);
}

#[test]
fn the_text_form_of_audit_names_every_collection_of_each_of_several_overlaps() {
    let project = project_of_collections(&[
        ("zeta", "*.md"),
        ("mid", "*.md"),
        ("alpha", "a.md"),
        ("solo", "solo.md"),
    ]);
    project.file("a.md", "");
    project.file("b.md", "");

    let ran = Spawn::args(["validate", "--audit"])
        .cwd(project.path())
        .run();

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    let expected = "typdoc audit: 4 collections, 2 files (0 in no collection)\n\
        \n\
        alpha  0 files   clean\n\
        mid    0 files   clean\n\
        solo   0 files   clean\n\
        zeta   0 files   clean\n\
        \n\
        matched by more than one collection: a.md (alpha, mid, zeta), b.md (mid, zeta) (2)\n";
    assert_eq!(ran.stdout, expected);
}

/// `notes` matches in both namespaces, `only-two` reaches a file in `two` alone, and `only-one`
/// reaches none.
#[test]
fn a_collections_number_covers_every_namespace_the_audit_reports() {
    let project = Scratch::project(&[
        (
            ".typdoc/config.json",
            r#"{ "version": 1, "namespaces": ["one", "two"] }"#,
        ),
        (
            ".typdoc/collections/notes.json",
            r#"{ "match": "notes/*.md", "schema": "note.json" }"#,
        ),
        (
            ".typdoc/collections/only-two.json",
            r#"{ "match": "extra/*.md", "schema": "note.json" }"#,
        ),
        (
            ".typdoc/collections/only-one.json",
            r#"{ "match": "gone/*.md", "schema": "note.json" }"#,
        ),
        ("note.json", r#"{ "name": "note", "fields": {} }"#),
    ]);
    project.file("one/notes/a.md", "");
    project.file("two/notes/b.md", "");
    project.file("two/extra/c.md", "");

    let both = audit_object(&project, &[]);
    assert_eq!(
        both["audit"]["collections"],
        json!([
            { "name": "notes", "documents": 2 },
            { "name": "only-one", "documents": 0 },
            { "name": "only-two", "documents": 1 },
        ])
    );

    let narrowed = audit_object(&project, &["--namespace", "one"]);
    assert_eq!(
        narrowed["audit"]["collections"],
        json!([
            { "name": "notes", "documents": 1 },
            { "name": "only-one", "documents": 0 },
            { "name": "only-two", "documents": 0 },
        ])
    );
}

/// Unlike an overlap, a duplicate key is not ambiguous about which collection or key is
/// involved, so both are carried.
#[test]
fn a_keys_unique_finding_carries_its_namespace_collection_and_key() {
    let object = broken("keys.unique");

    let findings: Vec<&Value> = object["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["rule"] == json!("keys.unique"))
        .collect();
    assert_eq!(findings.len(), 2, "{findings:?}");
    for finding in findings {
        assert_eq!(finding["namespace"], json!("default"));
        assert_eq!(finding["key"], json!("WF-1"));
        assert!(finding["collection"].is_string(), "{finding}");
    }
}

/// `filename.pattern` is about a file no collection matched, so it is not a document: it
/// carries the namespace it was found in, but no `collection` or `key`.
#[test]
fn a_filename_pattern_finding_carries_a_namespace_and_no_collection_or_key() {
    let object = broken("filename.pattern");

    let findings = object["findings"].as_array().unwrap();
    assert_eq!(findings.len(), 1, "{findings:?}");
    let finding = &findings[0];
    assert_eq!(finding["rule"], json!("filename.pattern"));
    assert_eq!(finding["path"], json!("tickets/README.md"));
    assert_eq!(finding["namespace"], json!("default"));
    assert!(finding.get("collection").is_none(), "{finding}");
    assert!(finding.get("key").is_none(), "{finding}");
}

#[test]
fn two_namespaces_sharing_a_key_is_clean() {
    let project = Scratch::project(&[
        (
            ".typdoc/config.json",
            r#"{ "version": 1, "namespaces": ["ns1", "ns2"] }"#,
        ),
        (
            ".typdoc/collections/tickets.json",
            r#"{ "match": "{key}.md", "schema": "schemas/ticket.json" }"#,
        ),
        (
            "schemas/ticket.json",
            r#"{ "name": "ticket", "code": "WF", "fields": {} }"#,
        ),
        (".typdoc/state/ns1.json", r#"{ "tickets": { "last": 1 } }"#),
        (".typdoc/state/ns2.json", r#"{ "tickets": { "last": 1 } }"#),
        ("ns1/WF-1.md", ""),
        ("ns2/WF-1.md", ""),
    ]);

    let ran = Spawn::args(["validate", "--json"])
        .cwd(project.path())
        .run();

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    assert_eq!(ran.stdout_json()["findings"], json!([]));
}

#[test]
fn an_extends_cycle_is_rejected_by_schema_valid() {
    let project = Scratch::project(&[
        (
            ".typdoc/collections/notes.json",
            r#"{ "match": "*.md", "schema": "a.json" }"#,
        ),
        (
            "a.json",
            r#"{ "name": "a", "extends": "./b.json", "fields": {} }"#,
        ),
        (
            "b.json",
            r#"{ "name": "b", "extends": "./a.json", "fields": {} }"#,
        ),
    ]);
    project.file("x.md", "");

    let ran = Spawn::args(["validate", "--json"])
        .cwd(project.path())
        .run();

    assert_eq!(ran.code, 2, "{}", ran.stderr);
    let findings = ran.stdout_json()["findings"].clone();
    let rules: Vec<&str> = findings
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["rule"].as_str().unwrap())
        .collect();
    assert!(rules.contains(&"schema.valid"), "{findings}");
}

#[test]
fn redefining_an_inherited_field_needs_override_true() {
    let base = |kind: &str| {
        format!(r#"{{ "name": "base", "fields": {{ "title": {{ "type": "{kind}" }} }} }}"#)
    };
    for (override_written, expect_finding) in [(false, true), (true, false)] {
        let child = format!(
            r#"{{ "name": "child", "extends": "./base.json", "fields": {{ "title": {{ "type": "string", "override": {override_written} }} }} }}"#
        );
        let project = Scratch::project(&[
            (
                ".typdoc/collections/notes.json",
                r#"{ "match": "*.md", "schema": "child.json" }"#,
            ),
            ("base.json", &base("string")),
            ("child.json", &child),
        ]);
        project.file("x.md", "");

        let ran = Spawn::args(["validate", "--json"])
            .cwd(project.path())
            .run();

        let findings = ran.stdout_json()["findings"].clone();
        let tripped = findings
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["rule"] == json!("schema.valid"));
        assert_eq!(
            tripped, expect_finding,
            "override={override_written}: {findings}"
        );
    }
}

#[test]
fn invalid_field_options_and_names_are_schema_valid_and_not_a_crash() {
    for fields in [
        r#"{ "a": { "type": "string", "required": "yes" } }"#,
        r#"{ "a": { "type": "string", "values": ["x"] } }"#,
        r#"{ "a": { "type": "date", "target": "*" } }"#,
        r#"{ "path": { "type": "string" } }"#,
        r#"{ "$body": { "type": "string" } }"#,
        r#"{ "not valid": { "type": "string" } }"#,
    ] {
        let schema = format!(r#"{{ "name": "n", "fields": {fields} }}"#);
        let project = Scratch::project(&[
            (
                ".typdoc/collections/notes.json",
                r#"{ "match": "*.md", "schema": "n.json" }"#,
            ),
            ("n.json", &schema),
        ]);
        project.file("x.md", "");

        let ran = Spawn::args(["validate", "--json"])
            .cwd(project.path())
            .run();

        assert_eq!(ran.code, 2, "{fields}: {}", ran.stderr);
        let findings = ran.stdout_json()["findings"].clone();
        assert!(
            findings
                .as_array()
                .unwrap()
                .iter()
                .any(|f| f["rule"] == json!("schema.valid")),
            "{fields}: {findings}"
        );
    }
}

#[test]
fn an_import_name_that_is_a_reserved_url_scheme_is_schema_valid() {
    let project = Scratch::project(&[(
        ".typdoc/config.json",
        r#"{ "version": 1, "imports": { "https": "../elsewhere" } }"#,
    )]);
    project.file("x.md", "");

    let ran = Spawn::args(["validate", "--schemas", "--json"])
        .cwd(project.path())
        .run();

    assert_eq!(ran.code, 2, "{}", ran.stderr);
    let findings = ran.stdout_json()["findings"].clone();
    assert_eq!(findings.as_array().unwrap().len(), 1, "{findings}");
    assert_eq!(findings[0]["rule"], json!("schema.valid"));
    assert_eq!(findings[0]["path"], json!(".typdoc/config.json"));
}

/// Four names are reserved, not a shape, and `memory` and `chief` are the aliases the examples
/// of SPC-14 use.
#[test]
fn import_names_the_designs_own_examples_use_are_not_schema_valid() {
    let project = Scratch::project(&[(
        ".typdoc/config.json",
        r#"{ "version": 1, "imports": { "memory": "../elsewhere", "chief": "../elsewhere2" } }"#,
    )]);
    project.file("x.md", "");

    let ran = Spawn::args(["validate", "--schemas", "--json"])
        .cwd(project.path())
        .run();

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    assert_eq!(ran.stdout_json()["findings"], json!([]));
}

#[test]
fn a_file_fitting_one_of_several_coplaced_coded_collections_is_not_a_stray() {
    let project = Scratch::project(&[
        (
            ".typdoc/collections/wf.json",
            r#"{ "match": "tickets/{key}.md", "schema": "wf.json" }"#,
        ),
        (
            ".typdoc/collections/rfc.json",
            r#"{ "match": "tickets/{key}.md", "schema": "rfc.json" }"#,
        ),
        ("wf.json", r#"{ "name": "wf", "code": "WF", "fields": {} }"#),
        (
            "rfc.json",
            r#"{ "name": "rfc", "code": "RFC", "fields": {} }"#,
        ),
        (
            ".typdoc/state/default.json",
            r#"{ "wf": { "last": 1 }, "rfc": { "last": 4 } }"#,
        ),
        ("tickets/WF-1.md", ""),
        ("tickets/RFC-4.md", ""),
    ]);

    let ran = Spawn::args(["validate", "--json"])
        .cwd(project.path())
        .run();

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    assert_eq!(ran.stdout_json()["findings"], json!([]));
}

// Refs, and `names.shadowed`: finding shapes, and positive cases that a `broken/` fixture cannot
// show beside a negative one.

const REF_SCHEMA: [(&str, &str); 2] = [
    (
        ".typdoc/collections/notes.json",
        r#"{ "match": "*.md", "schema": "note.json" }"#,
    ),
    (
        "note.json",
        r#"{ "name": "note", "fields": { "see": { "type": "ref", "target": "*" } } }"#,
    ),
];

#[test]
fn a_ref_whose_case_differs_from_the_files_is_not_found() {
    let project = Scratch::project(&REF_SCHEMA);
    project.file("Target.md", "");
    project.file("a.md", "---\nsee: target.md\n---\n");

    let ran = validate(&[], project.path());

    assert_eq!(ran.code, 2, "{}", ran.stderr);
    let findings = ran.stdout_json()["findings"].clone();
    let findings = findings.as_array().unwrap();
    assert_eq!(findings.len(), 1, "{findings:?}");
    assert_eq!(findings[0]["rule"], json!("refs.resolve"));
    assert_eq!(findings[0]["field"], json!("see"));
    assert!(
        findings[0]["message"]
            .as_str()
            .unwrap()
            .contains("not found"),
        "{findings:?}"
    );
}

/// `README.txt` is in no collection, so this takes the same branch as the case-mismatch test
/// above, with the case matching.
#[test]
fn a_ref_whose_case_exactly_matches_a_file_outside_every_collection_is_found() {
    let project = Scratch::project(&REF_SCHEMA);
    project.file("README.txt", "");
    project.file("a.md", "---\nsee: README.txt\n---\n");

    let ran = validate(&[], project.path());

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    assert_eq!(ran.stdout_json()["findings"], json!([]));
}

#[test]
fn chief_double_colon_wf_5_in_a_project_with_several_namespaces_is_bad_prefix() {
    let project = Scratch::project(&[
        (
            ".typdoc/config.json",
            r#"{ "version": 1, "namespaces": ["story-1", "story-2"] }"#,
        ),
        (
            ".typdoc/collections/notes.json",
            r#"{ "match": "*.md", "schema": "note.json" }"#,
        ),
        (
            "note.json",
            r#"{ "name": "note", "fields": { "see": { "type": "ref", "target": "*" } } }"#,
        ),
    ]);
    // Both namespace folders must exist: a plain (non-glob) `namespaces` entry naming a folder
    // that is not there is a config error, and `story-2` needs no document of its own for the
    // project to genuinely have several namespaces.
    project.file("story-2/.gitkeep", "");
    project.file("story-1/a.md", "---\nsee: chief::WF-5\n---\n");

    let ran = validate(&[], project.path());

    assert_eq!(ran.code, 2, "{}", ran.stderr);
    let findings = ran.stdout_json()["findings"].clone();
    let findings = findings.as_array().unwrap();
    assert_eq!(findings.len(), 1, "{findings:?}");
    assert_eq!(findings[0]["rule"], json!("refs.resolve"));
    assert_eq!(findings[0]["path"], json!("story-1/a.md"));
    assert!(
        findings[0]["message"]
            .as_str()
            .unwrap()
            .contains("no namespace"),
        "{findings:?}"
    );
}

#[test]
fn a_bare_key_resolves_in_the_documents_own_namespace() {
    let project = Scratch::project(&[
        (
            ".typdoc/collections/tickets.json",
            r#"{ "match": "tickets/{key}.md", "schema": "wf.json" }"#,
        ),
        ("wf.json", r#"{ "name": "wf", "code": "WF", "fields": {} }"#),
        (
            ".typdoc/collections/notes.json",
            r#"{ "match": "notes/*.md", "schema": "note.json" }"#,
        ),
        (
            "note.json",
            r#"{ "name": "note", "fields": { "see": { "type": "ref", "target": "*" } } }"#,
        ),
        (
            ".typdoc/state/default.json",
            r#"{ "tickets": { "last": 1 } }"#,
        ),
    ]);
    project.file("tickets/WF-1.md", "");
    project.file("notes/a.md", "---\nsee: WF-1\n---\n");

    let ran = validate(&[], project.path());

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    assert_eq!(ran.stdout_json()["findings"], json!([]));
}

#[test]
fn ref_base_namespace_reads_a_relative_ref_from_the_namespace_folder() {
    let project = Scratch::project(&[
        (
            ".typdoc/collections/notes.json",
            r#"{ "match": "notes/*.md", "schema": "note.json", "refBase": "namespace" }"#,
        ),
        (
            "note.json",
            r#"{ "name": "note", "fields": { "see": { "type": "ref", "target": "*" } } }"#,
        ),
    ]);
    project.file("target.md", "");
    project.file("notes/a.md", "---\nsee: target.md\n---\n");

    let ran = validate(&[], project.path());

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    assert_eq!(ran.stdout_json()["findings"], json!([]));
}

#[test]
fn refs_target_refuses_a_schema_not_named_by_target() {
    let project = Scratch::project(&[
        (
            ".typdoc/collections/allowed.json",
            r#"{ "match": "allowed/*.md", "schema": "allowed.json" }"#,
        ),
        ("allowed.json", r#"{ "name": "allowed", "fields": {} }"#),
        (
            ".typdoc/collections/tickets.json",
            r#"{ "match": "*.md", "schema": "ticket.json" }"#,
        ),
        (
            "ticket.json",
            r#"{ "name": "ticket", "fields": { "see": { "type": "ref", "target": ["allowed"] } } }"#,
        ),
    ]);
    project.file("allowed/a.md", "");
    project.file("good.md", "---\nsee: allowed/a.md\n---\n");

    let ran = validate(&[], project.path());

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    assert_eq!(ran.stdout_json()["findings"], json!([]));
}

#[test]
fn a_coded_document_referenced_by_key_is_clean_and_by_path_warns() {
    let files = [
        (
            ".typdoc/collections/tickets.json",
            r#"{ "match": "tickets/{key}.md", "schema": "wf.json" }"#,
        ),
        ("wf.json", r#"{ "name": "wf", "code": "WF", "fields": {} }"#),
        (
            ".typdoc/collections/notes.json",
            r#"{ "match": "notes/*.md", "schema": "note.json" }"#,
        ),
        (
            "note.json",
            r#"{ "name": "note", "fields": { "see": { "type": "ref", "target": "*" } } }"#,
        ),
        (
            ".typdoc/state/default.json",
            r#"{ "tickets": { "last": 1 } }"#,
        ),
    ];
    let by_key = Scratch::project(&files);
    by_key.file("tickets/WF-1.md", "");
    by_key.file("notes/a.md", "---\nsee: WF-1\n---\n");
    let by_path = Scratch::project(&files);
    by_path.file("tickets/WF-1.md", "");
    by_path.file("notes/a.md", "---\nsee: ../tickets/WF-1.md\n---\n");

    let key_ran = validate(&[], by_key.path());
    let path_ran = validate(&[], by_path.path());

    assert_eq!(key_ran.code, 0, "{}", key_ran.stderr);
    assert_eq!(key_ran.stdout_json()["findings"], json!([]));
    assert_eq!(
        path_ran.code, 0,
        "warn does not fail the run: {}",
        path_ran.stderr
    );
    let findings = path_ran.stdout_json()["findings"].clone();
    let findings = findings.as_array().unwrap();
    assert_eq!(findings.len(), 1, "{findings:?}");
    assert_eq!(findings[0]["rule"], json!("refs.codedByPath"));
    assert_eq!(findings[0]["level"], json!("warn"));
}

#[test]
fn refs_acyclic_reports_every_document_on_the_cycle_and_a_plain_ref_field_may_cycle_freely() {
    let files = [
        (
            ".typdoc/collections/notes.json",
            r#"{ "match": "*.md", "schema": "note.json" }"#,
        ),
        (
            "note.json",
            r#"{ "name": "note", "fields": {
                "blocked_by": { "type": "ref[]", "target": "*", "acyclic": true },
                "related": { "type": "ref[]", "target": "*" }
            } }"#,
        ),
    ];
    let cyclic = Scratch::project(&files);
    cyclic.file("a.md", "---\nblocked_by: [b.md]\n---\n");
    cyclic.file("b.md", "---\nblocked_by: [a.md]\n---\n");
    let free = Scratch::project(&files);
    free.file("a.md", "---\nrelated: [b.md]\n---\n");
    free.file("b.md", "---\nrelated: [a.md]\n---\n");

    let cyclic_ran = validate(&[], cyclic.path());
    let free_ran = validate(&[], free.path());

    assert_eq!(cyclic_ran.code, 2, "{}", cyclic_ran.stderr);
    let findings = cyclic_ran.stdout_json()["findings"].clone();
    let findings = findings.as_array().unwrap();
    assert_eq!(findings.len(), 2, "{findings:?}");
    for finding in findings {
        assert_eq!(finding["rule"], json!("refs.acyclic"));
        assert_eq!(finding["field"], json!("blocked_by"));
    }
    assert_eq!(free_ran.code, 0, "{}", free_ran.stderr);
    assert_eq!(free_ran.stdout_json()["findings"], json!([]));
}

/// Every finding in a `paths` scope is about a document the caller named.
#[test]
fn refs_acyclic_in_paths_scope_reports_only_the_named_document_on_the_cycle() {
    let project = Scratch::project(&[
        (
            ".typdoc/collections/notes.json",
            r#"{ "match": "*.md", "schema": "note.json" }"#,
        ),
        (
            "note.json",
            r#"{ "name": "note", "fields": {
                "blocked_by": { "type": "ref[]", "target": "*", "acyclic": true }
            } }"#,
        ),
    ]);
    project.file("a.md", "---\nblocked_by: [b.md]\n---\n");
    project.file("b.md", "---\nblocked_by: [a.md]\n---\n");

    let ran = validate(&["a.md"], project.path());

    assert_eq!(ran.code, 2, "{}", ran.stderr);
    let findings = ran.stdout_json()["findings"].clone();
    let findings = findings.as_array().unwrap();
    assert_eq!(findings.len(), 1, "{findings:?}");
    assert_eq!(findings[0]["rule"], json!("refs.acyclic"));
    assert_eq!(findings[0]["path"], json!("a.md"));
}

#[test]
fn refs_moved_replaces_refs_resolve_and_names_the_new_identity() {
    let files = [
        (
            ".typdoc/collections/notes.json",
            r#"{ "match": "*.md", "schema": "note.json" }"#,
        ),
        (
            "note.json",
            r#"{ "name": "note", "fields": {
                "moved_from": { "type": "list", "auto": "moves" },
                "see": { "type": "ref", "target": "*" }
            } }"#,
        ),
    ];
    let moved = Scratch::project(&files);
    moved.file("new.md", "---\nmoved_from: [old.md]\n---\n");
    moved.file("b.md", "---\nsee: old.md\n---\n");
    let not_recorded = Scratch::project(&files);
    not_recorded.file("new.md", "---\n---\n");
    not_recorded.file("b.md", "---\nsee: old.md\n---\n");

    let moved_ran = validate(&[], moved.path());
    let plain_ran = validate(&[], not_recorded.path());

    assert_eq!(moved_ran.code, 2, "{}", moved_ran.stderr);
    let findings = moved_ran.stdout_json()["findings"].clone();
    let findings = findings.as_array().unwrap();
    assert_eq!(findings.len(), 1, "{findings:?}");
    assert_eq!(findings[0]["rule"], json!("refs.moved"));
    assert_eq!(findings[0]["path"], json!("b.md"));
    assert!(
        findings[0]["message"].as_str().unwrap().contains("new.md"),
        "{findings:?}"
    );

    assert_eq!(plain_ran.code, 2, "{}", plain_ran.stderr);
    let findings = plain_ran.stdout_json()["findings"].clone();
    let findings = findings.as_array().unwrap();
    assert_eq!(findings.len(), 1, "{findings:?}");
    assert_eq!(findings[0]["rule"], json!("refs.resolve"));
}

/// `names.shadowed` is a fact about the config, not about a document: no `namespace`,
/// `collection` or `key`, the same shape `schema.valid` uses.
#[test]
fn a_names_shadowed_finding_carries_no_namespace_collection_or_key() {
    let project = Scratch::project(&[]);
    project.file(
        ".typdoc/config.json",
        r#"{ "version": 1, "namespaces": ["shadow_ns"], "imports": { "shadow_ns": "../elsewhere" } }"#,
    );
    // A collection that matches nothing here, only so the project is not also
    // `collections.empty` — this test is about `names.shadowed` alone.
    project.file(
        ".typdoc/collections/notes.json",
        r#"{ "match": "notes/*.md", "schema": "note.json" }"#,
    );
    project.file("note.json", r#"{ "name": "note", "fields": {} }"#);
    project.file("shadow_ns/a.md", "");

    let ran = validate(&[], project.path());

    assert_eq!(ran.code, 0, "warn does not fail the run: {}", ran.stderr);
    let findings = ran.stdout_json()["findings"].clone();
    let findings = findings.as_array().unwrap();
    assert_eq!(findings.len(), 1, "{findings:?}");
    let finding = &findings[0];
    assert_eq!(finding["rule"], json!("names.shadowed"));
    assert_eq!(finding["level"], json!("warn"));
    assert!(finding.get("namespace").is_none(), "{finding}");
    assert!(finding.get("collection").is_none(), "{finding}");
    assert!(finding.get("key").is_none(), "{finding}");
}

#[test]
fn names_shadowed_is_reported_under_schemas_only_too() {
    let project = Scratch::project(&[]);
    project.file(
        ".typdoc/config.json",
        r#"{ "version": 1, "namespaces": ["shadow_ns"], "imports": { "shadow_ns": "../elsewhere" } }"#,
    );
    project.file("shadow_ns/a.md", "");

    let ran = validate(&["--schemas"], project.path());

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    let findings = ran.stdout_json()["findings"].clone();
    let findings = findings.as_array().unwrap();
    assert_eq!(findings.len(), 1, "{findings:?}");
    assert_eq!(findings[0]["rule"], json!("names.shadowed"));
}

// `files.unreadable`, and documents under a folder whose name begins with a dot

const EVERY_MARKDOWN: [(&str, &str); 2] = [
    (
        ".typdoc/collections/notes.json",
        r#"{ "match": "**/*.md", "schema": "note.json" }"#,
    ),
    ("note.json", r#"{ "name": "note", "fields": {} }"#),
];

#[test]
fn a_symbolic_link_a_match_reaches_is_files_unreadable_and_every_other_file_is_still_checked() {
    let project = Scratch::project(&EVERY_MARKDOWN);
    project.file("a.md", "---\n---\n");
    project.file("real/b.md", "---\n---\n");
    project.symlink("link.md", "a.md");
    project.symlink("linked", "real");

    let ran = validate(&[], project.path());

    assert_eq!(ran.code, 2, "{}", ran.stderr);
    let object = ran.stdout_json();
    let findings = object["findings"].as_array().unwrap().clone();
    assert_eq!(findings.len(), 2, "{object}");
    for (finding, path) in findings.iter().zip(["link.md", "linked"]) {
        assert_eq!(finding["rule"], json!("files.unreadable"), "{object}");
        assert_eq!(finding["level"], json!("error"), "{object}");
        assert_eq!(finding["path"], json!(path), "{object}");
        assert_eq!(finding["namespace"], json!("default"), "{object}");
        assert_eq!(finding["collection"], Value::Null, "{object}");
        assert_eq!(finding["key"], Value::Null, "{object}");
    }
    assert_eq!(
        object["summary"]["checked"]["documents"],
        json!(2),
        "{object}"
    );
}

/// A file name that is not valid UTF-8: the last step of the template reaches it, so it is
/// skipped where a name it could match would have been taken.
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "a non-UTF-8 filename needs a POSIX filesystem that allows arbitrary bytes in a \
              name; APFS on macOS refuses to create one at all (EILSEQ), confirmed on a real \
              macos-latest CI run, 2026-09-24"
)]
#[test]
fn a_file_name_that_is_not_valid_utf8_is_files_unreadable_and_the_rest_is_still_checked() {
    let project = Scratch::project(&EVERY_MARKDOWN);
    project.file("a.md", "---\n---\n");
    project.file_named_by_bytes(b"\xff.md", "---\n---\n");

    let ran = validate(&[], project.path());

    assert_eq!(ran.code, 2, "{}", ran.stderr);
    let object = ran.stdout_json();
    let findings = object["findings"].as_array().unwrap().clone();
    assert_eq!(findings.len(), 1, "{object}");
    assert_eq!(findings[0]["rule"], json!("files.unreadable"), "{object}");
    assert_eq!(findings[0]["path"], json!("\u{fffd}.md"), "{object}");
    assert_eq!(
        object["summary"]["checked"]["documents"],
        json!(1),
        "{object}"
    );
}

/// A folder name that is not valid UTF-8 goes through the step that enters a folder, not the
/// step that takes a file, so it is its own case.
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "a non-UTF-8 filename needs a POSIX filesystem that allows arbitrary bytes in a \
              name; APFS on macOS refuses to create one at all (EILSEQ), confirmed on a real \
              macos-latest CI run, 2026-09-24"
)]
#[test]
fn a_folder_name_that_is_not_valid_utf8_is_files_unreadable_and_the_rest_is_still_checked() {
    let project = Scratch::project(&EVERY_MARKDOWN);
    project.file("a.md", "---\n---\n");
    project.file_named_by_bytes(b"\xff/b.md", "---\n---\n");

    let ran = validate(&[], project.path());

    assert_eq!(ran.code, 2, "{}", ran.stderr);
    let object = ran.stdout_json();
    let findings = object["findings"].as_array().unwrap().clone();
    assert_eq!(findings.len(), 1, "{object}");
    assert_eq!(findings[0]["rule"], json!("files.unreadable"), "{object}");
    assert_eq!(findings[0]["path"], json!("\u{fffd}"), "{object}");
    assert_eq!(
        object["summary"]["checked"]["documents"],
        json!(1),
        "{object}"
    );
}

// Temp files and leftovers. The reserved name and the rule that skips it are unit-tested in
// `typdoc-core`; these go through the binary.

/// The broadest glob, so a leftover is reached whatever its name.
const MATCH_STAR: [(&str, &str); 2] = [
    (
        ".typdoc/collections/notes.json",
        r#"{ "match": "*", "schema": "note.json" }"#,
    ),
    ("note.json", r#"{ "name": "note", "fields": {} }"#),
];

/// The glob a leftover ending in `.md` would otherwise fit.
const MATCH_MD: [(&str, &str); 2] = [
    (
        ".typdoc/collections/notes.json",
        r#"{ "match": "*.md", "schema": "note.json" }"#,
    ),
    ("note.json", r#"{ "name": "note", "fields": {} }"#),
];

#[test]
fn a_leftover_a_match_of_star_reaches_is_files_leftover_and_counted_in_not_read() {
    let project = Scratch::project(&MATCH_STAR);
    project.file("a.md", "---\n---\n");
    let leftover = format!("{}12345-cafef00d1", TEMP_PREFIX);
    assert!(
        is_temp_name(&leftover),
        "the fixture's own name must have the reserved shape: {leftover}"
    );
    project.file(&leftover, "");

    let ran = validate(&["--audit"], project.path());

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    let object = ran.stdout_json();
    let findings = object["findings"].as_array().unwrap().clone();
    assert_eq!(findings.len(), 1, "{object}");
    assert_eq!(findings[0]["rule"], json!("files.leftover"), "{object}");
    assert_eq!(findings[0]["level"], json!("warn"), "{object}");
    assert_eq!(findings[0]["path"], json!(leftover), "{object}");
    assert_eq!(findings[0]["namespace"], json!("default"), "{object}");
    assert_eq!(findings[0]["collection"], Value::Null, "{object}");
    assert_eq!(findings[0]["key"], Value::Null, "{object}");
    assert_eq!(
        object["summary"]["checked"]["documents"],
        json!(1),
        "{object}: the leftover must never be a document"
    );
    assert_eq!(object["summary"]["not_read"], json!(1), "{object}");
    assert_eq!(
        object["audit"]["uncollected"],
        json!([]),
        "{object}: a leftover is not uncollected either"
    );
    let not_read = object["audit"]["not_read"].as_array().unwrap();
    assert_eq!(not_read.len(), 1, "{object}");
    assert_eq!(not_read[0]["path"], json!(leftover), "{object}");
    assert!(
        not_read[0]["reason"].as_str().unwrap().contains("leftover"),
        "{object}"
    );
}

/// An always-on rule's level is never merged or raised, so `files.leftover` stays `warn` under
/// `--strict`.
#[test]
fn a_leftover_a_match_of_star_dot_md_reaches_is_files_leftover_at_warn_even_under_strict() {
    let project = Scratch::project(&MATCH_MD);
    project.file("a.md", "---\n---\n");
    let leftover = format!("{}54321-deadbeef2.md", TEMP_PREFIX);
    assert!(
        is_temp_name(&leftover),
        "the fixture's own name must have the reserved shape: {leftover}"
    );
    project.file(&leftover, "");

    let ran = validate(&["--strict"], project.path());

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    let object = ran.stdout_json();
    let findings = object["findings"].as_array().unwrap().clone();
    assert_eq!(findings.len(), 1, "{object}");
    assert_eq!(findings[0]["rule"], json!("files.leftover"), "{object}");
    assert_eq!(findings[0]["level"], json!("warn"), "{object}");
    assert_eq!(findings[0]["path"], json!(leftover), "{object}");
    assert_eq!(
        object["summary"]["checked"]["documents"],
        json!(1),
        "{object}"
    );
}

/// The twin of `a_dotted_file_name_in_a_coded_collections_folder_is_a_stray`: the leftover fits
/// no `match` (`{key}` needs the code and a number), so neither `files.leftover` nor
/// `filename.pattern` reports it.
#[test]
fn a_leftover_in_a_coded_collections_folder_is_not_a_stray_or_anything_else() {
    let project = Scratch::project(&[]);
    project.file(
        ".typdoc/collections/tickets.json",
        r#"{ "match": "tickets/{key}.md", "schema": "ticket.json" }"#,
    );
    project.file(
        "ticket.json",
        r#"{ "name": "ticket", "code": "WF", "fields": {} }"#,
    );
    project.file(
        ".typdoc/state/default.json",
        r#"{ "tickets": { "last": 1 } }"#,
    );
    project.file("tickets/WF-1.md", "---\n---\n");
    let leftover = format!("tickets/{}1-abc", TEMP_PREFIX);
    project.file(&leftover, "");

    let ran = validate(&[], project.path());

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    let object = ran.stdout_json();
    assert_eq!(object["findings"], json!([]), "{object}");
}

#[test]
fn a_leftover_left_by_a_stopped_run_is_reported_once_then_removed_by_the_next_command_that_holds_the_lock()
 {
    let project = Scratch::project(&MATCH_MD);
    project.file("a.md", "---\n---\n");
    let leftover = format!("{}77777-1234abcd.md", TEMP_PREFIX);
    project.file(&leftover, "");
    let leftover_path = project.path().join(&leftover);
    assert!(
        leftover_path.is_file(),
        "the leftover must be manufactured on disk first"
    );

    let ran = validate(&["--audit"], project.path());
    assert_eq!(ran.code, 0, "{}", ran.stderr);
    let before = ran.stdout_json();
    assert_eq!(before["summary"]["not_read"], json!(1), "{before}");
    assert_eq!(
        before["findings"][0]["rule"],
        json!("files.leftover"),
        "{before}"
    );

    // Played by hand: the next command that holds the lock removes it.
    let fs = typdoc_fs::SystemFs;
    let lock_path = project.path().join(".typdoc/locks/default.lock");
    let lock = typdoc_core::acquire(
        &fs,
        &typdoc_testkit::fake::FixedClock::new(),
        lock_path,
        "leftover-test-host",
        std::time::Duration::from_secs(5),
    )
    .expect("the lock is acquired");
    let found = typdoc_core::find_leftovers(project.path());
    assert_eq!(found, vec![leftover_path.clone()], "{found:?}");
    let removed = typdoc_core::remove_leftovers(&fs, &lock, &found);
    assert_eq!(removed, 1, "the leftover must actually be removed");
    assert!(
        !leftover_path.exists(),
        "the leftover must be gone from disk"
    );
    typdoc_core::release(lock).expect("the lock releases");

    let ran = validate(&["--audit"], project.path());
    assert_eq!(ran.code, 0, "{}", ran.stderr);
    let after = ran.stdout_json();
    assert_eq!(after["summary"]["not_read"], json!(0), "{after}");
    assert_eq!(after["findings"], json!([]), "{after}");
}

/// `valid/dot-folder` names `.agents` in `match`. The list holds a file whose own name begins
/// with a dot too, since the leading-dot rule is about folders.
#[test]
fn an_audit_of_a_project_under_a_dot_folder_lists_what_no_collection_covers_there() {
    let ran = validate(&["--audit"], &fixture("valid/dot-folder"));

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    let object = ran.stdout_json();
    assert_eq!(
        object["audit"]["uncollected"],
        json!([".agents/.private.md", ".agents/other.md"]),
        "{object}"
    );
    assert_eq!(
        object["summary"]["checked"]["documents"],
        json!(1),
        "{object}"
    );
}

#[test]
fn a_dot_folder_a_match_names_after_a_wildcard_is_read_like_one_it_names_first() {
    let project = Scratch::project(&[]);
    project.file(
        ".typdoc/collections/notes.json",
        r#"{ "match": "**/.agents/*.md", "schema": "note.json" }"#,
    );
    project.file("note.json", r#"{ "name": "note", "fields": {} }"#);
    project.file(".agents/a.md", "---\n---\n");
    project.file(".agents/sub/x.md", "---\n---\n");

    let ran = validate(&["--audit"], project.path());

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    let object = ran.stdout_json();
    assert_eq!(
        object["summary"]["checked"]["documents"],
        json!(1),
        "{object}"
    );
    assert_eq!(
        object["audit"]["uncollected"],
        json!([".agents/sub/x.md"]),
        "{object}"
    );
}

#[test]
fn a_dotted_file_name_in_a_coded_collections_folder_is_a_stray() {
    let project = Scratch::project(&[]);
    project.file(
        ".typdoc/collections/tickets.json",
        r#"{ "match": "tickets/{key}.md", "schema": "ticket.json" }"#,
    );
    project.file(
        "ticket.json",
        r#"{ "name": "ticket", "code": "WF", "fields": {} }"#,
    );
    project.file(
        ".typdoc/state/default.json",
        r#"{ "tickets": { "last": 1 } }"#,
    );
    project.file("tickets/WF-1.md", "---\n---\n");
    project.file("tickets/.notes.md", "---\n---\n");

    let ran = validate(&[], project.path());

    assert_eq!(ran.code, 2, "{}", ran.stderr);
    let object = ran.stdout_json();
    let findings = object["findings"].as_array().unwrap().clone();
    assert_eq!(findings.len(), 1, "{object}");
    assert_eq!(findings[0]["rule"], json!("filename.pattern"), "{object}");
    assert_eq!(findings[0]["path"], json!("tickets/.notes.md"), "{object}");
}

// The account of `validate --audit`

/// Read from the collection files as JSON, never through typdoc. The last step of a template
/// names a file, and a step holding a wildcard names no folder, though the plain steps after it
/// still do.
fn folders_a_match_names(project: &std::path::Path) -> std::collections::BTreeSet<String> {
    let mut named = std::collections::BTreeSet::new();
    let Ok(entries) = std::fs::read_dir(project.join(".typdoc/collections")) else {
        return named;
    };
    for entry in entries {
        let text = std::fs::read_to_string(entry.unwrap().path()).unwrap();
        let Ok(object) = serde_json::from_str::<Value>(&text) else {
            continue;
        };
        let Some(template) = object["match"].as_str() else {
            continue;
        };
        let segments: Vec<&str> = template.split('/').collect();
        for segment in &segments[..segments.len().saturating_sub(1)] {
            if !segment.contains('*') && !segment.contains('{') {
                named.insert((*segment).to_owned());
            }
        }
    }
    named
}

/// Walked here rather than through `typdoc_core::index`, so the count owes nothing to typdoc's
/// own idea of which files it reads. No fixture used here has a name that is not valid UTF-8, so
/// that reason for not reading an entry is not modelled. Returns `(documents, not_read)`.
fn count_markdown_files(
    dir: &std::path::Path,
    named: &std::collections::BTreeSet<String>,
) -> (usize, usize) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return (0, 0);
    };
    let (mut documents, mut not_read) = (0, 0);
    for entry in entries {
        let entry = entry.unwrap();
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        let path = entry.path();
        let file_type = entry.file_type().unwrap();
        if file_type.is_symlink() {
            if name.ends_with(".md") {
                not_read += 1;
            }
            continue;
        }
        if file_type.is_dir() {
            let reached = !name.starts_with('.') || named.contains(name);
            if !reached || path.join(".typdoc/config.json").is_file() {
                continue;
            }
            let (sub_documents, sub_not_read) = count_markdown_files(&path, named);
            documents += sub_documents;
            not_read += sub_not_read;
        } else if file_type.is_file() && name.ends_with(".md") {
            if is_temp_name(name) {
                not_read += 1;
            } else {
                documents += 1;
            }
        }
    }
    (documents, not_read)
}

/// Only the folder each name maps to is read here, never how the name was matched. Returns
/// `(documents, not_read)`.
fn independent_document_count(project: &std::path::Path, namespaces: &[&str]) -> (usize, usize) {
    let named = folders_a_match_names(project);
    namespaces
        .iter()
        .map(|name| {
            let folder = if *name == "default" {
                project.to_path_buf()
            } else {
                project.join(name)
            };
            count_markdown_files(&folder, &named)
        })
        .fold((0, 0), |(documents, not_read), (d, n)| {
            (documents + d, not_read + n)
        })
}

/// The files and entries the run meets are counted without typdoc. A `broken/config.*` fixture
/// never reaches a report, so it is asserted to fail rather than skipped.
///
/// `summary.overlapping` is read from the output, not counted from `findings`: counting the
/// findings would check that an overlap produces one, which another test does, and not that the
/// summary reports the count. `broken/collections.overlap` is the fixture that gives it a value
/// other than 0.
#[test]
fn the_accounting_invariant_holds_on_every_fixture_project() {
    let root = fixture("");
    let mut checked_any = false;
    for group in ["valid", "broken"] {
        let dir = root.join(group);
        let mut names: Vec<String> = std::fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        for name in names {
            let project = dir.join(&name);
            if !project.join(".typdoc/config.json").is_file() {
                continue;
            }
            checked_any = true;
            // A `broken/` fixture may need its `env` to trip its rule. Its `command` is not
            // read: this test runs its own `--audit`.
            let mut spawn = Spawn::args(["validate", "--audit", "--json"]).cwd(&project);
            if group == "broken" {
                let spec = typdoc_testkit::spec::FixtureSpec::load(&project, &name)
                    .unwrap_or_else(|e| panic!("{group}/{name}: {e}"));
                for (var, value) in &spec.env {
                    spawn = spawn.var(var, value);
                }
            }
            let ran = spawn.run();
            if name.starts_with("config.") {
                assert_ne!(
                    ran.code, 0,
                    "{group}/{name}: a config error fixture is expected to fail to load: {}",
                    ran.stdout
                );
                assert_eq!(ran.stdout, "", "{group}/{name}");
                continue;
            }
            assert_eq!(ran.code, 0, "{group}/{name}: {}", ran.stderr);
            let object = ran.stdout_json();
            let namespaces: Vec<&str> = object["summary"]["checked"]["namespaces"]
                .as_array()
                .unwrap_or_else(|| panic!("{group}/{name}: {object}"))
                .iter()
                .map(|v| v.as_str().unwrap())
                .collect();
            let (independent_documents, independent_not_read) =
                independent_document_count(&project, &namespaces);
            let checked = object["summary"]["checked"]["documents"].as_u64().unwrap() as usize;
            let uncollected = object["summary"]["unreported"]["uncollected"]
                .as_u64()
                .unwrap_or_else(|| panic!("{group}/{name}: {object}"))
                as usize;
            let no_frontmatter = object["summary"]["unreported"]["no_frontmatter"]
                .as_u64()
                .unwrap() as usize;
            let overlapping = object["summary"]["overlapping"]
                .as_u64()
                .unwrap_or_else(|| panic!("{group}/{name}: {object}"))
                as usize;
            let not_read = object["summary"]["not_read"]
                .as_u64()
                .unwrap_or_else(|| panic!("{group}/{name}: {object}"))
                as usize;
            assert_eq!(
                independent_documents,
                checked + uncollected + no_frontmatter + overlapping,
                "{group}/{name}: independently counted {independent_documents} .md files, typdoc \
                 reports checked={checked} uncollected={uncollected} \
                 no_frontmatter={no_frontmatter} overlapping={overlapping}: {object}"
            );
            // Without the entries the run met and did not read, the account would be short.
            assert_eq!(
                independent_not_read, not_read,
                "{group}/{name}: independently counted {independent_not_read} not-read entries, \
                 typdoc reports not_read={not_read}: {object}"
            );
            let not_read_listed = object["audit"]["not_read"].as_array().unwrap();
            assert_eq!(not_read_listed.len(), not_read, "{group}/{name}: {object}");
            for entry in not_read_listed {
                let path = entry["path"]
                    .as_str()
                    .unwrap_or_else(|| panic!("{group}/{name}: {entry}"));
                assert!(
                    !entry["reason"].as_str().unwrap_or_default().is_empty(),
                    "{group}/{name}: {entry}"
                );
                assert!(
                    project.join(path).symlink_metadata().is_ok(),
                    "{group}/{name}: {path}"
                );
            }
            let listed = object["audit"]["overlapping"].as_array().unwrap();
            assert_eq!(listed.len(), overlapping, "{group}/{name}: {object}");
            for entry in listed {
                let path = entry["path"]
                    .as_str()
                    .unwrap_or_else(|| panic!("{group}/{name}: {entry}"));
                assert!(project.join(path).is_file(), "{group}/{name}: {path}");
                let names = entry["collections"]
                    .as_array()
                    .unwrap_or_else(|| panic!("{group}/{name}: {entry}"));
                assert!(names.len() >= 2, "{group}/{name}: {entry}");
            }
            // A collection's number holds a document that was checked and one that had no
            // frontmatter, and never a file matched twice, so together the collections hold
            // exactly the files that belong to one collection.
            let held: u64 = object["audit"]["collections"]
                .as_array()
                .unwrap()
                .iter()
                .map(|collection| collection["documents"].as_u64().unwrap())
                .sum();
            assert_eq!(
                held as usize,
                checked + no_frontmatter,
                "{group}/{name}: {object}"
            );
        }
    }
    assert!(checked_any, "no fixture project was found to check");
}

// --- `collections.empty` ---

#[test]
fn a_project_with_no_collections_at_all_is_collections_empty() {
    let project = Scratch::empty();
    project.file(".typdoc/config.json", r#"{ "version": 1 }"#);

    let ran = validate(&[], project.path());

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    let findings = ran.stdout_json()["findings"].as_array().unwrap().clone();
    assert_eq!(findings.len(), 1, "{findings:?}");
    let finding = &findings[0];
    assert_eq!(finding["rule"], json!("collections.empty"));
    assert_eq!(finding["level"], json!("warn"));
    assert_eq!(finding["path"], json!(".typdoc"));
    assert!(finding["namespace"].is_null(), "{finding}");
    assert!(finding["collection"].is_null(), "{finding}");
    assert!(finding["key"].is_null(), "{finding}");
    assert!(finding["field"].is_null(), "{finding}");
}

#[test]
fn a_project_with_a_collection_and_no_documents_leaves_collections_empty_silent() {
    let project = Scratch::project(&common::NOTES);

    let ran = validate(&[], project.path());

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    assert_eq!(ran.stdout_json()["findings"], json!([]));
}

#[test]
fn a_zero_collection_project_still_lets_list_and_get_run() {
    let project = Scratch::empty();
    project.file(".typdoc/config.json", r#"{ "version": 1 }"#);

    let listed = Spawn::args(["list", "--json"]).cwd(project.path()).run();
    assert_eq!(listed.code, 0, "{}", listed.stderr);
    assert_eq!(listed.stdout_json()["total"], json!(0));

    // No document exists, so this is the ordinary not-found exit, not a config error.
    let got = Spawn::args(["get", "anything.md", "--json"])
        .cwd(project.path())
        .run();
    assert_eq!(got.code, 5, "{}", got.stderr);
}
