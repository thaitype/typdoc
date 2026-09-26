//! Covers SPC-7, SPC-8, SPC-14, SPC-17.

#[allow(dead_code, reason = "each test file uses part of the shared helper")]
mod common;

use common::{NOTES, Ran, Scratch, Spawn, fixture};
use serde_json::{Value, json};

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
fn an_entry_with_a_star_reaches_no_dot_folder_though_a_literal_entry_reaches_the_same_one() {
    let glob = project(r#"["one", ".*"]"#, &["one", ".hidden"]);
    let literal = project(r#"["one", ".hidden"]"#, &["one", ".hidden"]);

    assert_eq!(
        document(&get(glob.path(), "one/a.md"))["namespace"],
        json!("one")
    );
    assert_eq!(
        rules(&get(literal.path(), "one/a.md"))[0].0,
        "config.namespace-name"
    );
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

/// Declares the one field the documents of `clean` write, so a report holds only what a test is
/// about.
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

fn clean(namespaces: &str, folders: &[&str]) -> Scratch {
    let project = Scratch::project(&TITLED_NOTE);
    project.file(".typdoc/config.json", &config(namespaces));
    for folder in folders {
        project.file(&format!("{folder}/a.md"), "---\ntitle: x\n---\n");
    }
    project
}

fn project_with_a_link(namespaces: &str) -> Scratch {
    let project = clean(namespaces, &["story-1", "story-2"]);
    project.symlink("current", "story-2");
    project
}

fn json_of(project: &Scratch, args: &[&str]) -> Ran {
    Spawn::args(args.iter().copied().chain(["--json"]))
        .cwd(project.path())
        .run()
}

fn paths_listed(ran: &Ran) -> Vec<String> {
    assert_eq!(ran.code, 0, "stderr: {}", ran.stderr);
    ran.stdout_json()["documents"]
        .as_array()
        .expect("documents")
        .iter()
        .map(|d| d["path"].as_str().expect("path").to_owned())
        .collect()
}

#[test]
fn a_glob_that_reaches_a_link_skips_it_and_reports_it_and_the_other_namespaces_are_answered() {
    for glob in [r#""*""#, r#"["story-1", "story-2", "cur*"]"#, r#"["*r*"]"#] {
        let project = project_with_a_link(glob);

        let listed = json_of(&project, &["list"]);
        let checked = json_of(&project, &["validate"]);

        assert_eq!(
            paths_listed(&listed),
            ["story-1/a.md", "story-2/a.md"],
            "{glob}"
        );
        assert_eq!(checked.code, 2, "{glob}: {}", checked.stderr);
        let report = checked.stdout_json();
        assert_eq!(
            report["findings"],
            json!([{
                "path": "current",
                "rule": "files.unreadable",
                "level": "error",
                "message": "a symbolic link is not read: a run does not follow one out of the project, or read one file twice under two names",
            }]),
            "{glob}"
        );
        assert_eq!(
            report["summary"]["checked"]["documents"],
            json!(2),
            "{glob}"
        );
        assert_eq!(
            report["summary"]["checked"]["namespaces"],
            json!(["story-1", "story-2"]),
            "{glob}"
        );
    }
}

#[test]
fn a_link_a_glob_reaches_gives_a_document_no_second_path_and_leaves_the_commands_working() {
    let project = project_with_a_link(r#""*""#);

    let read = json_of(&project, &["get", "story-2/a.md"]);
    let through = json_of(&project, &["get", "current/a.md"]);
    let checked = json_of(&project, &["validate", "current/a.md"]);
    let table = json_of(&project, &["toc", "current/a.md"]);
    let refs = json_of(&project, &["refs", "story-2/a.md"]);

    assert_eq!(document(&read)["namespace"], json!("story-2"));
    assert_eq!(through.code, 5, "{}", through.stderr);
    assert_eq!(checked.code, 5, "{}", checked.stderr);
    assert_eq!(table.code, 5, "{}", table.stderr);
    assert_eq!(refs.code, 0, "{}", refs.stderr);
}

#[test]
fn a_link_a_glob_reaches_is_no_document_set_of_its_own_in_the_audit() {
    let project = project_with_a_link(r#""*""#);

    let ran = json_of(&project, &["validate", "--audit"]);

    let report = ran.stdout_json();
    assert_eq!(report["summary"]["checked"]["documents"], json!(2));
    assert_eq!(report["audit"]["uncollected"], json!([]));
    assert_eq!(report["audit"]["collections"][0]["documents"], json!(2));
}

#[test]
fn a_finding_about_a_skipped_link_is_reported_whichever_namespace_is_asked_for() {
    let project = project_with_a_link(r#""*""#);

    let ran = json_of(&project, &["validate", "--namespace", "story-1"]);

    let findings = ran.stdout_json()["findings"].clone();
    assert_eq!(
        findings.as_array().expect("findings").len(),
        1,
        "{findings}"
    );
    assert_eq!(findings[0]["path"], json!("current"));
}

#[test]
fn a_link_to_a_file_or_a_link_to_nothing_that_a_glob_reaches_is_left_alone_like_a_regular_file() {
    let project = clean(r#""*""#, &["story-1", "story-2"]);
    project.file("README.md", "read me\n");
    project.symlink("CLAUDE.md", "README.md");
    project.symlink("gone", "nowhere");

    let listed = json_of(&project, &["list"]);
    let checked = json_of(&project, &["validate"]);

    assert_eq!(paths_listed(&listed), ["story-1/a.md", "story-2/a.md"]);
    assert_eq!(checked.code, 0, "{}", checked.stderr);
    let report = checked.stdout_json();
    assert_eq!(report["findings"], json!([]));
    assert_eq!(report["summary"]["checked"]["documents"], json!(2));
}

#[test]
fn a_link_whose_name_begins_with_a_dot_is_reached_by_no_glob_and_reported_by_none() {
    let project = clean(r#""*""#, &["one"]);
    project.symlink(".current", "one");

    let ran = json_of(&project, &["validate"]);

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    assert_eq!(ran.stdout_json()["findings"], json!([]));
}

#[test]
fn a_link_that_an_entry_names_in_plain_text_is_config_namespaces_entry_and_says_what_to_name() {
    for name in ["current", ".current"] {
        let project = project(r#"["story-1", "story-2"]"#, &["story-1", "story-2"]);
        project.symlink(name, "story-2");
        project.file(
            ".typdoc/config.json",
            &config(&format!(r#"["story-1", "story-2", "{name}"]"#)),
        );

        let ran = get(project.path(), "story-1/a.md");

        assert_eq!(
            rules(&ran),
            [(
                "config.namespaces-entry".to_owned(),
                ".typdoc/config.json".to_owned()
            )],
            "{name}"
        );
        let said = messages(&ran);
        assert!(said.contains(&format!("`{name}`")), "{said}");
        assert!(said.contains("symbolic link"), "{said}");
        assert!(
            said.contains("name the folder the link points to"),
            "{said}"
        );
        assert!(!said.contains("not decided"), "{said}");
    }
}

#[test]
fn a_link_named_in_plain_text_is_an_error_where_the_same_link_under_a_glob_is_a_finding() {
    let named = project_with_a_link(r#"["story-1", "story-2", "current"]"#);
    let globbed = project_with_a_link(r#"["story-1", "story-2", "curr*"]"#);

    let named = json_of(&named, &["list"]);
    let globbed = json_of(&globbed, &["list"]);

    assert_eq!(named.code, 2, "{}", named.stderr);
    assert_eq!(globbed.code, 0, "{}", globbed.stderr);
}

#[cfg_attr(
    not(target_os = "linux"),
    ignore = "a non-UTF-8 filename needs a POSIX filesystem that allows arbitrary bytes in a \
              name; APFS on macOS refuses to create one at all (EILSEQ), confirmed on a real \
              macos-latest CI run, 2026-09-24"
)]
#[test]
fn a_folder_whose_name_is_not_valid_utf8_that_a_glob_reaches_is_skipped_and_reported() {
    let project = clean(r#""*""#, &["one"]);
    project.file_named_by_bytes(b"\xff/a.md", "---\ntitle: x\n---\n");

    let read = get(project.path(), "one/a.md");
    let checked = json_of(&project, &["validate"]);

    assert_eq!(document(&read)["namespace"], json!("one"));
    let report = checked.stdout_json();
    assert_eq!(report["findings"].as_array().expect("findings").len(), 1);
    assert_eq!(report["findings"][0]["rule"], json!("files.unreadable"));
    assert!(
        report["findings"][0]["message"]
            .as_str()
            .expect("message")
            .contains("not valid UTF-8")
    );
    assert_eq!(report["summary"]["checked"]["documents"], json!(1));
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
// A namespace named after a URL scheme.
// -------------------------------------------------------------------------------------------

fn a_scheme_named_folder_is_refused(scheme: &str) {
    for entry in [format!(r#""{scheme}""#), r#""*""#.to_owned()] {
        let project = project(&entry, &["one", scheme]);

        let ran = get(project.path(), "one/a.md");

        assert_eq!(
            rules(&ran),
            [(
                "config.namespace-name".to_owned(),
                ".typdoc/config.json".to_owned()
            )],
            "{entry}"
        );
        let said = messages(&ran);
        assert!(said.contains(&format!("`{scheme}`")), "{said}");
        assert!(said.contains("URL scheme"), "{said}");
        assert!(!said.contains("ASCII"), "{said}");
    }
}

#[test]
fn a_namespace_named_http_is_config_namespace_name() {
    a_scheme_named_folder_is_refused("http");
}

#[test]
fn a_namespace_named_https_is_config_namespace_name() {
    a_scheme_named_folder_is_refused("https");
}

#[test]
fn a_namespace_named_mailto_is_config_namespace_name() {
    a_scheme_named_folder_is_refused("mailto");
}

#[test]
fn a_namespace_named_file_is_config_namespace_name() {
    a_scheme_named_folder_is_refused("file");
}

#[test]
fn a_name_that_only_looks_like_a_scheme_is_a_namespace() {
    for name in ["httpx", "Http", "HTTPS", "ftp", "files"] {
        let project = project(&format!(r#""{name}""#), &[name]);

        let ran = get(project.path(), &format!("{name}/a.md"));

        assert_eq!(document(&ran)["namespace"], json!(name));
    }
}

#[test]
fn an_import_alias_named_after_a_url_scheme_is_still_refused_under_schema_valid() {
    let project = Scratch::project(&NOTES);
    project.file(
        ".typdoc/config.json",
        r#"{ "version": 1, "imports": { "http": "./nowhere" } }"#,
    );

    let ran = json_of(&project, &["validate"]);

    let findings = ran.stdout_json()["findings"].clone();
    let findings = findings.as_array().unwrap();
    assert!(
        findings.iter().any(|f| f["rule"] == json!("schema.valid")
            && f["message"].as_str().unwrap().contains("http")),
        "{findings:?}"
    );
}

// --- Excluding a namespace with a `!` entry ---

#[test]
fn a_later_exclusion_hides_the_folder_from_every_command() {
    let project = clean(
        r#"["story-*", "!story-1"]"#,
        &["story-1", "story-2", "story-3"],
    );

    let listed = json_of(&project, &["list"]);
    let checked = json_of(&project, &["validate"]);
    let visible = get(project.path(), "story-2/a.md");
    let excluded = get(project.path(), "story-1/a.md");

    assert_eq!(paths_listed(&listed), ["story-2/a.md", "story-3/a.md"]);
    assert_eq!(checked.code, 0, "{}", checked.stderr);
    let report = checked.stdout_json();
    assert_eq!(report["findings"], json!([]));
    assert_eq!(
        report["summary"]["checked"]["namespaces"],
        json!(["story-2", "story-3"])
    );
    assert_eq!(document(&visible)["namespace"], json!("story-2"));
    assert_eq!(
        excluded.code, 5,
        "an excluded namespace's document is not found, the same as one outside every namespace: {}",
        excluded.stderr
    );
}

#[test]
fn a_later_plain_entry_re_includes_a_namespace_an_earlier_exclusion_removed() {
    let project = clean(
        r#"["story-*", "!story-1", "story-1"]"#,
        &["story-1", "story-2"],
    );

    let found = document(&get(project.path(), "story-1/a.md"));

    assert_eq!(
        found["namespace"],
        json!("story-1"),
        "the last pattern that matches a folder decides"
    );
}

#[test]
fn an_exclusion_entry_matching_no_folder_is_silent_whether_exact_or_glob() {
    for namespaces in [r#"["story-*", "!story-9"]"#, r#"["story-*", "!old-*"]"#] {
        let project = clean(namespaces, &["story-1"]);

        let ran = get(project.path(), "story-1/a.md");

        assert_eq!(
            document(&ran)["namespace"],
            json!("story-1"),
            "{namespaces}: a `!` entry matching nothing is never reported"
        );
    }
}

#[test]
fn a_plain_entry_matching_no_folder_is_still_reported_alongside_an_exclusion() {
    let project = clean(r#"["story-*", "absent", "!story-9"]"#, &["story-1"]);

    let ran = get(project.path(), "story-1/a.md");

    assert_eq!(
        rules(&ran),
        [(
            "config.namespaces-entry".to_owned(),
            ".typdoc/config.json".to_owned()
        )],
        "the plain entry `absent` still reports; the `!` entry stays silent"
    );
}

#[test]
fn a_flag_naming_an_excluded_namespace_explicitly_is_not_a_namespace_of_this_project() {
    let project = clean(r#"["story-*", "!story-1"]"#, &["story-1", "story-2"]);

    let ran = json_of(&project, &["list", "--namespace", "story-1"]);

    assert_eq!(ran.code, 1, "{}", ran.stderr);
    let text = ran.stderr_json()["error"].as_str().unwrap().to_owned();
    assert!(text.contains("story-1"), "{text}");
    assert!(
        text.contains("not a namespace of this project"),
        "an excluded namespace refuses the same way one that never existed does: {text}"
    );
}

#[test]
fn a_flag_with_a_leading_bang_is_a_syntax_error_not_a_silent_misread() {
    let project = clean(r#"["story-*", "!story-1"]"#, &["story-1", "story-2"]);

    let ran = json_of(&project, &["list", "--namespace", "!story-1"]);

    assert_eq!(ran.code, 1, "{}", ran.stderr);
    let text = ran.stderr_json()["error"].as_str().unwrap().to_owned();
    assert!(
        text.contains("not a namespace name or a glob"),
        "`!` is namespaces-only this story; `--namespace` still rejects it outright: {text}"
    );
}

#[test]
fn a_write_into_an_excluded_namespace_fails_the_same_way_as_a_namespace_that_never_existed() {
    let project = clean(r#"["story-*", "!story-1"]"#, &["story-1", "story-2"]);

    let ran = json_of(&project, &["new", "story-1/b.md"]);

    assert_eq!(ran.code, 1, "{}", ran.stderr);
    let text = ran.stderr_json()["error"].as_str().unwrap().to_owned();
    assert!(
        text.contains("not inside any namespace"),
        "`new` into an excluded namespace's folder needs no code of its own: {text}"
    );
}

fn coded(namespaces: &str) -> Scratch {
    let project = Scratch::project(&[]);
    project.file(".typdoc/config.json", &config(namespaces));
    project.file(
        ".typdoc/collections/tickets.json",
        r#"{ "match": "tickets/{key}.md", "schema": "ticket.json" }"#,
    );
    project.file(
        "ticket.json",
        r#"{ "name": "ticket", "code": "WF", "fields": {
            "title": { "type": "string" },
            "context": { "type": "ref", "target": "*" }
        } }"#,
    );
    project.file("story-1/.keep", "");
    project.file("story-2/.keep", "");
    project
}

#[test]
fn a_renumber_into_an_excluded_namespace_fails_the_same_way_as_a_namespace_that_never_existed() {
    let project = coded(r#"["story-*", "!story-1"]"#);
    project.file("story-2/tickets/WF-1.md", "---\ntitle: One\n---\n");

    let ran = json_of(&project, &["mv", "WF-1", "--renumber", "story-1"]);

    assert_eq!(ran.code, 1, "{}", ran.stderr);
    let text = ran.stderr_json()["error"].as_str().unwrap().to_owned();
    assert!(text.contains("story-1"), "{text}");
    assert!(
        text.contains("not a namespace of this project"),
        "`mv --renumber` into an excluded namespace refuses the same way one that never existed does: {text}"
    );
}

#[test]
fn a_bare_key_the_excluded_namespace_alone_ever_issued_resolves_as_not_found() {
    let project = coded(r#"["story-*", "!story-1"]"#);
    // story-1 is excluded but still holds WF-1 on disk: the key never enters the project's
    // index, the same as if the folder held nothing at all.
    project.file("story-1/tickets/WF-1.md", "---\ntitle: Excluded\n---\n");
    project.file(
        "story-2/tickets/WF-2.md",
        "---\ntitle: Visible\ncontext: WF-1\n---\n",
    );

    let ran = json_of(&project, &["refs", "story-2/tickets/WF-2.md"]);

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    assert_eq!(
        ran.stdout_json()["refs"],
        json!([{ "unresolved": "not-found", "field": "context", "written": "WF-1" }]),
        "a key only the excluded namespace ever issued is not found, the same as any unknown key"
    );
}

// --- The state file of an excluded namespace ---

#[test]
fn an_excluded_namespaces_existing_state_does_not_stop_other_commands() {
    let project = coded(r#"["story-*", "!story-1"]"#);
    project.file(
        ".typdoc/state/story-1.json",
        r#"{ "tickets": { "last": 3 } }"#,
    );
    project.file("story-1/tickets/WF-1.md", "---\ntitle: Excluded\n---\n");
    project.file(
        ".typdoc/state/story-2.json",
        r#"{ "tickets": { "last": 1 } }"#,
    );
    project.file("story-2/tickets/WF-1.md", "---\ntitle: Visible\n---\n");

    let validated = json_of(&project, &["validate"]);
    let listed = json_of(&project, &["list"]);
    let got = json_of(&project, &["get", "story-2/tickets/WF-1.md"]);
    let created = json_of(&project, &["new", "WF", "Fresh"]);

    assert_eq!(validated.code, 0, "{}", validated.stderr);
    assert_eq!(listed.code, 0, "{}", listed.stderr);
    assert_eq!(got.code, 0, "{}", got.stderr);
    assert_eq!(created.code, 0, "{}", created.stderr);
    assert_eq!(
        created.stdout_json()["document"]["key"],
        json!("WF-2"),
        "story-2's own numbering, untouched by story-1's excluded state: {}",
        created.stdout
    );
}

/// The state file's bytes are compared too, so the numbering cannot come out right by luck.
#[test]
fn re_including_a_namespace_continues_numbering_with_its_state_untouched() {
    let project = coded(r#"["story-1", "story-2"]"#);
    let original_state = "{\n  \"tickets\": {\n    \"last\": 3\n  }\n}\n";
    project.file(".typdoc/state/story-1.json", original_state);
    let before_exclusion = project.read(".typdoc/state/story-1.json");

    project.file(
        ".typdoc/config.json",
        &config(r#"["story-1", "story-2", "!story-1"]"#),
    );
    let while_excluded = json_of(&project, &["validate"]);
    assert_eq!(while_excluded.code, 0, "{}", while_excluded.stderr);

    project.file(".typdoc/config.json", &config(r#"["story-1", "story-2"]"#));
    let after_reinclusion = project.read(".typdoc/state/story-1.json");
    assert_eq!(
        after_reinclusion, before_exclusion,
        "exclusion must never read or write the state file"
    );

    let created = json_of(
        &project,
        &["new", "WF", "Reincluded", "--namespace", "story-1"],
    );

    assert_eq!(created.code, 0, "{}", created.stderr);
    assert_eq!(
        created.stdout_json()["document"]["key"],
        json!("WF-4"),
        "continues from the state's own last, 3, not from 1: {}",
        created.stdout
    );
}

/// A `!` entry that matches no folder excludes nothing: matching, not the entry's text, puts a
/// name in the excluded set.
#[test]
fn a_state_files_folder_gone_and_named_only_by_a_bang_entry_is_still_an_orphan() {
    let project = coded(r#"["story-2", "!story-9"]"#);
    project.file(
        ".typdoc/state/story-9.json",
        r#"{ "tickets": { "last": 1 } }"#,
    );

    let ran = json_of(&project, &["validate"]);

    assert_eq!(ran.code, 2, "{}", ran.stderr);
    let error = ran.stderr_json();
    assert_eq!(error["complete"], json!(true), "{error}");
    assert_eq!(
        error["details"],
        json!([{
            "level": "error",
            "rule": "config.state-orphan",
            "path": ".typdoc/state/story-9.json",
            "message": ".typdoc/state/story-9.json matches no current namespace: delete it after removing a namespace, or rename it after renaming a folder",
        }]),
        "{error}"
    );
}
