//! Covers SPC-2 on Windows: a project-relative path argument written with `\` names the same
//! document as the one written with `/`, with or without a `namespace:` or `project::` prefix, in
//! every command that takes a document or a path to create.
//!
//! Windows only: on Unix `\` is a character a file name may hold.

#![cfg(windows)]

#[allow(dead_code, reason = "each test file uses part of the shared helper")]
mod common;

use common::{Ran, Scratch, Spawn, fixture};
use serde_json::json;

fn get(argument: &str, project: &str) -> Ran {
    Spawn::args(["get", argument, "--json"])
        .cwd(fixture(project))
        .run()
}

#[test]
fn a_project_path_with_backslashes_names_the_document_the_slashed_one_does() {
    for argument in [r"story-1/tickets/WF-1.md", r"story-1\tickets\WF-1.md"] {
        let ran = get(argument, "valid/several-namespaces");

        assert_eq!(ran.code, 0, "{argument}: {}", ran.stderr);
        assert_eq!(
            ran.stdout_json()["document"]["path"],
            json!("story-1/tickets/WF-1.md"),
            "{argument}"
        );
    }
}

#[test]
fn a_namespace_prefixed_path_with_backslashes_names_the_document_the_slashed_one_does() {
    // The path after the prefix names its namespace folder too (SPC-2).
    for argument in [
        r"story-1:story-1/tickets/WF-1.md",
        r"story-1:story-1\tickets\WF-1.md",
    ] {
        let ran = get(argument, "valid/several-namespaces");

        assert_eq!(ran.code, 0, "{argument}: {}", ran.stderr);
        let document = ran.stdout_json()["document"].clone();
        assert_eq!(
            document["path"],
            json!("story-1/tickets/WF-1.md"),
            "{argument}"
        );
        assert_eq!(document["namespace"], json!("story-1"), "{argument}");
    }
}

#[test]
fn a_project_prefixed_path_with_backslashes_names_the_document_the_slashed_one_does() {
    for argument in [
        r"memory_import::learnings/LRN-1.md",
        r"memory_import::learnings\LRN-1.md",
    ] {
        let ran = get(argument, "valid/imports/main");

        assert_eq!(ran.code, 0, "{argument}: {}", ran.stderr);
        let document = ran.stdout_json()["document"].clone();
        assert_eq!(document["path"], json!("learnings/LRN-1.md"), "{argument}");
        assert_eq!(document["project"], json!("memory_import"), "{argument}");
    }
}

/// One collection of `notes/*.md` whose schema has a `title`.
fn notes() -> Scratch {
    let project = Scratch::project(&[
        (
            ".typdoc/collections/notes.json",
            r#"{ "match": "notes/*.md", "schema": "note.json" }"#,
        ),
        (
            "note.json",
            r#"{ "name": "note", "fields": { "title": { "type": "string" } } }"#,
        ),
    ]);
    project.file("notes/a.md", "---\ntitle: A\n---\n\n# A\n");
    project
}

fn run(project: &Scratch, args: &[&str]) -> Ran {
    Spawn::args(args.iter().copied()).cwd(project.path()).run()
}

#[test]
fn set_takes_a_path_written_with_backslashes() {
    let project = notes();

    let ran = run(&project, &["set", r"notes\a.md", "title=B"]);

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    assert_eq!(project.read("notes/a.md"), "---\ntitle: B\n---\n\n# A\n");
}

#[test]
fn toc_refs_and_validate_take_a_path_written_with_backslashes() {
    let project = notes();

    for args in [
        &["toc", r"notes\a.md", "--json"][..],
        &["refs", r"notes\a.md", "--json"][..],
        &["validate", r"notes\a.md", "--json"][..],
    ] {
        let ran = run(&project, args);

        assert_eq!(ran.code, 0, "{args:?}: {}", ran.stderr);
    }
}

#[test]
fn new_creates_the_path_written_with_backslashes_as_its_slashed_path() {
    let project = notes();

    let ran = run(
        &project,
        &["new", r"notes\b.md", "--set", "title=B", "--json"],
    );

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    assert_eq!(ran.stdout_json()["document"]["path"], json!("notes/b.md"));
    assert!(project.path().join("notes").join("b.md").is_file());
}

#[test]
fn mv_takes_both_paths_written_with_backslashes() {
    let project = notes();

    let ran = run(&project, &["mv", r"notes\a.md", r"notes\c.md", "--json"]);

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    assert!(project.path().join("notes").join("c.md").is_file());
    assert!(!project.path().join("notes").join("a.md").exists());
}

#[test]
fn mv_renumber_takes_a_source_written_with_backslashes() {
    let project = Scratch::project(&[
        (
            ".typdoc/config.json",
            r#"{ "version": 1, "namespaces": ["story-1", "story-3"] }"#,
        ),
        (
            ".typdoc/collections/tickets.json",
            r#"{ "match": "tickets/{key}.md", "schema": "ticket.json" }"#,
        ),
        (
            "ticket.json",
            r#"{ "name": "ticket", "code": "WF", "fields": { "title": { "type": "string" } } }"#,
        ),
    ]);
    project.file("story-1/tickets/WF-5.md", "---\ntitle: One\n---\n");
    project.file("story-3/.keep", "");

    let ran = run(
        &project,
        &[
            "mv",
            r"story-1\tickets\WF-5.md",
            "--renumber",
            "story-3",
            "--json",
        ],
    );

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    assert!(
        !project
            .path()
            .join("story-1")
            .join("tickets")
            .join("WF-5.md")
            .exists()
    );
    assert!(
        project
            .path()
            .join("story-3")
            .join("tickets")
            .join("WF-1.md")
            .is_file()
    );
}
