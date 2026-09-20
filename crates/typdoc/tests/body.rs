//! `body.links`, `body.anchors` and `body.mentions` through the built binary: message wording,
//! positions, the `ignore` option, and the cases ticket 10 of the decisions names by name that
//! the coarse `trips` set of `fixtures/broken/` cannot show on their own (`version 1.2` not
//! reported, an undefined `[t][ref]` not reported, a definition inside code not a definition).
//! The exact set each `broken/` fixture trips is checked by `coverage.rs`.

#[allow(dead_code, reason = "each test file uses part of the shared helper")]
mod common;

use common::{NOTES, Scratch, Spawn, fixture};
use serde_json::json;

fn validate(cwd: &std::path::Path) -> common::Ran {
    Spawn::args(["validate", "--json"]).cwd(cwd).run()
}

#[test]
fn the_clean_body_links_fixture_validates_with_no_findings() {
    // Holds, in one real project, every case ticket 10 of the decisions names by name that must
    // not be reported: `version 1.2`, an undefined `[t][ref]`, and a definition inside a fenced
    // code block: see fixtures/valid/body-links/clean.md.
    let project = fixture("valid/body-links");

    let ran = validate(&project);

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    let object = ran.stdout_json();
    assert_eq!(object["findings"], json!([]), "{object}");
    assert_eq!(object["summary"]["checked"]["documents"], json!(2));
}

#[test]
fn a_broken_inline_link_names_the_written_destination_at_its_opening_bracket() {
    let project = Scratch::project(&NOTES);
    project.file("note.md", "---\n---\n\n[t](missing.md)\n");

    let ran = validate(project.path());

    assert_eq!(ran.code, 2, "{}", ran.stderr);
    let object = ran.stdout_json();
    let findings = object["findings"].as_array().unwrap();
    assert_eq!(findings.len(), 1, "{object}");
    assert_eq!(findings[0]["rule"], json!("body.links"));
    assert_eq!(
        findings[0]["message"],
        json!("link target missing: missing.md")
    );
    assert_eq!(findings[0]["line"], json!(4));
    assert_eq!(findings[0]["col"], json!(1));
}

#[test]
fn a_broken_image_is_reported_at_its_bang() {
    let project = Scratch::project(&NOTES);
    project.file("note.md", "---\n---\n\n![alt](missing.png)\n");

    let ran = validate(project.path());

    let object = ran.stdout_json();
    let findings = object["findings"].as_array().unwrap();
    assert_eq!(findings.len(), 1, "{object}");
    assert_eq!(findings[0]["col"], json!(1));
    assert_eq!(
        findings[0]["message"],
        json!("link target missing: missing.png")
    );
}

#[test]
fn a_broken_reference_definition_is_reported_once_with_its_use_count() {
    let project = Scratch::project(&NOTES);
    project.file(
        "note.md",
        "---\n---\n\n[a][ref] [b][ref] [c][ref]\n\n[ref]: missing.md\n",
    );

    let ran = validate(project.path());

    let object = ran.stdout_json();
    let findings = object["findings"].as_array().unwrap();
    assert_eq!(findings.len(), 1, "{object}");
    assert_eq!(
        findings[0]["message"],
        json!("link target missing: missing.md (used 3 times)")
    );
    assert_eq!(findings[0]["line"], json!(6));
}

#[test]
fn a_label_defined_twice_reports_only_the_later_definition_and_checks_the_first() {
    let project = Scratch::project(&NOTES);
    project.file("a.md", "---\n---\n");
    project.file(
        "note.md",
        "---\n---\n\n[t][ref]\n\n[ref]: a.md\n[ref]: also-missing.md\n",
    );

    let ran = validate(project.path());

    let object = ran.stdout_json();
    let findings = object["findings"].as_array().unwrap();
    assert_eq!(
        findings.len(),
        1,
        "{object}: a.md exists, so only the duplicate is a finding"
    );
    assert_eq!(findings[0]["rule"], json!("body.links"));
    assert_eq!(
        findings[0]["message"],
        json!("already defined at line 6; this definition is ignored")
    );
    assert_eq!(findings[0]["line"], json!(7));
}

#[test]
fn the_ignore_option_skips_a_matching_destination_entirely() {
    let config = [
        (
            ".typdoc/config.json",
            r#"{ "version": 1, "validation": { "global": {
                "body.links": { "level": "error", "ignore": ["assets/**"] } } } }"#,
        ),
        (
            ".typdoc/collections/notes.json",
            r#"{ "match": "*.md", "schema": "note.json" }"#,
        ),
        ("note.json", r#"{ "name": "note", "fields": {} }"#),
    ];
    let project = Scratch::project(&config);
    project.file("note.md", "---\n---\n\n[img](assets/missing.png)\n");

    let ran = validate(project.path());

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    assert_eq!(ran.stdout_json()["findings"], json!([]));
}

