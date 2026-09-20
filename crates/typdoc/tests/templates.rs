//! Match templates: which files a collection holds, for globs with `*` and `**` and for the
//! key placeholder of a coded schema.

#[allow(dead_code, reason = "each test file uses part of the shared helper")]
mod common;

use common::{NOTES, Ran, Scratch, Spawn, fixture};
use serde_json::{Value, json};

fn get(project: &std::path::Path, path: &str) -> Ran {
    Spawn::args(["get", path, "--json"]).cwd(project).run()
}

fn collection_of(ran: &Ran) -> Value {
    assert_eq!(ran.code, 0, "stderr: {}", ran.stderr);
    ran.stdout_json()["document"]["collection"].clone()
}

/// A project with one collection, `notes`, whose `match` is `pattern`, over the schema of
/// `NOTES`, and an empty document at each of `files`.
fn with_match(pattern: &str, files: &[&str]) -> Scratch {
    let project = Scratch::project(&NOTES);
    project.file(
        ".typdoc/collections/notes.json",
        &json!({ "match": pattern, "schema": "note.json" }).to_string(),
    );
    for file in files {
        project.file(file, "");
    }
    project
}

fn found(project: &Scratch, path: &str) -> bool {
    match get(project.path(), path).code {
        0 => true,
        5 => false,
        other => panic!("{path}: exit {other}"),
    }
}

#[test]
fn the_fixture_holds_what_each_template_matches_and_nothing_else() {
    let root = fixture("valid/templates");

    for (path, collection) in [
        ("README.md", "top"),
        ("notes/a.md", "notes"),
        ("notes/deep/b.md", "notes"),
        ("notes/deep/er/c.md", "notes"),
        ("tickets/WF-1.md", "tickets"),
        ("tickets/WF-12.md", "tickets"),
    ] {
        assert_eq!(
            collection_of(&get(&root, path)),
            json!(collection),
            "{path}"
        );
    }
    for path in [
        "notes/.hidden/d.md",
        "notes/.e.md",
        "tickets/README.md",
        "tickets/wf-2.md",
        "tickets/WF-3a.md",
        "a.md",
    ] {
        assert_eq!(get(&root, path).code, 5, "{path}");
    }
}

#[test]
fn a_star_stays_within_one_segment() {
    let project = with_match("notes/*.md", &["notes/a.md", "notes/deep/b.md", "a.md"]);

    assert!(found(&project, "notes/a.md"));
    assert!(!found(&project, "notes/deep/b.md"));
    assert!(!found(&project, "a.md"));
}

#[test]
fn a_folder_name_in_a_template_is_a_folder_the_walk_enters() {
    let project = with_match("a/b/c.md", &["a/b/c.md", "a/c.md", "b/c.md"]);

    assert!(found(&project, "a/b/c.md"));
    assert!(!found(&project, "a/c.md"));
    assert!(!found(&project, "b/c.md"));
}

#[test]
fn a_star_in_a_folder_segment_matches_the_folders_it_fits() {
    let project = with_match("*/x.md", &["a/x.md", "b/x.md", "x.md", "a/b/x.md"]);

    assert!(found(&project, "a/x.md"));
    assert!(found(&project, "b/x.md"));
    assert!(!found(&project, "x.md"));
    assert!(!found(&project, "a/b/x.md"));
}

#[test]
fn a_double_star_is_any_number_of_folders_and_none_is_one_of_them() {
    let project = with_match(
        "**/*.md",
        &["top.md", "a/one.md", "a/b/two.md", "a/b/c/three.md"],
    );

    for path in ["top.md", "a/one.md", "a/b/two.md", "a/b/c/three.md"] {
        assert!(found(&project, path), "{path}");
    }
}

#[test]
fn a_double_star_in_the_middle_of_a_template_keeps_the_names_around_it() {
    let project = with_match(
        "docs/**/index.md",
        &[
            "docs/index.md",
            "docs/a/index.md",
            "docs/a/b/index.md",
            "docs/a/other.md",
            "index.md",
        ],
    );

    for path in ["docs/index.md", "docs/a/index.md", "docs/a/b/index.md"] {
        assert!(found(&project, path), "{path}");
    }
    assert!(!found(&project, "docs/a/other.md"));
    assert!(!found(&project, "index.md"));
}

#[test]
fn a_double_star_at_the_end_is_every_file_below_the_folder() {
    let project = with_match(
        "docs/**",
        &[
            "docs/a.md",
            "docs/b/c.md",
            "docs/b/d/e.md",
            "other/f.md",
            "docs.md",
        ],
    );

    for path in ["docs/a.md", "docs/b/c.md", "docs/b/d/e.md"] {
        assert!(found(&project, path), "{path}");
    }
    assert!(!found(&project, "other/f.md"));
    assert!(!found(&project, "docs.md"));
}

#[test]
fn a_wildcard_never_matches_a_name_that_starts_with_a_dot() {
    let project = with_match(
        "**/*.md",
        &[
            ".hidden.md",
            ".folder/a.md",
            "a/.b/c.md",
            "a/.d.md",
            "a/e.md",
        ],
    );

    assert!(found(&project, "a/e.md"));
    for path in [".hidden.md", ".folder/a.md", "a/.b/c.md", "a/.d.md"] {
        assert!(!found(&project, path), "{path}");
    }
}

