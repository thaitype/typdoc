//! Covers SPC-2, SPC-7, SPC-17, SPC-18.
//!
//! Every argument is written by hand from SPC-2's table, never taken from typdoc's own output.

#[allow(dead_code, reason = "each test file uses part of the shared helper")]
mod common;

use std::path::Path;

use common::{Ran, Scratch, Spawn, fixture, slugged_projects};
use serde_json::json;

fn get(project: &Path, argument: &str) -> Ran {
    Spawn::args(["get", argument, "--json"]).cwd(project).run()
}

fn round_trips(project: &Path, argument: &str, path: &str) {
    let ran = get(project, argument);

    assert_eq!(ran.code, 0, "{argument}: {}", ran.stderr);
    assert_eq!(
        ran.stdout_json()["document"]["path"],
        json!(path),
        "{argument}"
    );
}

#[test]
fn in_one_namespace_the_path_form_and_the_bare_key_form_both_read_the_document() {
    let project = fixture("valid/templates");

    for (path, key) in [("tickets/WF-1.md", "WF-1"), ("tickets/WF-12.md", "WF-12")] {
        round_trips(&project, path, path);
        round_trips(&project, key, path);
    }

    // An uncoded document has a path and no key at all.
    round_trips(&project, "notes/a.md", "notes/a.md");
}

#[test]
fn in_several_namespaces_the_path_form_takes_no_prefix_and_the_key_form_needs_one() {
    let project = fixture("valid/several-namespaces");

    round_trips(
        &project,
        "story-1/tickets/WF-1.md",
        "story-1/tickets/WF-1.md",
    );
    round_trips(&project, "story-1:WF-1", "story-1/tickets/WF-1.md");
    round_trips(
        &project,
        "story-2/tickets/WF-9.md",
        "story-2/tickets/WF-9.md",
    );
    round_trips(&project, "story-2:WF-9", "story-2/tickets/WF-9.md");
}

/// As in a ref: `namespace:path` is read from that namespace's folder.
#[test]
fn a_namespace_prefix_on_a_path_reads_the_path_from_that_namespaces_folder() {
    let project = fixture("valid/several-namespaces");

    round_trips(
        &project,
        "story-2:tickets/WF-9.md",
        "story-2/tickets/WF-9.md",
    );

    // The folder written twice: `story-2/story-2/tickets/WF-9.md`, which names nothing.
    let missing = get(&project, "story-2:story-2/tickets/WF-9.md");
    assert_eq!(missing.code, 5, "{}", missing.stderr);

    let unknown = get(&project, "nosuch:tickets/WF-9.md");
    assert_eq!(unknown.code, 1, "{}", unknown.stderr);
    assert!(
        unknown.stderr_json()["error"]
            .as_str()
            .unwrap()
            .contains("nosuch"),
        "{}",
        unknown.stderr
    );
}

/// A prefix names one namespace exactly; `--namespace` is where a glob or a list selects several.
#[test]
fn a_namespace_prefix_is_never_a_glob_or_a_list() {
    let project = fixture("valid/several-namespaces");

    for argument in [
        "story-*:WF-9",
        "story-1,story-2:WF-9",
        "story-*:tickets/WF-9.md",
    ] {
        let ran = get(&project, argument);

        assert_eq!(ran.code, 1, "{argument}: {}", ran.stderr);
        assert!(
            ran.stderr_json()["error"]
                .as_str()
                .unwrap()
                .contains("`--namespace` selects several"),
            "{argument}: {}",
            ran.stderr
        );
    }
    let selected = Spawn::args(["get", "WF-9", "--namespace", "story-*", "--json"])
        .cwd(&project)
        .run();
    assert_eq!(selected.code, 0, "{}", selected.stderr);
}