/// A collection that states only a rule's `level` still keeps the global `ignore` for that same
/// rule (design line 618: "A collection file merges key by key, so it states only what
/// differs" — read as holding for the options inside one rule's setting, not only for which rule
/// names a collection's `validation` states at all). The control beside it, a collection that
/// states no `validation` at all, already passed before this fix and stays green, so a later
/// change cannot quietly reintroduce the wholesale-replace behaviour without this pair catching
/// it either way.
#[test]
fn a_collections_own_level_for_a_rule_does_not_discard_the_globals_options_for_it() {
    let files = [
        (
            ".typdoc/config.json",
            r#"{ "version": 1, "validation": { "global": {
                "body.links": { "level": "error", "ignore": ["assets/**"] } } } }"#,
        ),
        (
            ".typdoc/collections/drafts.json",
            r#"{ "match": "drafts/*.md", "schema": "note.json",
                "validation": { "body.links": { "level": "warn" } } }"#,
        ),
        ("note.json", r#"{ "name": "note", "fields": {} }"#),
    ];
    let project = Scratch::project(&files);
    project.file(
        "drafts/d.md",
        "---\n---\n\nSee [a](../assets/missing.md).\n",
    );

    let ran = validate(project.path());

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    assert_eq!(
        ran.stdout_json()["findings"],
        json!([]),
        "the collection only restates `level`; the global `ignore` for the same rule must still apply"
    );
}

#[test]
fn control_a_collection_that_states_no_validation_at_all_keeps_the_globals_ignore() {
    let files = [
        (
            ".typdoc/config.json",
            r#"{ "version": 1, "validation": { "global": {
                "body.links": { "level": "error", "ignore": ["assets/**"] } } } }"#,
        ),
        (
            ".typdoc/collections/drafts.json",
            r#"{ "match": "drafts/*.md", "schema": "note.json" }"#,
        ),
        ("note.json", r#"{ "name": "note", "fields": {} }"#),
    ];
    let project = Scratch::project(&files);
    project.file(
        "drafts/d.md",
        "---\n---\n\nSee [a](../assets/missing.md).\n",
    );

    let ran = validate(project.path());

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    assert_eq!(ran.stdout_json()["findings"], json!([]));
}

#[test]
fn a_self_anchor_with_no_matching_heading_is_body_anchors_not_body_links() {
    let project = Scratch::project(&NOTES);
    project.file("note.md", "---\n---\n\n# Heading\n\n[t](#missing)\n");

    let ran = validate(project.path());

    let object = ran.stdout_json();
    let findings = object["findings"].as_array().unwrap();
    assert_eq!(findings.len(), 1, "{object}");
    assert_eq!(findings[0]["rule"], json!("body.anchors"));
    assert_eq!(findings[0]["line"], json!(6));
    assert_eq!(findings[0]["col"], json!(1));
}

#[test]
fn an_anchor_that_matches_its_headings_slug_is_clean() {
    let project = Scratch::project(&NOTES);
    project.file("note.md", "---\n---\n\n# My Heading\n\n[t](#my-heading)\n");

    let ran = validate(project.path());

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    assert_eq!(ran.stdout_json()["findings"], json!([]));
}