#[test]
fn a_name_written_with_a_dot_in_the_template_is_matched() {
    let project = with_match(".docs/*.md", &[".docs/a.md", ".docs/.b.md", "docs/a.md"]);

    assert!(found(&project, ".docs/a.md"));
    assert!(!found(&project, ".docs/.b.md"));
    assert!(!found(&project, "docs/a.md"));
}

#[test]
fn a_segment_that_starts_with_a_dot_reaches_the_hidden_names_it_fits() {
    let project = with_match(".*", &[".a", ".b.md", "c.md"]);

    assert!(found(&project, ".b.md"));
    assert!(!found(&project, "c.md"));
}

#[test]
fn a_folder_with_its_own_project_is_not_entered_and_its_files_belong_to_it() {
    let project = with_match("**/*.md", &["a.md", "sub/b.md", "inner/c.md"]);
    project.file("inner/.typdoc/config.json", r#"{ "version": 1 }"#);

    assert!(found(&project, "a.md"));
    assert!(found(&project, "sub/b.md"));
    assert!(!found(&project, "inner/c.md"));
}

#[test]
fn a_folder_with_a_typdoc_folder_and_no_config_is_not_a_project() {
    let project = with_match("**/*.md", &["inner/c.md"]);
    project.file("inner/.typdoc/collections/x.json", "{}");

    assert!(found(&project, "inner/c.md"));
}

#[test]
fn a_symbolic_link_to_a_folder_that_a_double_star_would_enter_is_refused() {
    let project = with_match("**/*.md", &["real/a.md"]);
    project.symlink("linked", "real");

    let ran = get(project.path(), "real/a.md");

    assert_eq!(ran.code, 6, "{}", ran.stderr);
}

#[test]
fn a_symbolic_link_to_a_file_that_the_template_does_not_match_is_left_alone() {
    let project = with_match("**/*.md", &["a.md", "real.txt"]);
    project.symlink("link.txt", "real.txt");

    assert!(found(&project, "a.md"));
}

#[test]
fn a_symbolic_link_to_a_file_that_the_template_matches_is_refused() {
    let project = with_match("**/*.md", &["real.txt"]);
    project.symlink("link.md", "real.txt");

    assert_eq!(get(project.path(), "link.md").code, 6);
}

#[test]
fn a_folder_whose_name_is_not_utf8_under_a_double_star_is_refused() {
    let project = with_match("**/*.md", &["a.md"]);
    project.file_named_by_bytes(b"\xff/b.md", "");

    let ran = get(project.path(), "a.md");

    assert_eq!(ran.code, 6, "{}", ran.stderr);
}

#[test]
fn a_file_two_collections_reach_is_an_overlap_however_they_reach_it() {
    let project = with_match("**/*.md", &["a/b.md"]);
    project.file(
        ".typdoc/collections/more.json",
        r#"{ "match": "a/*.md", "schema": "note.json" }"#,
    );

    let ran = get(project.path(), "a/b.md");

    assert_eq!(ran.code, 2, "{}", ran.stderr);
    assert!(ran.stderr.contains("`more`"), "{}", ran.stderr);
}

#[test]
fn the_case_of_a_name_in_a_template_counts() {
    let project = with_match("Notes/*.md", &["Notes/a.md"]);

    assert!(found(&project, "Notes/a.md"));
    assert!(!found(&project, "notes/a.md"));
}

#[test]
fn a_template_that_cannot_be_read_is_refused_and_names_the_collection_file() {
    for pattern in [
        "",
        "/a.md",
        "a/",
        "a//b.md",
        "./a.md",
        "../a.md",
        "a**b.md",
        "**a/b.md",
        "{other}.md",
        "a{b.md",
        "{key}.md",
    ] {
        let project = with_match(pattern, &["a.md"]);

        let ran = get(project.path(), "a.md");

        assert_eq!(ran.code, 2, "{pattern:?}: {}", ran.stderr);
        assert!(
            ran.stderr.contains("notes.json"),
            "{pattern:?}: {}",
            ran.stderr
        );
    }
}

fn coded(pattern: &str) -> Scratch {
    let project = Scratch::project(&[]);
    project.file(
        ".typdoc/collections/tickets.json",
        &json!({ "match": pattern, "schema": "ticket.json" }).to_string(),
    );
    project.file(
        "ticket.json",
        r#"{ "name": "ticket", "code": "WF", "fields": {} }"#,
    );
    for file in ["WF-1.md", "t/WF-2.md", "RFC-3.md", "WF-x.md"] {
        project.file(file, "");
    }
    project
}

#[test]
fn a_coded_schema_matches_its_code_a_dash_and_a_number_where_the_template_puts_the_key() {
    let root = coded("{key}.md");
    let nested = coded("t/{key}.md");

    assert!(found(&root, "WF-1.md"));
    assert!(!found(&root, "RFC-3.md"));
    assert!(!found(&root, "WF-x.md"));
    assert!(!found(&root, "t/WF-2.md"));
    assert!(found(&nested, "t/WF-2.md"));
    assert!(!found(&nested, "WF-1.md"));
}

#[test]
fn a_coded_schema_with_a_wildcard_or_no_key_or_two_is_refused() {
    for pattern in [
        "*.md",
        "**/{key}.md",
        "WF-1.md",
        "{key}/{key}.md",
        "{key}*.md",
    ] {
        let project = coded(pattern);

        let ran = get(project.path(), "WF-1.md");

        assert_eq!(ran.code, 2, "{pattern}: {}", ran.stderr);
        assert!(
            ran.stderr.contains("tickets.json"),
            "{pattern}: {}",
            ran.stderr
        );
    }
}
