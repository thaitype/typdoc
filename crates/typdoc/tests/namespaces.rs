//! Several namespaces in one project, the namespace `default`, and the config errors of
//! `namespaces`.

#[allow(dead_code, reason = "each test file uses part of the shared helper")]
mod common;

use common::{NOTES, Ran, Scratch, Spawn, fixture};
use serde_json::{Value, json};
use typdoc::registry;

fn get(project: &std::path::Path, path: &str) -> Ran {
    Spawn::args(["get", path, "--json"]).cwd(project).run()
}

fn document(ran: &Ran) -> Value {
    assert_eq!(ran.code, 0, "stderr: {}", ran.stderr);
    ran.stdout_json()["document"].clone()
}

fn rules(ran: &Ran) -> Vec<(String, String)> {
    assert_eq!(ran.code, 2, "stderr: {}", ran.stderr);
    let object = ran.stderr_json();
    assert_eq!(object["complete"], json!(true), "{object}");
    object["details"]
        .as_array()
        .expect("details")
        .iter()
        .map(|d| {
            (
                d["rule"].as_str().expect("rule").to_owned(),
                d["path"].as_str().expect("path").to_owned(),
            )
        })
        .collect()
}

fn messages(ran: &Ran) -> String {
    ran.stderr_json()["details"]
        .as_array()
        .expect("details")
        .iter()
        .map(|d| d["message"].as_str().expect("message").to_owned())
        .collect::<Vec<_>>()
        .join("\n")
}

