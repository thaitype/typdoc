//! A ref or a body link that reaches a file outside every namespace folder: it resolves, the
//! command goes on, and the file is named by its path alone (no `namespace` in `--json`).
//! Every project here is built in a scratch folder, and every expected value is written out by
//! hand from the design (Namespaces, Refs, JSON output).

#[allow(dead_code, reason = "each test file uses part of the shared helper")]
mod common;

use common::{Ran, Scratch, Spawn};
use serde_json::{Value, json};

const COLLECTION: &str = r#"{ "match": "*.md", "schema": "schemas/n.json" }"#;
const SCHEMA: &str = r#"{ "name": "n", "fields": { "up": { "type": "ref", "target": "*" } } }"#;

/// Two namespaces and a `README.md` at the project root, which belongs to neither. `story-1/a.md`
/// refers to the README, and `story-2/b.md` refers to `story-1/a.md`.
fn two_namespaces(extra: &[(&str, &str)]) -> Scratch {
    let project = Scratch::empty();
    project.file(
        ".typdoc/config.json",
        r#"{ "version": 1, "namespaces": ["story-1", "story-2"] }"#,
    );
    project.file(".typdoc/collections/notes.json", COLLECTION);
    project.file("schemas/n.json", SCHEMA);
    project.file("README.md", "# Root\n");
    project.file("story-1/a.md", "---\nup: ../README.md\n---\n\nbody\n");
    project.file("story-2/b.md", "---\nup: ../story-1/a.md\n---\n\nbody\n");
    for (path, text) in extra {
        project.file(path, text);
    }
    project
}

fn run(project: &std::path::Path, args: &[&str]) -> Ran {
    Spawn::args(args.iter().copied()).cwd(project).run()
}

/// The run went well: exit 0, nothing on stderr. Returns what it printed.
fn ok(ran: &Ran) -> Value {
    assert_eq!(ran.code, 0, "stderr: {}", ran.stderr);
    assert_eq!(ran.stderr, "");
    ran.stdout_json()
}

