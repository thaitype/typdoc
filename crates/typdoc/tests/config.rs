//! The config errors that need no schema, import, state file or pin: what the error object on
//! standard error carries for each, and that every error that can be determined is in it.

#[allow(dead_code, reason = "each test file uses part of the shared helper")]
mod common;

use common::{NOTES, Ran, Scratch, Spawn, fixture};
use serde_json::{Value, json};

fn get_a(project: &Scratch) -> Ran {
    Spawn::args(["get", "a.md", "--json"])
        .cwd(project.path())
        .run()
}

/// The error object of a run that ended with 2, checked to have the shape of a config error.
fn config_error(ran: &Ran) -> Value {
    assert_eq!(ran.code, 2, "stderr: {}", ran.stderr);
    assert_eq!(ran.stdout, "", "a failure prints nothing on stdout");
    let object = ran.stderr_json();
    assert_eq!(object["code"], json!(2));
    assert!(object["error"].is_string(), "{object}");
    assert!(object["complete"].is_boolean(), "{object}");
    object
}

/// `(rule, path)` of each detail, in the order printed.
fn details(object: &Value) -> Vec<(String, String)> {
    object["details"]
        .as_array()
        .expect("details")
        .iter()
        .map(|d| {
            assert_eq!(d["level"], json!("error"), "{d}");
            assert!(d["message"].as_str().is_some_and(|m| !m.is_empty()), "{d}");
            (
                d["rule"].as_str().expect("rule").to_owned(),
                d["path"].as_str().expect("path").to_owned(),
            )
        })
        .collect()
}

fn pair(rule: &str, path: &str) -> (String, String) {
    (rule.to_owned(), path.to_owned())
}

fn message_of(object: &Value, index: usize) -> String {
    object["details"][index]["message"]
        .as_str()
        .expect("message")
        .to_owned()
}

fn with_config(config: &str) -> Scratch {
    let project = Scratch::project(&NOTES);
    project.file(".typdoc/config.json", config);
    project
}

fn with_collection(file: &str, text: &str) -> Scratch {
    let project = Scratch::project(&NOTES);
    project.file(file, text);
    project
}

#[test]
fn a_config_that_cannot_be_parsed_is_config_parse_and_the_list_is_not_complete() {
    for text in ["{ version", "", "[1]", "null", r#"{ "version": 1, }"#] {
        let object = config_error(&get_a(&with_config(text)));

        assert_eq!(
            details(&object),
            [pair("config.parse", ".typdoc/config.json")],
            "{text:?}"
        );
        assert_eq!(object["complete"], json!(false), "{text:?}");
    }
}

#[test]
fn a_version_that_is_missing_or_not_known_is_config_version_and_the_list_is_not_complete() {
    for text in [
        "{}",
        r#"{ "version": 2 }"#,
        r#"{ "version": 0 }"#,
        r#"{ "version": "1" }"#,
        r#"{ "version": 1.5 }"#,
        r#"{ "version": null }"#,
    ] {
        let object = config_error(&get_a(&with_config(text)));

        assert_eq!(
            details(&object),
            [pair("config.version", ".typdoc/config.json")],
            "{text:?}"
        );
        assert_eq!(object["complete"], json!(false), "{text:?}");
    }
}