fn config(namespaces: &str) -> String {
    format!(r#"{{ "version": 1, "namespaces": {namespaces} }}"#)
}

/// A project with the collection of every `*.md` and the given `namespaces`, and a folder
/// with a document for each name.
fn project(namespaces: &str, folders: &[&str]) -> Scratch {
    let project = Scratch::project(&NOTES);
    project.file(".typdoc/config.json", &config(namespaces));
    for folder in folders {
        project.file(&format!("{folder}/a.md"), "---\ntitle: x\n---\n");
    }
    project
}

#[test]
fn a_project_with_several_namespaces_names_each_document_by_the_folder_it_is_in() {
    let root = fixture("valid/several-namespaces");

    for (path, namespace) in [
        ("story-1/notes/a.md", "story-1"),
        ("story-2/notes/a.md", "story-2"),
        ("archive/notes/old.md", "archive"),
    ] {
        let found = document(&get(&root, path));

        assert_eq!(found["path"], json!(path));
        assert_eq!(found["namespace"], json!(namespace));
        assert_eq!(found["collection"], json!("notes"));
        assert_eq!(found["schema"], json!("note"));
    }
}

#[test]
fn the_same_path_below_two_namespace_folders_is_two_documents() {
    let root = fixture("valid/several-namespaces");

    let one = document(&get(&root, "story-1/notes/a.md"));
    let two = document(&get(&root, "story-2/notes/a.md"));

    assert_eq!(one["fields"], json!({ "title": "Story one, note a" }));
    assert_eq!(two["fields"], json!({ "title": "Story two, note a" }));
}

#[test]
fn a_match_counts_from_the_namespace_folder() {
    let root = fixture("valid/several-namespaces");

    for path in ["notes/a.md", "story-1/notes/deeper/c.md", "story-1/a.md"] {
        let ran = get(&root, path);

        assert_eq!(ran.code, 5, "{path}: {}", ran.stderr);
    }
}

#[test]
fn a_file_outside_every_namespace_folder_belongs_to_no_namespace() {
    let ran = get(&fixture("valid/several-namespaces"), "other/notes/x.md");

    assert_eq!(ran.code, 5, "{}", ran.stderr);
}

#[test]
fn a_document_is_found_from_any_folder_of_the_project() {
    let inside = fixture("valid/several-namespaces").join("story-1/notes");

    let found = document(&get(&inside, "story-2/notes/a.md"));

    assert_eq!(found["namespace"], json!("story-2"));
}

#[test]
fn the_namespace_default_has_no_folder_of_its_own() {
    let found = document(&get(&fixture("valid/minimal"), "note.md"));

    assert_eq!(found["namespace"], json!("default"));
    assert_eq!(found["path"], json!("note.md"));
}

#[test]
fn a_project_with_folders_for_namespaces_has_no_namespace_default() {
    let root = fixture("valid/several-namespaces");
    for path in ["story-1/notes/a.md", "archive/notes/old.md"] {
        assert_ne!(document(&get(&root, path))["namespace"], json!("default"));
    }
}

#[test]
fn an_entry_is_a_name_a_glob_or_an_array_of_them_and_a_folder_is_one_namespace_once() {
    for namespaces in [
        r#""story-*""#,
        r#"["story-1", "story-2"]"#,
        r#"["story-*", "story-1"]"#,
        r#"["*"]"#,
    ] {
        let project = project(namespaces, &["story-1", "story-2"]);

        let found = document(&get(project.path(), "story-2/a.md"));

        assert_eq!(found["namespace"], json!("story-2"), "{namespaces}");
    }
}

#[test]
fn a_glob_that_matches_no_folder_is_no_namespace_and_no_error() {
    for namespaces in [r#""story-*""#, "[]"] {
        let project = project(namespaces, &["other"]);

        let ran = get(project.path(), "other/a.md");

        assert_eq!(ran.code, 5, "{namespaces}: {}", ran.stderr);
    }
}

#[test]
fn a_star_never_matches_a_folder_that_starts_with_a_dot_and_no_file_is_a_namespace() {
    let project = project(r#""*""#, &["one", ".git"]);
    project.file("notes.md", "");

    let ran = get(project.path(), "one/a.md");

    assert_eq!(document(&ran)["namespace"], json!("one"));
    assert_eq!(get(project.path(), ".git/a.md").code, 5);
}

#[test]
fn an_entry_of_more_than_one_segment_or_that_names_no_folder_is_config_namespaces_entry() {
    for entry in [
        r#""a/b""#,
        r#""a/**""#,
        r#""**""#,
        r#""*/x""#,
        r#""absent""#,
        r#""..""#,
        r#""""#,
        "5",
        r#"["one", 5]"#,
        "{}",
    ] {
        let project = project(entry, &["one"]);

        let ran = get(project.path(), "one/a.md");

        assert_eq!(
            rules(&ran),
            [(
                "config.namespaces-entry".to_owned(),
                ".typdoc/config.json".to_owned()
            )],
            "{entry}"
        );
    }
}

#[test]
fn an_entry_that_names_a_file_and_not_a_folder_names_no_folder() {
    let project = project(r#""notes""#, &["one"]);
    project.file("notes", "not a folder");

    let ran = get(project.path(), "one/a.md");

    assert_eq!(rules(&ran)[0].0, "config.namespaces-entry");
}

#[test]
fn a_matched_folder_with_another_name_is_config_namespace_name_and_the_message_names_it() {
    for folder in ["bad name", "n\u{e9}", "a.b", "default"] {
        let project = project(r#""*""#, &["one", folder]);

        let ran = get(project.path(), "one/a.md");

        assert_eq!(
            rules(&ran),
            [(
                "config.namespace-name".to_owned(),
                ".typdoc/config.json".to_owned()
            )],
            "{folder}"
        );
        assert!(
            messages(&ran).contains(&format!("`{folder}`")),
            "{}",
            messages(&ran)
        );
    }
}

#[test]
fn a_folder_that_is_named_by_an_entry_is_checked_like_one_that_a_glob_matches() {
    for entry in [r#""default""#, r#""bad name""#, r#"".hidden""#] {
        let name = entry.trim_matches('"');
        let project = project(entry, &[name]);

        let ran = get(project.path(), "one/a.md");

        assert_eq!(rules(&ran)[0].0, "config.namespace-name", "{entry}");
    }
}

#[test]
fn a_matched_folder_that_holds_its_own_typdoc_is_config_namespace_nested() {
    let project = project(r#""*""#, &["one", "two"]);
    project.file("two/.typdoc/config.json", r#"{ "version": 1 }"#);

    let ran = get(project.path(), "one/a.md");

    assert_eq!(
        rules(&ran),
        [(
            "config.namespace-nested".to_owned(),
            ".typdoc/config.json".to_owned()
        )]
    );
    assert!(messages(&ran).contains("`two`"));
}

#[test]
fn a_folder_that_is_both_badly_named_and_nested_is_reported_for_each() {
    let project = project(r#""*""#, &["one"]);
    project.file("bad name/.typdoc/config.json", "{}");

    let ran = get(project.path(), "one/a.md");

    let found: Vec<String> = rules(&ran).into_iter().map(|(rule, _)| rule).collect();
    assert_eq!(found, ["config.namespace-name", "config.namespace-nested"]);
}

#[test]
fn the_errors_of_namespaces_and_of_collections_are_reported_together() {
    let project = project(r#"["absent", "*"]"#, &["bad name"]);
    project.file(".typdoc/collections/x y.json", "{");

    let ran = get(project.path(), "one/a.md");

    let found: Vec<String> = rules(&ran).into_iter().map(|(rule, _)| rule).collect();
    assert_eq!(
        found,
        [
            "config.collection-name",
            "config.collection-parse",
            "config.namespace-name",
            "config.namespaces-entry",
        ]
    );
}

#[test]
fn a_symbolic_link_that_an_entry_matches_is_refused_and_not_followed() {
    let project = project(r#""*""#, &["one"]);
    project.symlink("linked", "one");

    let ran = get(project.path(), "one/a.md");

    assert_eq!(ran.code, 6, "{}", ran.stderr);
}

#[test]
fn a_namespace_with_no_documents_is_not_an_error() {
    let project = project(r#""empty""#, &[]);
    project.file("empty/.keep", "");

    let ran = get(project.path(), "empty/a.md");

    assert_eq!(ran.code, 5);
}

fn several() -> std::path::PathBuf {
    fixture("valid/several-namespaces")
}

#[test]
fn a_scope_that_names_a_namespace_of_the_project_leaves_a_path_argument_alone() {
    let root = several();

    for args in [
        vec!["--namespace", "story-1"],
        vec!["--namespace", "story-*"],
        vec!["--namespace", "archive,story-2"],
        vec!["--namespace", "*"],
    ] {
        let ran = Spawn::args(
            ["get", "story-2/notes/a.md", "--json"]
                .into_iter()
                .chain(args.iter().copied()),
        )
        .cwd(&root)
        .run();

        assert_eq!(document(&ran)["namespace"], json!("story-2"), "{args:?}");
    }
}

#[test]
fn the_flag_may_come_before_the_command() {
    let ran = Spawn::args([
        "--namespace",
        "story-1",
        "get",
        "story-1/notes/a.md",
        "--json",
    ])
    .cwd(several())
    .run();

    assert_eq!(document(&ran)["namespace"], json!("story-1"));
}

#[test]
fn a_namespace_that_the_project_does_not_have_is_bad_arguments_from_the_flag_and_from_the_variable()
{
    let from_flag = Spawn::args([
        "get",
        "story-1/notes/a.md",
        "--json",
        "--namespace",
        "nosuch",
    ])
    .cwd(several())
    .run();
    let from_variable = Spawn::args(["get", "story-1/notes/a.md", "--json"])
        .var("TYPDOC_NAMESPACE", "nosuch")
        .cwd(several())
        .run();

    for ran in [from_flag, from_variable] {
        assert_eq!(ran.code, 1, "{}", ran.stderr);
        assert!(
            ran.stderr_json()["error"]
                .as_str()
                .unwrap()
                .contains("nosuch")
        );
    }
}

#[test]
fn a_flag_that_is_valid_wins_over_a_variable_that_is_not_and_an_empty_variable_is_not_set() {
    let flag_wins = Spawn::args([
        "get",
        "story-1/notes/a.md",
        "--json",
        "--namespace",
        "story-1",
    ])
    .var("TYPDOC_NAMESPACE", "nosuch")
    .cwd(several())
    .run();
    let empty = Spawn::args(["get", "story-1/notes/a.md", "--json"])
        .var("TYPDOC_NAMESPACE", "")
        .cwd(several())
        .run();

    assert_eq!(flag_wins.code, 0, "{}", flag_wins.stderr);
    assert_eq!(empty.code, 0, "{}", empty.stderr);
}

#[test]
fn a_malformed_list_and_a_name_of_an_imported_project_are_bad_arguments() {
    for list in ["", "story-1,", "a b", "other::*"] {
        let ran = Spawn::args(["get", "story-1/notes/a.md", "--json", "--namespace", list])
            .cwd(several())
            .run();

        assert_eq!(ran.code, 1, "{list:?}: {}", ran.stderr);
    }
}

#[test]
fn a_config_error_is_reported_before_the_scope_is_looked_at() {
    let project = project(r#""absent""#, &[]);

    let ran = Spawn::args(["get", "a.md", "--json", "--namespace", "nosuch"])
        .cwd(project.path())
        .run();

    assert_eq!(ran.code, 2, "{}", ran.stderr);
}

#[test]
fn a_key_is_read_from_the_namespace_the_current_directory_is_in() {
    for (folder, title) in [
        ("story-1", "Story one, ticket WF-1"),
        ("story-2", "Story two, ticket WF-1"),
    ] {
        let ran = get(&several().join(folder), "WF-1");

        let found = document(&ran);
        assert_eq!(found["namespace"], json!(folder), "{folder}");
        assert_eq!(found["key"], json!("WF-1"), "{folder}");
        assert_eq!(found["code"], json!("WF"), "{folder}");
        assert_eq!(found["fields"]["title"], json!(title), "{folder}");
    }
}

#[test]
fn a_key_prefixed_by_a_namespace_is_read_from_that_namespace_from_anywhere() {
    let ran = get(&several(), "story-2:WF-9");

    let found = document(&ran);
    assert_eq!(found["namespace"], json!("story-2"));
    assert_eq!(found["path"], json!("story-2/tickets/WF-9.md"));
}

#[test]
fn a_key_in_more_than_one_namespace_in_scope_exits_1_with_every_candidate() {
    let ran = Spawn::args(["get", "WF-1", "--json", "--namespace", "*"])
        .cwd(several())
        .run();

    assert_eq!(ran.code, 1, "{}", ran.stderr);
    let object = ran.stderr_json();
    let candidates: Vec<String> = object["candidates"]
        .as_array()
        .expect("candidates")
        .iter()
        .map(|c| c.as_str().unwrap().to_owned())
        .collect();
    assert_eq!(candidates, ["story-1:WF-1", "story-2:WF-1"]);
}

#[test]
fn a_key_that_names_no_document_in_scope_exits_5() {
    let ran = get(&several().join("story-1"), "WF-404");

    assert_eq!(ran.code, 5, "{}", ran.stderr);
}

#[test]
fn a_namespace_prefix_on_a_key_that_is_not_a_namespace_of_the_project_is_bad_arguments() {
    let ran = get(&several(), "nosuch:WF-1");

    assert_eq!(ran.code, 1, "{}", ran.stderr);
    assert!(
        ran.stderr_json()["error"]
            .as_str()
            .unwrap()
            .contains("nosuch")
    );
}

// -------------------------------------------------------------------------------------------
// A gap the design's own sentence leaves open (`registry::KNOWN_GAPS`): "Sibling names and
// import aliases may not collide with URL schemes... `validate` enforces this" names both, but
// the rule table (and the code) names only import aliases, so only an alias is enforced. Pinned
// here so that closing the gap — a namespace named for a scheme starting to be reported —
// turns this test red rather than going unnoticed.
// -------------------------------------------------------------------------------------------

/// A schema whose one field, `title`, is what `project()`'s documents write, so a report of
/// this project has nothing to say about anything but the namespace name itself.
const TITLED_NOTE: [(&str, &str); 2] = [
    (
        ".typdoc/collections/notes.json",
        r#"{ "match": "*.md", "schema": "note.json" }"#,
    ),
    (
        "note.json",
        r#"{ "name": "note", "fields": { "title": { "type": "string" } } }"#,
    ),
];

#[test]
fn a_namespace_named_after_a_url_scheme_validates_clean_unlike_an_import_alias_of_the_same_name() {
    assert!(
        registry::KNOWN_GAPS
            .iter()
            .any(|gap| gap.starts_with("[namespace-scheme]")),
        "this test pins a gap that `registry::KNOWN_GAPS` no longer lists"
    );

    // The namespace `http` (a name that is also one of the design's four reserved URL schemes):
    // a project whose only complaint could be that name validates with no finding at all.
    let project = Scratch::project(&TITLED_NOTE);
    project.file(".typdoc/config.json", &config(r#""http""#));
    project.file("http/a.md", "---\ntitle: x\n---\n");

    let ran = Spawn::args(["validate", "--json"])
        .cwd(project.path())
        .run();

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    assert_eq!(ran.stdout_json()["findings"], json!([]));

    // The same name as an import alias, by contrast, is refused today (`schema.valid`) — the
    // asymmetry this gap is about, shown from both sides in one test.
    let project = Scratch::project(&NOTES);
    project.file(
        ".typdoc/config.json",
        r#"{ "version": 1, "imports": { "http": "./nowhere" } }"#,
    );

    let ran = Spawn::args(["validate", "--json"])
        .cwd(project.path())
        .run();

    let findings = ran.stdout_json()["findings"].clone();
    let findings = findings.as_array().unwrap();
    assert!(
        findings.iter().any(|f| f["rule"] == json!("schema.valid")
            && f["message"].as_str().unwrap().contains("http")),
        "{findings:?}"
    );
}
