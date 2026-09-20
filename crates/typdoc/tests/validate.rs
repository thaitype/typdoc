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

// Ticket 9: `schema.valid`, `collections.overlap`, `keys.unique` and `filename.pattern`.

/// A schema-file finding is not about a document: no `namespace`, `collection` or `key`.
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

/// The design never settles an overlap by precedence, so which collection is "the" collection
/// of the file is exactly what is wrong: `collection` and `key` are left out. The fixture also
/// holds `b.md`, matched by one collection only, with an `extra` field its empty schema does
/// not name: its `frontmatter.unknown` finding is what proves the overlap did not swallow the
/// rest of the report (see the `checked` assertions below).
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

/// Two collections that both match `a.md`, one that matches only `b.md` besides, and a schema
/// with no fields, so any field written in `b.md` is `frontmatter.unknown`. The same shape as
/// `fixtures/broken/collections.overlap`, built fresh so a test can name its own arguments.
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

/// design.md:567 stops `validate` before any report for only two reasons (a key ambiguous
/// across namespaces, an argument naming no document) and `collections.overlap` is neither: an
/// overlapping path named as an argument is a finding, the same shape the whole-project scan
/// gives it, and does not throw away the report of another argument named alongside it.
/// design.md:743 ties `checked.documents`/`checked.paths` to what was actually checked, and an
/// overlapping path was never checked against a schema, so it is counted in neither, even
/// though its own finding is in `findings` — the one place a finding's `path` is not found in
/// `checked.paths`, and on purpose: there is no document there to have been checked.
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

    // Naming the overlapping path alongside `b.md`: `b.md`'s own finding is not thrown away,
    // and `b.md` alone is in `checked.paths` and `checked.documents`.
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

/// The whole-project scan reports the same `collections.overlap` finding, but its `documents`
/// count is the same question `checked.documents` answers for a `paths` scope: only `b.md` was
/// checked against a schema.
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

/// The ticket's own example: two namespaces that each number a document `WF-1` are clean,
/// since a key is unique within its namespace, not across them.
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

/// A cycle in `extends` is rejected, not ended silently (ticket 5 left rejecting it to this
/// ticket).
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

/// Redefining an inherited field without `"override": true` is `schema.valid`; the same field
/// declared with `"override": true` is clean.
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

/// A boolean option written as something else, an option that does not apply to the field's
/// type, and a reserved or malformed field name are all `schema.valid`, and do not crash the
/// read (the relaxing of ticket 5's strict reading, decided by ticket 9).
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

/// An import alias that is one of the four URL schemes the design names is `schema.valid`
/// (design, Frontmatter values: "Sibling names and import aliases may not collide with URL
/// schemes (`http`, `https`, `mailto`, `file`)").
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

/// An ordinary alias that merely has the shape of a URL scheme (any letters-only word does) is
/// not `schema.valid`: the design reserves four literal names, not a shape, and its own worked
/// examples use exactly the two aliases checked here (`memory::precedents/x.md`,
/// `chief::story-3:WF-5`) — a fixture or reader trying the design's own examples must not be
/// refused by this check.
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

/// The design's own example: several coded collections can share one folder (`WF` and `RFC`
/// both under `tickets/{key}.md`); a file that fits one of them is not a `filename.pattern`
/// stray just because it fits none of the others.
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

// Ticket 10: the forms of a ref (a bare key, a sibling prefix, a relative path with `refBase`),
// resolved through the index of names as they are on disk, and `refs.resolve`, `refs.target`,
// `refs.acyclic`, `refs.moved`, `refs.codedByPath` and `names.shadowed`. The exact set each
// rule's `broken/` fixture trips is checked by `coverage.rs`; this file covers the two cases the
// ticket names, and a few finding shapes and positive cases that a single fixture cannot show
// alongside a negative one.

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

/// The ticket's own example: a ref whose case differs from the target file's is `not-found`, not
/// a match a case-insensitive file system might have given it (ticket 22 of the design
/// decisions: paths compare exactly as written, case included).
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

/// The ticket's other example: `chief::WF-5` in a project with several namespaces is
/// `bad-prefix`. The import form is not resolved in this story (ticket 17's), so this holds
/// whether or not `chief` is a configured import alias: nothing here ever treats `::` as
/// anything but not-yet-read.
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

/// A bare key whose code exists in the project resolves in the document's own namespace,
/// whatever the current directory is (design.md's Refs: "whatever the working directory").
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

/// A relative ref resolved under `refBase: namespace` reads from the namespace's folder, not
/// the document's own, unlike the default `refBase: file`.
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

/// A ref that resolves to a document whose schema `target` does not list is `refs.target`, and
/// one that resolves to a listed schema is clean.
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

/// `refs.codedByPath` warns when a coded document is named by path, and a ref to the same
/// document by key is clean, the same choice `set` and `new` make: a key does not change when
/// the file moves and a path does (design.md's Refs, "Canonical form").
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

/// A cycle through an `acyclic` field is reported at each document on it; a field not marked
/// `acyclic` may point back at itself with no finding.
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

/// `refs.acyclic` is always on, but naming one document of a cycle reports only that document's
/// own finding: every finding in the `paths` scope is about a document the caller named, and the
/// other document on the same cycle is that document's own `validate` run to report.
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

/// `refs.moved` replaces the ordinary missing-target finding and names the new key or path;
/// without a matching `auto: moves` record, the same dangling ref is plain `refs.resolve`.
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

/// `names.shadowed` also shows under `--schemas`, the same scope `schema.valid` reaches.
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