#[test]
fn an_unknown_version_ends_the_list_and_nothing_after_it_is_reported() {
    let project = with_config(r#"{ "version": 2, "name": "x" }"#);
    project.file(".typdoc/collections/bad name.json", "{");

    let object = config_error(&get_a(&project));

    assert_eq!(
        details(&object),
        [pair("config.version", ".typdoc/config.json")]
    );
    assert_eq!(object["complete"], json!(false));
}

#[test]
fn a_key_that_is_not_in_the_file_is_config_unknown_key_for_each_key() {
    let project = with_config(r#"{ "version": 1, "name": "x", "title": "y" }"#);

    let object = config_error(&get_a(&project));

    assert_eq!(
        details(&object),
        [
            pair("config.unknown-key", ".typdoc/config.json"),
            pair("config.unknown-key", ".typdoc/config.json"),
        ]
    );
    assert_eq!(object["complete"], json!(true));
    let messages = [message_of(&object, 0), message_of(&object, 1)].join("\n");
    assert!(
        messages.contains("`name`") && messages.contains("`title`"),
        "{messages}"
    );
}

#[test]
fn a_key_in_a_collection_file_that_is_not_in_the_format_is_config_unknown_key() {
    for key in ["last", "extra"] {
        let project = with_collection(
            ".typdoc/collections/notes.json",
            &format!(r#"{{ "match": "*.md", "schema": "note.json", "{key}": 3 }}"#),
        );

        let object = config_error(&get_a(&project));

        assert_eq!(
            details(&object),
            [pair("config.unknown-key", ".typdoc/collections/notes.json")]
        );
        assert!(message_of(&object, 0).contains(key));
        assert_eq!(object["complete"], json!(true));
    }
}

/// A project of two notes, each with a title, and nothing else in it.
fn two_notes() -> Scratch {
    let project = Scratch::project(&[
        (
            ".typdoc/collections/notes.json",
            r#"{ "match": "*.md", "schema": "note.json" }"#,
        ),
        (
            "note.json",
            r#"{ "name": "note", "fields": { "title": { "type": "string", "required": true } } }"#,
        ),
    ]);
    project.file("a.md", "---\ntitle: First\n---\n\n# First\n");
    project.file("b.md", "---\ntitle: Second\n---\n\n# Second\n");
    project
}

#[test]
fn a_stray_typdoc_json_beside_the_folder_changes_nothing_for_validate_and_list() {
    for stray in [r#"{ "version": 1 }"#, "{}", "{ not json", ""] {
        let project = two_notes();
        project.file(".typdoc.json", stray);

        let validated = Spawn::args(["validate", "--json"])
            .cwd(project.path())
            .run();
        let listed = Spawn::args(["list", "--json"]).cwd(project.path()).run();

        assert_eq!(validated.code, 0, "{stray:?}, stderr: {}", validated.stderr);
        assert_eq!(validated.stderr, "", "{stray:?}");
        let report = validated.stdout_json();
        assert_eq!(report["summary"]["checked"]["documents"], json!(2));
        assert_eq!(
            report["summary"]["findings"],
            json!({ "error": 0, "warn": 0, "info": 0 })
        );
        assert_eq!(report["findings"], json!([]), "{stray:?}");
        assert_eq!(listed.code, 0, "{stray:?}, stderr: {}", listed.stderr);
        let out = listed.stdout_json();
        assert_eq!(out["total"], json!(2), "{stray:?}");
        let paths: Vec<&str> = out["documents"]
            .as_array()
            .expect("documents")
            .iter()
            .map(|d| d["path"].as_str().expect("path"))
            .collect();
        assert_eq!(paths, ["a.md", "b.md"], "{stray:?}");
    }
}

#[test]
fn a_stray_typdoc_json_is_not_reported_with_a_config_that_cannot_be_parsed() {
    let project = with_config("{");
    project.file(".typdoc.json", "{}");

    let object = config_error(&get_a(&project));

    assert_eq!(
        details(&object),
        [pair("config.parse", ".typdoc/config.json")]
    );
    assert_eq!(object["complete"], json!(false));
}

/// The error of a run that found no project: exit 5, the folder it looked for named in the
/// message (discovery is folder-based, M-24: a project is marked by `.typdoc/` existing, not by
/// `config.json` inside it, so the message names the folder, never the file).
fn no_project_error(ran: &Ran) -> String {
    assert_eq!(ran.code, 5, "stderr: {}", ran.stderr);
    assert_eq!(ran.stdout, "", "a failure prints nothing on stdout");
    let object = ran.stderr_json();
    assert_eq!(object["code"], json!(5));
    assert!(object.get("complete").is_none(), "{object}");
    let message = object["error"].as_str().expect("an error message");
    assert!(message.starts_with("no project found"), "{message}");
    assert!(message.contains(".typdoc/"), "{message}");
    assert!(
        !message.contains(".typdoc/config.json"),
        "the folder is what marks a project, not the file inside it: {message}"
    );
    message.to_owned()
}

/// The exact wording of both `NoProject` (the ancestor walk) and `NoProjectAt` (`TYPDOC_DIR`),
/// locked in on its own: `no_project_error`'s own substring checks above compose several partial
/// assertions shared by every case in this section, none of which pin the full sentence down —
/// this is the test that does.
#[test]
fn the_no_project_message_reads_exactly_no_project_folder_not_no_project_file() {
    let empty = Scratch::empty();
    let ancestor_walk = no_project_error(&get_a(&empty));
    assert_eq!(
        ancestor_walk,
        format!(
            "no project found: there is no .typdoc/ in {} or above it",
            empty.path().display()
        )
    );

    let elsewhere = Scratch::empty();
    let ran = Spawn::args(["get", "a.md", "--json"])
        .var("TYPDOC_DIR", "elsewhere")
        .cwd(elsewhere.path())
        .run();
    let typdoc_dir = no_project_error(&ran);
    assert_eq!(
        typdoc_dir,
        format!(
            "no project found: {} has no .typdoc/",
            elsewhere.path().join("elsewhere").display()
        )
    );
}

#[test]
fn an_empty_folder_is_no_project_and_the_message_names_the_typdoc_folder() {
    let empty = Scratch::empty();

    let message = no_project_error(&get_a(&empty));

    assert!(message.contains("or above it"), "{message}");
}

#[test]
fn a_folder_holding_only_a_typdoc_json_is_no_project_and_the_message_names_the_typdoc_folder() {
    let only = Scratch::empty();
    only.file(".typdoc.json", r#"{ "version": 1 }"#);

    let ran = get_a(&only);

    let message = no_project_error(&ran);
    assert!(!message.contains(".typdoc.json"), "{message}");
}

#[test]
fn typdoc_dir_naming_a_folder_without_a_config_is_no_project_and_names_the_typdoc_folder() {
    let project = two_notes();
    project.file("elsewhere/readme.md", "# Nothing here\n");

    let ran = Spawn::args(["get", "a.md", "--json"])
        .var("TYPDOC_DIR", "elsewhere")
        .cwd(project.path())
        .run();

    let message = no_project_error(&ran);
    assert!(message.contains("elsewhere"), "{message}");
}

// --- M-24: `.typdoc/config.json` becomes optional; project discovery looks for the folder ---

/// A plain file named `.typdoc` (no extension, the same name the folder would have) is not a
/// project: discovery needs the directory, not merely something at that name (contract, M-24:
/// `.is_dir()` specifically, not `.exists()`). Covers the ancestor-walk branch of `discover()`.
#[test]
fn a_plain_file_named_dot_typdoc_is_not_a_project_via_the_ancestor_walk() {
    let only = Scratch::empty();
    only.file(".typdoc", "not a directory");

    let ran = get_a(&only);

    no_project_error(&ran);
}

/// The same plain-file-named-`.typdoc` case, through the `TYPDOC_DIR` branch of `discover()`:
/// `TYPDOC_DIR` names the folder the project would live in, and that folder holds a plain file
/// called `.typdoc`, not a directory.
#[test]
fn a_plain_file_named_dot_typdoc_is_not_a_project_via_typdoc_dir() {
    let project = Scratch::empty();
    project.file("elsewhere/.typdoc", "not a directory");

    let ran = Spawn::args(["get", "a.md", "--json"])
        .var("TYPDOC_DIR", "elsewhere")
        .cwd(project.path())
        .run();

    no_project_error(&ran);
}

/// A `.typdoc/` folder with no `config.json` at all is still a valid project, read as
/// `{"version": 1}` (contract, M-24): every command's discovery looks for the folder, and a
/// missing `config.json` is not a read error.
#[test]
fn a_typdoc_folder_with_no_config_json_at_all_loads_as_version_1() {
    let project = Scratch::empty();
    project.file(
        ".typdoc/collections/notes.json",
        r#"{ "match": "*.md", "schema": "note.json" }"#,
    );
    project.file("note.json", r#"{ "name": "note", "fields": {} }"#);
    project.file("a.md", "---\ntitle: x\n---\n");

    let ran = get_a(&project);

    assert_eq!(ran.code, 0, "stderr: {}", ran.stderr);
    assert_eq!(ran.stdout_json()["document"]["path"], json!("a.md"));
}

/// A bare `.typdoc/` folder with nothing in it at all (no `config.json`, no `collections/`) is
/// still a valid project: `list` returns a normal exit code and an empty result, not a config
/// error (contract, M-24, the demoable case).
#[test]
fn a_bare_typdoc_folder_with_nothing_in_it_still_lets_list_run() {
    let project = Scratch::empty();
    std::fs::create_dir_all(project.path().join(".typdoc")).expect("a folder");

    let ran = Spawn::args(["list", "--json"]).cwd(project.path()).run();

    assert_eq!(ran.code, 0, "stderr: {}", ran.stderr);
    assert_eq!(ran.stdout_json()["total"], json!(0));
}

#[test]
fn a_collection_file_that_cannot_be_parsed_or_is_the_wrong_shape_is_config_collection_parse() {
    for text in [
        "{",
        "[]",
        "",
        r#"{ "schema": "note.json" }"#,
        r#"{ "match": "*.md" }"#,
        r#"{ "match": 3, "schema": "note.json" }"#,
        r#"{ "match": "*.md", "schema": ["note.json"] }"#,
        r#"{ "match": "*.md", "schema": "note.json", "refBase": "folder" }"#,
        r#"{ "match": "*.md", "schema": "note.json", "refBase": 1 }"#,
    ] {
        let project = with_collection(".typdoc/collections/notes.json", text);

        let object = config_error(&get_a(&project));

        assert_eq!(
            details(&object),
            [pair(
                "config.collection-parse",
                ".typdoc/collections/notes.json"
            )],
            "{text:?}"
        );
        assert_eq!(object["complete"], json!(true), "{text:?}");
    }
}

#[test]
fn both_values_of_ref_base_and_a_validation_object_are_read() {
    for extra in [
        r#""refBase": "file""#,
        r#""refBase": "namespace""#,
        r#""validation": { "body.links": { "level": "warn" } }"#,
        r#""validation": {}"#,
    ] {
        let project = with_collection(
            ".typdoc/collections/notes.json",
            &format!(r#"{{ "match": "*.md", "schema": "note.json", {extra} }}"#),
        );
        project.file("a.md", "");

        let ran = get_a(&project);

        assert_eq!(ran.code, 0, "{extra}: {}", ran.stderr);
    }
}

#[test]
fn a_collection_file_name_with_anything_but_ascii_letters_digits_dash_and_underscore_is_config_collection_name()
 {
    for name in ["my notes", "notes.v2", "n\u{e9}", "\u{200b}x", "a+b", "a:b"] {
        let path = format!(".typdoc/collections/{name}.json");
        let project = with_collection(&path, r#"{ "match": "*.md", "schema": "note.json" }"#);

        let object = config_error(&get_a(&project));

        let rules: Vec<String> = details(&object).into_iter().map(|(rule, _)| rule).collect();
        assert!(
            rules.contains(&"config.collection-name".to_owned()),
            "{name:?}: {object}"
        );
        assert_eq!(object["complete"], json!(true));
    }
}

#[cfg_attr(
    not(target_os = "linux"),
    ignore = "a non-UTF-8 filename needs a POSIX filesystem that allows arbitrary bytes in a \
              name; APFS on macOS refuses to create one at all (EILSEQ), confirmed on a real \
              macos-latest CI run, 2026-09-24"
)]
#[test]
fn a_collection_file_named_only_by_its_extension_or_by_bytes_that_are_not_utf8_is_named_wrongly() {
    let project = Scratch::project(&NOTES);
    project.file(
        ".typdoc/collections/.json",
        r#"{ "match": "*.md", "schema": "note.json" }"#,
    );
    project.file_named_by_bytes(
        b".typdoc/collections/\xff.json",
        r#"{ "match": "*.md", "schema": "note.json" }"#,
    );

    let object = config_error(&get_a(&project));

    let found = details(&object);
    assert_eq!(found.len(), 2, "{object}");
    assert!(
        found
            .iter()
            .all(|(rule, _)| rule == "config.collection-name")
    );
}

#[test]
fn a_name_that_is_valid_is_read_and_a_file_that_is_not_json_is_ignored() {
    let project = Scratch::project(&NOTES);
    project.file(
        ".typdoc/collections/Other_2-x.json",
        r#"{ "match": "other-*.md", "schema": "note.json" }"#,
    );
    project.file(".typdoc/collections/README.md", "not a collection {");
    project.file(".typdoc/collections/notes.json.bak", "{");
    project.file("a.md", "");

    let ran = get_a(&project);

    assert_eq!(ran.code, 0, "{}", ran.stderr);
}

#[test]
fn a_rule_name_that_is_not_in_the_design_is_config_rule_unknown() {
    for rule in ["body.linkz", "links", "config.parse", "state.orphan", ""] {
        let config =
            json!({ "version": 1, "validation": { "global": { rule: { "level": "warn" } } } });
        let object = config_error(&get_a(&with_config(&config.to_string())));

        assert_eq!(
            details(&object),
            [pair("config.rule-unknown", ".typdoc/config.json")],
            "{rule:?}"
        );
        assert_eq!(object["complete"], json!(true));
    }
}

#[test]
fn an_option_or_a_level_that_a_rule_does_not_accept_is_config_rule_unknown() {
    for setting in [
        json!({ "level": "fatal" }),
        json!({ "level": 1 }),
        json!({ "level": "warn", "colour": true }),
        json!({ "level": "warn", "ignore": "assets/**" }),
        json!({ "level": "warn", "ignore": [1] }),
        json!({ "level": "warn", "inlineCode": true }),
    ] {
        let config = json!({ "version": 1, "validation": { "global": { "body.links": setting } } });
        let object = config_error(&get_a(&with_config(&config.to_string())));

        assert_eq!(
            details(&object),
            [pair("config.rule-unknown", ".typdoc/config.json")],
            "{setting}"
        );
    }
    let config = json!({ "version": 1, "validation": { "global": {
        "body.mentions": { "level": "warn", "inlineCode": "yes" } } } });
    let object = config_error(&get_a(&with_config(&config.to_string())));
    assert_eq!(
        details(&object),
        [pair("config.rule-unknown", ".typdoc/config.json")]
    );
}

#[test]
fn a_rule_that_is_always_on_and_is_configured_is_config_rule_always_on() {
    for rule in [
        "schema.valid",
        "frontmatter.parse",
        "frontmatter.types",
        "frontmatter.transitions",
        "refs.resolve",
        "refs.target",
        "refs.acyclic",
        "keys.unique",
        "collections.overlap",
        "state.missing",
    ] {
        let config =
            json!({ "version": 1, "validation": { "global": { rule: { "level": "off" } } } });
        let object = config_error(&get_a(&with_config(&config.to_string())));

        assert_eq!(
            details(&object),
            [pair("config.rule-always-on", ".typdoc/config.json")],
            "{rule}"
        );
        assert!(message_of(&object, 0).contains(rule));
    }
}

#[test]
fn the_rules_of_a_collection_are_checked_the_same_way_and_name_the_collection_file() {
    let project = with_collection(
        ".typdoc/collections/notes.json",
        r#"{ "match": "*.md", "schema": "note.json", "validation": {
            "keys.unique": { "level": "off" }, "body.linkz": { "level": "off" } } }"#,
    );

    let object = config_error(&get_a(&project));

    assert_eq!(
        details(&object),
        [
            pair("config.rule-always-on", ".typdoc/collections/notes.json"),
            pair("config.rule-unknown", ".typdoc/collections/notes.json"),
        ]
    );
}

#[test]
fn every_configurable_rule_with_its_options_is_read() {
    let config = json!({ "version": 1, "validation": { "global": {
        "body.links": { "level": "error", "ignore": ["assets/**", "generated/**"] },
        "body.anchors": { "level": "warn" },
        "body.mentions": { "level": "warn", "inlineCode": true, "fencedCode": false },
        "refs.codedByPath": { "level": "off" },
        "refs.moved": { "level": "error" },
        "names.shadowed": { "level": "warn" },
        "frontmatter.unknown": { "level": "off" },
        "filename.pattern": { "level": "error" },
        "imports.absent": { "level": "error" }
    } } });
    let project = with_config(&config.to_string());
    project.file("a.md", "");

    let ran = get_a(&project);

    assert_eq!(ran.code, 0, "{}", ran.stderr);
}

#[test]
fn a_rule_may_state_only_the_options_that_differ() {
    let config = json!({ "version": 1, "validation": { "global": { "body.mentions": { "fencedCode": true } } } });
    let project = with_config(&config.to_string());
    project.file("a.md", "");

    assert_eq!(get_a(&project).code, 0);
}

#[test]
fn a_validation_key_that_is_not_global_is_config_unknown_key() {
    let config = json!({ "version": 1, "validation": { "collections": {} } });

    let object = config_error(&get_a(&with_config(&config.to_string())));

    assert_eq!(
        details(&object),
        [pair("config.unknown-key", ".typdoc/config.json")]
    );
}

#[test]
fn a_value_of_the_wrong_kind_in_a_key_of_config_json_is_config_parse() {
    for config in [
        json!({ "version": 1, "validation": [] }),
        json!({ "version": 1, "validation": { "global": [] } }),
        json!({ "version": 1, "validation": { "global": { "body.links": "error" } } }),
        json!({ "version": 1, "lock": "elsewhere" }),
        json!({ "version": 1, "lock": 1 }),
    ] {
        let object = config_error(&get_a(&with_config(&config.to_string())));

        assert_eq!(
            details(&object),
            [pair("config.parse", ".typdoc/config.json")],
            "{config}"
        );
        assert_eq!(object["complete"], json!(false), "{config}");
    }
}

#[test]
fn imports_and_lock_are_keys_of_the_file_and_are_not_called_unknown() {
    let config =
        json!({ "version": 1, "imports": { "docs": "${DOCS_DIR}/docs" }, "lock": "git-common" });
    let project = with_config(&config.to_string());
    project.file("a.md", "");

    assert_eq!(get_a(&project).code, 0);
}

#[test]
fn every_error_that_can_be_determined_is_reported_in_one_object_in_the_order_of_path_rule_message()
{
    let project = with_config(
        r#"{ "version": 1, "name": "x", "validation": { "global": { "keys.unique": {} } } }"#,
    );
    project.file(".typdoc/collections/b.json", "{");
    project.file(
        ".typdoc/collections/a.json",
        r#"{ "match": "*.md", "schema": "note.json", "last": 1 }"#,
    );
    project.file(
        ".typdoc/collections/bad name.json",
        r#"{ "match": "*.md", "schema": "note.json" }"#,
    );

    let object = config_error(&get_a(&project));

    assert_eq!(
        details(&object),
        [
            pair("config.unknown-key", ".typdoc/collections/a.json"),
            pair("config.collection-parse", ".typdoc/collections/b.json"),
            pair(
                "config.collection-name",
                ".typdoc/collections/bad name.json"
            ),
            pair("config.rule-always-on", ".typdoc/config.json"),
            pair("config.unknown-key", ".typdoc/config.json"),
        ]
    );
    assert_eq!(object["complete"], json!(true));
}

#[test]
fn a_config_error_stops_the_command_before_any_document_is_read() {
    let project = with_config(r#"{ "version": 1, "name": "x" }"#);
    project.file("a.md", "---\nnever closed\n");

    let ran = get_a(&project);

    let object = config_error(&ran);
    assert_eq!(
        details(&object),
        [pair("config.unknown-key", ".typdoc/config.json")]
    );
}

#[test]
fn an_error_that_is_not_a_config_error_has_no_complete_key() {
    let ran = Spawn::args(["get", "absent.md", "--json"])
        .cwd(fixture("valid/minimal"))
        .run();

    assert_eq!(ran.code, 5);
    assert!(ran.stderr_json().get("complete").is_none());
}

/// The fixtures for the config errors, each run as its `fixture.json` says.
fn broken_config_runs() -> Vec<(String, Ran)> {
    let ids: Vec<String> = typdoc_testkit::fixtures::broken_entries()
        .into_iter()
        .filter(|id| id.starts_with("config."))
        .collect();
    assert!(!ids.is_empty(), "no fixture for a config error");
    ids.into_iter()
        .map(|id| {
            let dir = fixture("broken").join(&id);
            let (_spec, ran) = common::spawn_fixture(&dir, &id).expect("the fixture runs");
            (id, ran)
        })
        .collect()
}

#[test]
fn every_fixture_of_a_config_error_ends_with_2_and_the_error_object_on_standard_error() {
    for (id, ran) in broken_config_runs() {
        let object = config_error(&ran);

        let ids: Vec<String> = details(&object).into_iter().map(|(rule, _)| rule).collect();
        assert_eq!(ids, std::slice::from_ref(&id), "{id}");
    }
}

#[test]
fn only_a_config_that_cannot_be_parsed_and_an_unknown_version_say_the_list_is_not_complete() {
    for (id, ran) in broken_config_runs() {
        let complete = config_error(&ran)["complete"].clone();

        let expected = !matches!(id.as_str(), "config.parse" | "config.version");
        assert_eq!(complete, json!(expected), "{id}");
    }
}

#[test]
fn a_config_error_in_a_fixture_names_the_file_it_is_about() {
    let runs = broken_config_runs();
    for (id, path) in [
        ("config.parse", ".typdoc/config.json"),
        ("config.version", ".typdoc/config.json"),
        ("config.unknown-key", ".typdoc/config.json"),
        ("config.collection-parse", ".typdoc/collections/notes.json"),
        (
            "config.collection-name",
            ".typdoc/collections/my notes.json",
        ),
        ("config.rule-unknown", ".typdoc/config.json"),
        ("config.rule-always-on", ".typdoc/config.json"),
        ("config.namespaces-entry", ".typdoc/config.json"),
        ("config.namespace-name", ".typdoc/config.json"),
        ("config.namespace-nested", ".typdoc/config.json"),
        ("config.config-dir", ".typdoc/config.json"),
    ] {
        let (_, ran) = runs.iter().find(|(found, _)| found == id).expect(id);

        assert_eq!(details(&config_error(ran)), [pair(id, path)], "{id}");
    }
}
