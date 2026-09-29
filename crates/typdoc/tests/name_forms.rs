//! Covers SPC-2, SPC-14 and SPC-18: how each form of a name is read in a frontmatter ref, in a body
//! link and as a command's argument, from the project folder and from a namespace folder, and the
//! names commands print.
//!
//! A characterisation: every row states what typdoc does, including where the places disagree. A
//! row that a change to the grammar moves is changed on purpose, in the same commit as the code.
//!
//! The project: namespaces `story-1` and `story-2`, a `notes` collection (`notes/**/*.md`, a `see`
//! ref, `refBase` left at `file`) and a coded `tickets` collection (`WF`), and an import `memory`
//! with one namespace (`notes`, and a coded `learn` collection, `LRN`). Every name is written in
//! `story-1/notes/a.md`.

#[allow(dead_code, reason = "each test file uses part of the shared helper")]
mod common;

use common::{Ran, Scratch, Spawn};
use serde_json::Value;

fn project() -> Scratch {
    let project = Scratch::project(&[
        (
            ".typdoc/config.json",
            r#"{ "version": 1, "namespaces": ["story-*"], "imports": { "memory": "./memory" } }"#,
        ),
        (
            ".typdoc/collections/notes.json",
            r#"{ "match": "notes/**/*.md", "schema": "note.json" }"#,
        ),
        (
            ".typdoc/collections/tickets.json",
            r#"{ "match": "tickets/{key}.md", "schema": "ticket.json" }"#,
        ),
        (
            "note.json",
            r#"{ "name": "note", "fields": { "title": { "type": "string" }, "see": { "type": "ref", "target": "*" } } }"#,
        ),
        (
            "ticket.json",
            r#"{ "name": "ticket", "code": "WF", "fields": { "title": { "type": "string" } } }"#,
        ),
        (
            ".typdoc/state/story-1.json",
            r#"{ "tickets": { "last": 1 } }"#,
        ),
        (
            ".typdoc/state/story-2.json",
            r#"{ "tickets": { "last": 1 } }"#,
        ),
        ("memory/.typdoc/config.json", r#"{ "version": 1 }"#),
        (
            "memory/.typdoc/collections/notes.json",
            r#"{ "match": "notes/*.md", "schema": "note.json" }"#,
        ),
        (
            "memory/.typdoc/collections/learn.json",
            r#"{ "match": "learn/{key}.md", "schema": "learn.json" }"#,
        ),
        (
            "memory/note.json",
            r#"{ "name": "note", "fields": { "title": { "type": "string" } } }"#,
        ),
        (
            "memory/learn.json",
            r#"{ "name": "learn", "code": "LRN", "fields": { "title": { "type": "string" } } }"#,
        ),
        (
            "memory/.typdoc/state/default.json",
            r#"{ "learn": { "last": 1 } }"#,
        ),
    ]);
    project.file("story-2/notes/x.md", "---\ntitle: X in story-2\n---\n");
    project.file("story-1/notes/x.md", "---\ntitle: X in story-1\n---\n");
    project.file("story-1/notes/sub/deep.md", "---\ntitle: deep\n---\n");
    project.file(
        "story-2/tickets/WF-1.md",
        "---\ntitle: WF-1 of story-2\n---\n",
    );
    project.file(
        "story-1/tickets/WF-1.md",
        "---\ntitle: WF-1 of story-1\n---\n",
    );
    project.file("memory/notes/y.md", "---\ntitle: Y in memory\n---\n");
    project.file("memory/learn/LRN-1.md", "---\ntitle: LRN-1\n---\n");
    project
}

const HOLDER: &str = "story-1/notes/a.md";

fn run(project: &Scratch, args: &[&str], cwd: &str) -> Ran {
    Spawn::args(args.iter().copied())
        .cwd(project.path().join(cwd))
        .run()
}

/// A resolved name as `project::namespace:path`, the project only for an import.
fn named(entry: &Value) -> String {
    let project = entry["project"]
        .as_str()
        .map(|p| format!("{p}::"))
        .unwrap_or_default();
    format!(
        "{project}{}:{}",
        entry["namespace"].as_str().unwrap_or("?"),
        entry["path"].as_str().unwrap_or("?")
    )
}

/// What the holder's first out-ref reads as: a resolved name, `unresolved:<why>`, or `no ref`.
fn first_out_ref(project: &Scratch) -> String {
    let ran = run(project, &["refs", HOLDER, "--json"], "");
    assert_eq!(ran.code, 0, "refs: {}", ran.stderr);
    let refs = ran.stdout_json()["refs"].clone();
    match refs.as_array().and_then(|r| r.first()) {
        None => "no ref".to_owned(),
        Some(entry) => match entry["unresolved"].as_str() {
            Some(why) => format!("unresolved:{why}"),
            None => named(entry),
        },
    }
}

fn as_ref(project: &Scratch, form: &str) -> String {
    project.file(HOLDER, &format!("---\ntitle: A\nsee: '{form}'\n---\n"));
    first_out_ref(project)
}

fn as_body_link(project: &Scratch, form: &str) -> String {
    project.file(HOLDER, &format!("---\ntitle: A\n---\n\nSee [l]({form}).\n"));
    first_out_ref(project)
}

/// `get` of the name, run from `cwd`: the document it names, or the exit code.
fn as_argument(project: &Scratch, form: &str, cwd: &str) -> String {
    let ran = run(project, &["get", form, "--json"], cwd);
    if ran.code == 0 {
        named(&ran.stdout_json()["document"])
    } else {
        format!("exit {}", ran.code)
    }
}

struct Row {
    form: String,
    in_ref: &'static str,
    in_body_link: &'static str,
    argument_from_project: &'static str,
    argument_from_namespace: &'static str,
}

fn row(
    form: &str,
    in_ref: &'static str,
    in_body_link: &'static str,
    argument_from_project: &'static str,
    argument_from_namespace: &'static str,
) -> Row {
    Row {
        form: form.to_owned(),
        in_ref,
        in_body_link,
        argument_from_project,
        argument_from_namespace,
    }
}

const X2: &str = "story-2:story-2/notes/x.md";
const WF1_1: &str = "story-1:story-1/tickets/WF-1.md";
const WF1_2: &str = "story-2:story-2/tickets/WF-1.md";
const NOT_FOUND: &str = "unresolved:not-found";

fn rows(project: &Scratch) -> Vec<Row> {
    let mut rows = vec![
        // A key in the holder's own namespace. As an argument from the project folder the key is
        // in both namespaces, so it is ambiguous; from `story-1` it is that namespace's.
        row("WF-1", WF1_1, NOT_FOUND, "exit 1", WF1_1),
        row("WF-1-some-slug", WF1_1, NOT_FOUND, "exit 1", WF1_1),
        row("story-2:WF-1", WF1_2, NOT_FOUND, WF1_2, WF1_2),
        row("story-2:WF-1-x", WF1_2, NOT_FOUND, WF1_2, WF1_2),
        row(
            "memory::LRN-1",
            "memory::default:learn/LRN-1.md",
            NOT_FOUND,
            "memory::default:learn/LRN-1.md",
            "memory::default:learn/LRN-1.md",
        ),
        // A key-shaped name with a code no schema has: a key in a ref and as an argument, which
        // no document has.
        row("XX-1", NOT_FOUND, NOT_FOUND, "exit 5", "exit 5"),
        // A path after a namespace prefix: from the namespace's folder, in every place.
        row("story-2:notes/x.md", X2, X2, X2, X2),
        row(
            "story-2:story-2/notes/x.md",
            NOT_FOUND,
            NOT_FOUND,
            "exit 5",
            "exit 5",
        ),
        // A path with no prefix: from the document in a ref and a body link, from the project
        // folder as an argument, whatever the current directory.
        row("notes/x.md", NOT_FOUND, NOT_FOUND, "exit 5", "exit 5"),
        row("story-2/notes/x.md", NOT_FOUND, NOT_FOUND, X2, X2),
        row("../../story-2/notes/x.md", X2, X2, "exit 5", "exit 5"),
        row(
            "sub/deep.md",
            "story-1:story-1/notes/sub/deep.md",
            "story-1:story-1/notes/sub/deep.md",
            "exit 5",
            "exit 5",
        ),
        // `./`: from the document in a ref and a body link, from the current directory as an
        // argument.
        row(
            "./x.md",
            "story-1:story-1/notes/x.md",
            "story-1:story-1/notes/x.md",
            "exit 5",
            "exit 5",
        ),
        row(
            "memory::notes/y.md",
            "memory::default:notes/y.md",
            "memory::default:notes/y.md",
            "memory::default:notes/y.md",
            "memory::default:notes/y.md",
        ),
        // A prefix that names no namespace: a bad prefix everywhere. In a body link only because
        // the rest ends in `.md`; `tel:123` is a URL, not a ref.
        row(
            "default:notes/x.md",
            "unresolved:bad-prefix",
            "unresolved:bad-prefix",
            "exit 1",
            "exit 1",
        ),
        row(
            "stroy-2:notes/x.md",
            "unresolved:bad-prefix",
            "unresolved:bad-prefix",
            "exit 1",
            "exit 1",
        ),
        row(
            "tel:123",
            "unresolved:bad-prefix",
            "no ref",
            "exit 1",
            "exit 1",
        ),
    ];
    // An absolute path, a drive letter included: never followed in a ref and a body link, on
    // disk as an argument.
    let absolute = project.path().join("story-2/notes/x.md");
    rows.push(row(
        absolute.to_str().expect("a UTF-8 scratch path"),
        NOT_FOUND,
        NOT_FOUND,
        X2,
        X2,
    ));
    rows
}

#[test]
fn each_name_form_is_read_as_this_table_says() {
    let project = project();
    let mut differences = Vec::new();
    for row in rows(&project) {
        let form = row.form.as_str();
        let seen = [
            ("in a ref", as_ref(&project, form), row.in_ref),
            (
                "in a body link",
                as_body_link(&project, form),
                row.in_body_link,
            ),
            (
                "as an argument from the project folder",
                as_argument(&project, form, ""),
                row.argument_from_project,
            ),
            (
                "as an argument from story-1",
                as_argument(&project, form, "story-1"),
                row.argument_from_namespace,
            ),
        ];
        for (place, got, expected) in seen {
            if got != expected {
                differences.push(format!("`{form}` {place}: expected {expected}, got {got}"));
            }
        }
    }
    assert!(differences.is_empty(), "\n{}", differences.join("\n"));
}

/// A path is printed from the project folder, with no namespace prefix, and a document of an
/// import with no project prefix.
#[test]
fn the_names_commands_print_today() {
    let project = project();
    project.file(HOLDER, "---\ntitle: A\n---\n");

    let ran = run(&project, &["list", "--namespace", "*"], "");
    assert_eq!(ran.code, 0, "{}", ran.stderr);
    for name in ["story-2:WF-1", "story-1:WF-1", "story-2/notes/x.md", HOLDER] {
        assert!(
            ran.stdout
                .lines()
                .any(|line| line.starts_with(&format!("{name} "))),
            "{name} not printed:\n{}",
            ran.stdout
        );
    }

    let ran = run(&project, &["list", "--namespace", "memory::*"], "");
    assert_eq!(ran.code, 0, "{}", ran.stderr);
    for name in ["default:LRN-1", "notes/y.md"] {
        assert!(
            ran.stdout
                .lines()
                .any(|line| line.starts_with(&format!("{name} "))),
            "{name} not printed:\n{}",
            ran.stdout
        );
    }
}