fn listed(project: &std::path::Path, condition: &str) -> Vec<String> {
    let ran = run(project, &["list", "--where", condition, "--json"]);
    let out = ok(&ran);
    let mut paths: Vec<String> = out["documents"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["path"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(out["total"], json!(paths.len()), "{condition}");
    paths.sort();
    paths
}

#[test]
fn refs_names_a_root_file_reached_by_a_frontmatter_ref_by_its_path_alone() {
    let project = two_namespaces(&[]);

    let out = ok(&run(project.path(), &["refs", "story-1/a.md", "--json"]));

    assert_eq!(
        out["document"],
        json!({ "path": "story-1/a.md", "namespace": "story-1" })
    );
    assert_eq!(out["direction"], json!("out"));
    assert_eq!(
        out["refs"],
        json!([{ "path": "README.md", "field": "up", "written": "../README.md" }])
    );
}

#[test]
fn refs_names_a_root_file_reached_by_a_body_link_by_its_path_alone() {
    let project = two_namespaces(&[("story-1/c.md", "---\n---\n[root](../README.md)\n")]);

    let out = ok(&run(project.path(), &["refs", "story-1/c.md", "--json"]));

    assert_eq!(
        out["refs"],
        json!([{
            "path": "README.md",
            "field": "$body",
            "written": "../README.md",
            "line": 3,
            "col": 1
        }])
    );
}

#[test]
fn refs_reverse_finds_the_holder_while_another_document_refers_to_a_root_file() {
    let project = two_namespaces(&[]);

    let out = ok(&run(
        project.path(),
        &["refs", "story-1/a.md", "--reverse", "--json"],
    ));

    assert_eq!(out["direction"], json!("in"));
    assert_eq!(
        out["refs"],
        json!([{
            "path": "story-2/b.md",
            "namespace": "story-2",
            "field": "up",
            "written": "../story-1/a.md"
        }])
    );
}

#[test]
fn ref_any_goes_on_past_a_ref_to_a_root_file() {
    let project = two_namespaces(&[]);

    assert_eq!(
        listed(project.path(), "ref.any(up)"),
        ["story-1/a.md", "story-2/b.md"]
    );
    assert_eq!(
        listed(project.path(), "ref.any(up).path=README.md"),
        ["story-1/a.md"]
    );
}

#[test]
fn ref_any_on_a_body_link_goes_on_past_a_link_to_a_root_file() {
    let project = two_namespaces(&[("story-1/c.md", "---\n---\n[root](../README.md)\n")]);

    assert_eq!(listed(project.path(), "ref.any($body)"), ["story-1/c.md"]);
}

#[test]
fn ref_all_goes_on_past_a_ref_to_a_root_file() {
    let project = two_namespaces(&[]);

    assert_eq!(
        listed(project.path(), "ref.all(up).path=README.md"),
        ["story-1/a.md"]
    );
    assert_eq!(
        listed(project.path(), "ref.all(up).path=story-1/a.md"),
        ["story-2/b.md"]
    );
}

#[test]
fn a_root_file_has_no_namespace_for_a_condition_on_the_document_it_reaches() {
    let project = two_namespaces(&[]);

    // Absent fails `=` in every form, `*` included, and satisfies `!=`. (The query language reads
    // an empty value the same way, so a unit test in `typdoc-core` pins that the pseudo-field
    // holds no value at all for such a file.)
    assert_eq!(
        listed(project.path(), "ref.any(up).namespace=*"),
        ["story-2/b.md"]
    );
    assert_eq!(
        listed(project.path(), "ref.any(up).namespace!=*"),
        ["story-1/a.md"]
    );
    assert_eq!(
        listed(project.path(), "ref.any(up).namespace!=story-1"),
        ["story-1/a.md"]
    );
}

#[test]
fn refby_any_reports_the_referring_document_while_another_refers_to_a_root_file() {
    let project = two_namespaces(&[("story-1/d.md", "---\n---\n")]);

    // a.md is referred to by b.md; b.md and d.md (same namespace as a.md) by nobody; the README
    // is no candidate of `list`.
    assert_eq!(listed(project.path(), "refby.any(up)"), ["story-1/a.md"]);
    assert_eq!(
        listed(project.path(), "refby.any(up).path=story-2/b.md"),
        ["story-1/a.md"]
    );
    assert_eq!(
        listed(project.path(), "refby.any(up).path=story-1/a.md"),
        Vec::<String>::new()
    );
}

/// Project `a`, of one namespace, imports `b`, of two, which has a `README.md` at its root.
/// `a/notes/p.md` refers to it by a frontmatter ref and by a body link, and `a/notes/q.md` refers
/// to `p.md`.
fn importing_project() -> Scratch {
    let parent = Scratch::empty();
    parent.file(
        "a/.typdoc/config.json",
        r#"{ "version": 1, "imports": { "other": "../b" } }"#,
    );
    parent.file(
        "a/.typdoc/collections/notes.json",
        r#"{ "match": "notes/*.md", "schema": "schemas/n.json" }"#,
    );
    parent.file("a/schemas/n.json", SCHEMA);
    parent.file(
        "a/notes/p.md",
        "---\nup: other::README.md\n---\n\n[root](other::README.md)\n",
    );
    parent.file("a/notes/q.md", "---\nup: p.md\n---\n");
    parent.file(
        "b/.typdoc/config.json",
        r#"{ "version": 1, "namespaces": ["story-1", "story-2"] }"#,
    );
    parent.file("b/.typdoc/collections/notes.json", COLLECTION);
    parent.file("b/schemas/n.json", SCHEMA);
    parent.file("b/README.md", "# Root of b\n");
    parent.file("b/story-1/x.md", "body\n");
    parent.file("b/story-2/y.md", "body\n");
    parent
}

#[test]
fn refs_through_an_import_names_the_imported_root_file_by_path_and_project() {
    let parent = importing_project();

    let out = ok(&run(
        &parent.path().join("a"),
        &["refs", "notes/p.md", "--json"],
    ));

    assert_eq!(
        out["refs"],
        json!([
            {
                "path": "README.md",
                "project": "other",
                "field": "up",
                "written": "other::README.md"
            },
            {
                "path": "README.md",
                "project": "other",
                "field": "$body",
                "written": "other::README.md",
                "line": 5,
                "col": 1
            }
        ])
    );
}

#[test]
fn conditions_through_an_import_go_on_past_a_root_file_of_the_imported_project() {
    let parent = importing_project();
    let project = parent.path().join("a");

    assert_eq!(
        listed(&project, "ref.any(up)"),
        ["notes/p.md", "notes/q.md"]
    );
    assert_eq!(listed(&project, "ref.any($body)"), ["notes/p.md"]);
    assert_eq!(
        listed(&project, "ref.any(up).path=README.md"),
        ["notes/p.md"]
    );
    assert_eq!(
        listed(&project, "ref.all(up).path=README.md"),
        ["notes/p.md"]
    );
    assert_eq!(listed(&project, "refby.any(up)"), ["notes/p.md"]);
    assert_eq!(
        listed(&project, "refby.any(up).path=notes/q.md"),
        ["notes/p.md"]
    );
    assert_eq!(
        listed(&project, "refby.any(up).path=notes/p.md"),
        Vec::<String>::new()
    );
    // What `q.md` reaches is `p.md`, in this project's `default`; what `p.md` reaches is the
    // imported root file, which has no namespace.
    assert_eq!(listed(&project, "ref.any(up).namespace=*"), ["notes/q.md"]);
    assert_eq!(listed(&project, "ref.any(up).namespace!=*"), ["notes/p.md"]);
}

#[test]
fn with_one_namespace_a_file_outside_every_collection_is_still_in_default() {
    let project = Scratch::empty();
    project.file(".typdoc/config.json", r#"{ "version": 1 }"#);
    project.file(
        ".typdoc/collections/notes.json",
        r#"{ "match": "notes/*.md", "schema": "schemas/n.json" }"#,
    );
    project.file("schemas/n.json", SCHEMA);
    project.file("README.md", "# Root\n");
    project.file("notes/x.md", "---\nup: ../README.md\n---\n");

    let out = ok(&run(project.path(), &["refs", "notes/x.md", "--json"]));

    assert_eq!(
        out["refs"],
        json!([{
            "path": "README.md",
            "namespace": "default",
            "field": "up",
            "written": "../README.md"
        }])
    );
    assert_eq!(
        listed(project.path(), "ref.any(up).namespace=default"),
        ["notes/x.md"]
    );
}