#[test]
fn a_key_no_document_has_says_so_by_its_key() {
    let project = fixture("valid/several-namespaces");

    for argument in ["story-2:WF-99", "XX-1"] {
        let ran = get(&project, argument);

        assert_eq!(ran.code, 5, "{argument}: {}", ran.stderr);
        assert_eq!(
            ran.stderr_json()["error"],
            json!(format!("no document with key {argument}")),
            "{argument}"
        );
    }
}

#[test]
fn an_unknown_double_colon_prefix_is_bad_arguments_whether_it_prefixes_a_key_or_a_path() {
    let project = fixture("valid/several-namespaces");

    for argument in ["chief::WF-9", "chief::story-2/tickets/WF-9.md"] {
        let ran = get(&project, argument);

        assert_eq!(ran.code, 1, "{argument}: {}", ran.stderr);
    }
}

#[test]
fn with_an_import_the_path_form_takes_no_namespace_and_the_key_form_needs_one_past_the_first() {
    let project = fixture("valid/imports/main");

    // `memory_import` has one namespace, `default`.
    round_trips(
        &project,
        "memory_import::learnings/LRN-1.md",
        "learnings/LRN-1.md",
    );
    round_trips(&project, "memory_import::LRN-1", "learnings/LRN-1.md");

    // `several_import` has several.
    round_trips(
        &project,
        "several_import::story-1/tickets/WF-1.md",
        "story-1/tickets/WF-1.md",
    );
    round_trips(
        &project,
        "several_import::story-2/tickets/WF-9.md",
        "story-2/tickets/WF-9.md",
    );
    round_trips(
        &project,
        "several_import::story-1:WF-1",
        "story-1/tickets/WF-1.md",
    );
    round_trips(
        &project,
        "several_import::story-2:WF-9",
        "story-2/tickets/WF-9.md",
    );

    // `named_import` has one namespace, and it is not called `default`: it is `only`.
    round_trips(&project, "named_import::only/notes/a.md", "only/notes/a.md");
}

/// `WF-5`'s file is `WF-5-json-shapes.md` in both projects, so every slug written below is out of
/// date, and the output has to be exactly the key's.
#[test]
fn a_key_written_with_a_slug_names_the_key_in_every_form_and_a_stale_slug_prints_nothing_more() {
    let projects = slugged_projects();
    let main = projects.path().join("main");

    for (with_slug, key, path) in [
        (
            "WF-5-old-name",
            "story-2:WF-5",
            "story-2/tickets/WF-5-json-shapes.md",
        ),
        (
            "story-2:WF-5-old-name",
            "story-2:WF-5",
            "story-2/tickets/WF-5-json-shapes.md",
        ),
        (
            "chief::story-3:WF-5-old-name",
            "chief::story-3:WF-5",
            "story-3/tickets/WF-5-json-shapes.md",
        ),
    ] {
        let by_slug = get(&main, with_slug);
        let by_key = get(&main, key);

        assert_eq!(by_slug.code, 0, "{with_slug}: {}", by_slug.stderr);
        assert_eq!(by_slug.stderr, "", "{with_slug}");
        assert_eq!(by_slug.stdout, by_key.stdout, "{with_slug}");
        let document = &by_slug.stdout_json()["document"];
        assert_eq!(document["key"], json!("WF-5"), "{with_slug}");
        assert_eq!(document["path"], json!(path), "{with_slug}");

        let text = Spawn::args(["get", with_slug]).cwd(&main).run();
        let key_text = Spawn::args(["get", key]).cwd(&main).run();
        assert_eq!(text.code, 0, "{with_slug}: {}", text.stderr);
        assert_eq!(text.stderr, "", "{with_slug}");
        assert_eq!(text.stdout, key_text.stdout, "{with_slug}");
    }
}