const CODED: [(&str, &str); 4] = [
    (
        ".typdoc/config.json",
        r#"{ "version": 1, "validation": { "global": { "body.mentions": { "level": "error" } } } }"#,
    ),
    (
        ".typdoc/collections/tickets.json",
        r#"{ "match": "tickets/{key}.md", "schema": "wf.json" }"#,
    ),
    ("wf.json", r#"{ "name": "wf", "code": "WF", "fields": {} }"#),
    ("tickets/WF-1.md", "---\n---\n"),
];

#[test]
fn a_mention_that_does_not_resolve_names_the_written_token() {
    let project = Scratch::project(&CODED);
    project.file("tickets/WF-2.md", "---\n---\n\nSee WF-9 for context.\n");

    let ran = validate(project.path());

    let object = ran.stdout_json();
    let findings = object["findings"].as_array().unwrap();
    assert_eq!(findings.len(), 1, "{object}");
    assert_eq!(findings[0]["rule"], json!("body.mentions"));
    assert_eq!(findings[0]["message"], json!("WF-9 not found"));
    assert_eq!(findings[0]["line"], json!(4));
}

#[test]
fn a_mention_whose_code_is_not_registered_anywhere_is_never_checked() {
    let project = Scratch::project(&CODED);
    project.file("tickets/WF-2.md", "---\n---\n\nEncoded as UTF-8 today.\n");

    let ran = validate(project.path());

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    assert_eq!(ran.stdout_json()["findings"], json!([]));
}

#[test]
fn a_mention_that_resolves_is_clean() {
    let project = Scratch::project(&CODED);
    project.file("tickets/WF-2.md", "---\n---\n\nSee WF-1 for context.\n");

    let ran = validate(project.path());

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    assert_eq!(ran.stdout_json()["findings"], json!([]));
}

#[test]
fn a_broken_body_link_that_matches_a_recorded_move_is_refs_moved_not_body_links() {
    let files = [
        (
            ".typdoc/collections/notes.json",
            r#"{ "match": "*.md", "schema": "note.json" }"#,
        ),
        (
            "note.json",
            r#"{ "name": "note", "fields": {
                "moved_from": { "type": "list", "auto": "moves" }
            } }"#,
        ),
    ];
    let project = Scratch::project(&files);
    project.file("new.md", "---\nmoved_from: [old.md]\n---\n");
    project.file("b.md", "---\n---\n\n[t](old.md)\n");

    let ran = validate(project.path());

    assert_eq!(ran.code, 2, "{}", ran.stderr);
    let object = ran.stdout_json();
    let findings = object["findings"].as_array().unwrap();
    assert_eq!(findings.len(), 1, "{object}");
    assert_eq!(findings[0]["rule"], json!("refs.moved"));
    assert_eq!(findings[0]["path"], json!("b.md"));
    assert!(
        findings[0]["message"].as_str().unwrap().contains("new.md"),
        "{object}"
    );
}

#[test]
fn a_broken_body_link_with_an_anchor_still_matches_a_recorded_move() {
    // `auto: moves` records a plain path, never a fragment; the lookup must strip `#section`
    // from the written destination before matching, or a moved target written with an anchor
    // would silently fall through to `body.links` instead of `refs.moved`.
    let files = [
        (
            ".typdoc/collections/notes.json",
            r#"{ "match": "*.md", "schema": "note.json" }"#,
        ),
        (
            "note.json",
            r#"{ "name": "note", "fields": {
                "moved_from": { "type": "list", "auto": "moves" }
            } }"#,
        ),
    ];
    let project = Scratch::project(&files);
    project.file("new.md", "---\nmoved_from: [old.md]\n---\n\n# Section\n");
    project.file("b.md", "---\n---\n\n[t](old.md#section)\n");

    let ran = validate(project.path());

    assert_eq!(ran.code, 2, "{}", ran.stderr);
    let object = ran.stdout_json();
    let findings = object["findings"].as_array().unwrap();
    assert_eq!(findings.len(), 1, "{object}");
    assert_eq!(findings[0]["rule"], json!("refs.moved"));
    assert_eq!(findings[0]["path"], json!("b.md"));
    assert!(
        findings[0]["message"]
            .as_str()
            .unwrap()
            .contains("old.md#section"),
        "the message still shows what was written, anchor included: {object}"
    );
    assert!(
        findings[0]["message"].as_str().unwrap().contains("new.md"),
        "{object}"
    );
}

#[test]
fn a_mention_that_matches_a_recorded_move_is_refs_moved_not_body_mentions() {
    let files = [
        (
            ".typdoc/config.json",
            r#"{ "version": 1, "validation": { "global": { "body.mentions": { "level": "error" } } } }"#,
        ),
        (
            ".typdoc/collections/tickets.json",
            r#"{ "match": "tickets/{key}.md", "schema": "wf.json" }"#,
        ),
        (
            "wf.json",
            r#"{ "name": "wf", "code": "WF", "fields": {
                "moved_from": { "type": "list", "auto": "moves" }
            } }"#,
        ),
        ("tickets/WF-2.md", "---\nmoved_from: [WF-1]\n---\n"),
    ];
    let project = Scratch::project(&files);
    project.file("tickets/WF-3.md", "---\n---\n\nSee WF-1 for context.\n");

    let ran = validate(project.path());

    assert_eq!(ran.code, 2, "{}", ran.stderr);
    let object = ran.stdout_json();
    let findings = object["findings"].as_array().unwrap();
    assert_eq!(findings.len(), 1, "{object}");
    assert_eq!(findings[0]["rule"], json!("refs.moved"));
    assert_eq!(findings[0]["path"], json!("tickets/WF-3.md"));
    assert!(
        findings[0]["message"].as_str().unwrap().contains("WF-2"),
        "{object}"
    );
}