/// What follows the digits has to be a slug for the argument to have the form of a key;
/// otherwise it is neither a key nor, since it does not end in `.md`, a path.
#[test]
fn a_key_followed_by_text_that_is_not_a_slug_is_bad_arguments() {
    let projects = slugged_projects();
    let main = projects.path().join("main");

    for argument in [
        "story-2:WF-5-",
        "story-2:WF-5-a#b",
        "story-2:WF-5-a b",
        "story-2:WF-5x",
    ] {
        let ran = get(&main, argument);

        assert_eq!(ran.code, 1, "{argument}: {}", ran.stderr);
    }
}

/// Every command that takes a path reads a prefix that way, a destination and a new document
/// included.
#[test]
fn new_and_mv_read_a_prefixed_path_from_the_namespaces_folder() {
    let projects = slugged_projects();
    let main = projects.path().join("main");

    let created = Spawn::args(["new", "story-2:notes/n.md", "--json"])
        .cwd(&main)
        .run();
    assert_eq!(created.code, 0, "{}", created.stderr);
    assert_eq!(
        created.stdout_json()["document"]["path"],
        json!("story-2/notes/n.md")
    );

    let moved = Spawn::args(["mv", "story-2:notes/n.md", "story-1:notes/m.md", "--json"])
        .cwd(&main)
        .run();
    assert_eq!(moved.code, 0, "{}", moved.stderr);
    assert!(main.join("story-1/notes/m.md").is_file());
    assert!(!main.join("story-2/notes/n.md").exists());
}

/// A project with one namespace calls it `default` in a prefix, whatever its name.
#[test]
fn default_names_the_one_namespace_of_a_project_whatever_it_is_called() {
    let project = Scratch::project(&[
        (
            ".typdoc/config.json",
            r#"{ "version": 1, "namespaces": ["docs"] }"#,
        ),
        (
            ".typdoc/collections/notes.json",
            r#"{ "match": "notes/*.md", "schema": "note.json" }"#,
        ),
        ("note.json", r#"{ "name": "note", "fields": {} }"#),
        ("docs/notes/a.md", "---\n---\n"),
    ]);

    round_trips(project.path(), "default:notes/a.md", "docs/notes/a.md");
    round_trips(project.path(), "docs:notes/a.md", "docs/notes/a.md");
}

/// `default` holds through an import too: `alias::default:` names the import's one namespace.
#[test]
fn default_through_an_import_names_its_one_namespace_whatever_it_is_called() {
    let projects = Scratch::empty();
    projects.file(
        "main/.typdoc/config.json",
        r#"{ "version": 1, "imports": { "mem": "../mem" } }"#,
    );
    projects.file(
        "mem/.typdoc/config.json",
        r#"{ "version": 1, "namespaces": ["docs"] }"#,
    );
    projects.file(
        "mem/.typdoc/collections/notes.json",
        r#"{ "match": "notes/*.md", "schema": "note.json" }"#,
    );
    projects.file("mem/note.json", r#"{ "name": "note", "fields": {} }"#);
    projects.file("mem/docs/notes/y.md", "---\n---\n");
    let main = projects.path().join("main");

    for argument in [
        "mem::default:notes/y.md",
        "mem::docs:notes/y.md",
        "mem::docs/notes/y.md",
    ] {
        let ran = get(&main, argument);

        assert_eq!(ran.code, 0, "{argument}: {}", ran.stderr);
        assert_eq!(
            ran.stdout_json()["document"]["path"],
            json!("docs/notes/y.md"),
            "{argument}"
        );
    }
}

/// A namespace name holds no `/`, so a colon after one is part of the file name.
#[test]
#[cfg_attr(windows, ignore = "a file name on Windows cannot hold a colon")]
fn new_takes_a_path_with_a_colon_after_a_folder_as_it_is_written() {
    let projects = slugged_projects();
    let main = projects.path().join("main");

    let created = Spawn::args(["new", "story-2/notes/c:d.md", "--json"])
        .cwd(&main)
        .run();

    assert_eq!(created.code, 0, "{}", created.stderr);
    assert_eq!(
        created.stdout_json()["document"]["path"],
        json!("story-2/notes/c:d.md")
    );
}
