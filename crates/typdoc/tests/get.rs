mod common;

use common::{NOTES, Scratch, Spawn, fixture};
use serde_json::json;

#[test]
fn get_by_path_prints_the_document() {
    let ran = Spawn::args(["get", "note.md", "--json"])
        .cwd(fixture("valid/minimal"))
        .run();

    assert_eq!(ran.code, 0, "stderr: {}", ran.stderr);
    assert_eq!(ran.stderr, "");
    assert_eq!(
        ran.stdout_json(),
        json!({
            "document": {
                "path": "note.md",
                "namespace": "default",
                "code": null,
                "collection": "notes",
                "schema": "note",
                "fields": { "title": "A minimal note", "tags": ["alpha", "beta"] }
            }
        })
    );
}

fn error_of(ran: &common::Ran, code: i32) -> serde_json::Value {
    assert_eq!(ran.code, code, "stderr: {}", ran.stderr);
    assert_eq!(ran.stdout, "", "a failure prints nothing on stdout");
    let object = ran.stderr_json();
    assert_eq!(object["code"], json!(code));
    assert!(object["error"].is_string(), "{object}");
    object
}

#[test]
fn a_document_that_is_not_there_exits_5() {
    let ran = Spawn::args(["get", "absent.md", "--json"])
        .cwd(fixture("valid/minimal"))
        .run();

    error_of(&ran, 5);
}

#[test]
fn a_path_that_differs_from_the_file_in_case_only_is_not_found() {
    let ran = Spawn::args(["get", "Note.md", "--json"])
        .cwd(fixture("valid/minimal"))
        .run();

    error_of(&ran, 5);
}

#[test]
fn a_bare_path_is_read_from_the_project_folder_even_when_the_current_directory_has_a_file_of_the_same_name()
 {
    let project = Scratch::project(&NOTES);
    project.file("note.md", "---\ntitle: the project's own copy\n---\n");
    project.file("sub/note.md", "not the file this path names");

    let ran = Spawn::args(["get", "note.md", "--json"])
        .cwd(project.path().join("sub"))
        .run();

    assert_eq!(ran.code, 0, "stderr: {}", ran.stderr);
    assert_eq!(
        ran.stdout_json()["document"]["fields"]["title"],
        json!("the project's own copy")
    );
}

#[test]
fn a_missing_path_says_the_same_name_exists_relative_to_the_current_directory_when_it_does() {
    let project = Scratch::project(&NOTES);
    project.file("sub/only-here.md", "");

    let ran = Spawn::args(["get", "only-here.md", "--json"])
        .cwd(project.path().join("sub"))
        .run();

    let object = error_of(&ran, 5);
    assert!(
        object["error"].as_str().unwrap().contains("./only-here.md"),
        "{object}"
    );
}

#[test]
fn get_by_key_in_a_project_with_one_namespace() {
    let ran = Spawn::args(["get", "WF-1", "--json"])
        .cwd(fixture("valid/templates"))
        .run();

    assert_eq!(ran.code, 0, "stderr: {}", ran.stderr);
    let document = ran.stdout_json()["document"].clone();
    assert_eq!(document["path"], json!("tickets/WF-1.md"));
    assert_eq!(document["key"], json!("WF-1"));
    assert_eq!(document["code"], json!("WF"));
}

#[test]
fn a_key_that_does_not_exist_exits_5() {
    let ran = Spawn::args(["get", "WF-404", "--json"])
        .cwd(fixture("valid/templates"))
        .run();

    error_of(&ran, 5);
}

#[test]
fn an_argument_that_is_neither_a_path_nor_a_key_exits_1() {
    for argument in ["note", "note.txt", "wf-3"] {
        let ran = Spawn::args(["get", argument, "--json"])
            .cwd(fixture("valid/minimal"))
            .run();

        error_of(&ran, 1);
    }
}

#[test]
fn a_namespace_prefix_on_a_path_that_is_not_a_namespace_of_a_one_namespace_project_exits_1() {
    let ran = Spawn::args(["get", "story-2:notes/x.md", "--json"])
        .cwd(fixture("valid/minimal"))
        .run();

    let object = error_of(&ran, 1);
    assert!(
        object["error"].as_str().unwrap().contains("story-2"),
        "{object}"
    );
}

#[test]
fn an_argument_that_names_an_imported_project_exits_1() {
    let ran = Spawn::args(["get", "chief::WF-3", "--json"])
        .cwd(fixture("valid/minimal"))
        .run();

    let object = error_of(&ran, 1);
    assert!(
        object["error"].as_str().unwrap().contains("chief::WF-3"),
        "{object}"
    );
}

#[test]
fn a_path_relative_to_the_current_directory_is_read_the_same_as_one_relative_to_the_project() {
    for argument in ["./note.md", "../minimal/note.md"] {
        let ran = Spawn::args(["get", argument, "--json"])
            .cwd(fixture("valid/minimal"))
            .run();

        assert_eq!(ran.code, 0, "{argument}: {}", ran.stderr);
        assert_eq!(
            ran.stdout_json()["document"]["path"],
            json!("note.md"),
            "{argument}"
        );
    }
}

#[test]
fn an_absolute_path_names_the_project_by_itself_and_wins_over_typdoc_dir() {
    let project = Scratch::project(&NOTES);
    project.file("note.md", "---\ntitle: x\n---\n");
    let elsewhere = fixture("valid/minimal");
    let absolute = project.path().join("note.md");

    let ran = Spawn::args([
        "get",
        absolute.to_str().expect("a UTF-8 scratch path"),
        "--json",
    ])
    .var("TYPDOC_DIR", elsewhere.to_str().expect("UTF-8"))
    .cwd(&elsewhere)
    .run();

    assert_eq!(ran.code, 0, "stderr: {}", ran.stderr);
    assert_eq!(ran.stdout_json()["document"]["path"], json!("note.md"));
    assert_eq!(ran.stdout_json()["document"]["fields"]["title"], json!("x"));
}

#[test]
fn an_on_disk_path_below_a_folder_with_no_project_above_it_is_not_found() {
    let scratch = tempfile::tempdir().expect("a scratch folder");
    for ancestor in scratch.path().ancestors() {
        assert!(
            !ancestor.join(".typdoc/config.json").is_file(),
            "the temporary folder is below a project, at {}, so this test cannot run",
            ancestor.display()
        );
    }
    std::fs::write(scratch.path().join("note.md"), "").expect("a file");

    let ran = Spawn::args(["get", "./note.md", "--json"])
        .cwd(scratch.path())
        .run();

    error_of(&ran, 5);
}

#[test]
fn a_call_with_no_document_exits_1() {
    let ran = Spawn::args(["get", "--json"])
        .cwd(fixture("valid/minimal"))
        .run();

    error_of(&ran, 1);
}

#[test]
fn the_output_without_json_is_not_built_and_exits_1() {
    let ran = Spawn::args(["get", "note.md"])
        .cwd(fixture("valid/minimal"))
        .run();

    assert_eq!(ran.code, 1);
    assert_eq!(ran.stdout, "");
    assert!(ran.stderr.starts_with("typdoc: "), "{}", ran.stderr);
}

#[test]
fn the_project_is_found_from_a_folder_below_it() {
    let ran = Spawn::args(["get", "note.md", "--json"])
        .cwd(fixture("valid/minimal").join(".typdoc/collections"))
        .run();

    assert_eq!(ran.code, 0, "stderr: {}", ran.stderr);
    assert_eq!(ran.stdout_json()["document"]["path"], json!("note.md"));
}

#[test]
fn typdoc_dir_names_the_project_from_anywhere() {
    let ran = Spawn::args(["get", "note.md", "--json"])
        .var("TYPDOC_DIR", "valid/minimal")
        .cwd(fixture(""))
        .run();

    assert_eq!(ran.code, 0, "stderr: {}", ran.stderr);
    assert_eq!(ran.stdout_json()["document"]["path"], json!("note.md"));
}

#[test]
fn typdoc_dir_skips_the_walk_and_a_folder_without_a_project_is_not_found() {
    let inside = fixture("valid/minimal");
    let ran = Spawn::args(["get", "note.md", "--json"])
        .var("TYPDOC_DIR", "schemas")
        .cwd(&inside)
        .run();

    error_of(&ran, 5);
}

#[test]
fn a_file_with_no_frontmatter_is_a_document_with_no_fields() {
    let project = Scratch::project(&NOTES);
    project.file("plain.md", "# Only a heading\n");

    let ran = Spawn::args(["get", "plain.md", "--json"])
        .cwd(project.path())
        .run();

    assert_eq!(ran.code, 0, "stderr: {}", ran.stderr);
    assert_eq!(ran.stdout_json()["document"]["fields"], json!({}));
}

#[test]
fn an_empty_block_is_a_document_with_no_fields() {
    let project = Scratch::project(&NOTES);
    project.file("empty.md", "---\n---\nBody\n");

    let ran = Spawn::args(["get", "empty.md", "--json"])
        .cwd(project.path())
        .run();

    assert_eq!(ran.code, 0, "stderr: {}", ran.stderr);
    assert_eq!(ran.stdout_json()["document"]["fields"], json!({}));
}

#[test]
fn a_block_that_is_never_closed_exits_2() {
    let project = Scratch::project(&NOTES);
    project.file("open.md", "---\ntitle: x\n");

    let ran = Spawn::args(["get", "open.md", "--json"])
        .cwd(project.path())
        .run();

    error_of(&ran, 2);
}

#[test]
fn a_field_written_twice_exits_2() {
    let project = Scratch::project(&NOTES);
    project.file("twice.md", "---\ntitle: a\ntitle: b\n---\n");

    let ran = Spawn::args(["get", "twice.md", "--json"])
        .cwd(project.path())
        .run();

    error_of(&ran, 2);
}

#[test]
fn the_fields_keep_the_order_of_the_file() {
    let project = Scratch::project(&NOTES);
    project.file("order.md", "---\nzeta: z\nalpha: a\nmid: m\n---\n");

    let ran = Spawn::args(["get", "order.md", "--json"])
        .cwd(project.path())
        .run();

    assert_eq!(ran.code, 0, "stderr: {}", ran.stderr);
    let fields = ran.stdout_json()["document"]["fields"].clone();
    let names: Vec<&String> = fields.as_object().expect("fields").keys().collect();
    assert_eq!(names, ["zeta", "alpha", "mid"]);
}

#[test]
fn a_config_with_an_unknown_version_exits_2() {
    let project = Scratch::project(&NOTES);
    project.file(".typdoc/config.json", r#"{ "version": 2 }"#);

    let ran = Spawn::args(["get", "a.md", "--json"])
        .cwd(project.path())
        .run();

    error_of(&ran, 2);
}

#[test]
fn a_config_that_cannot_be_parsed_exits_2() {
    let project = Scratch::project(&NOTES);
    project.file(".typdoc/config.json", "{ version");

    let ran = Spawn::args(["get", "a.md", "--json"])
        .cwd(project.path())
        .run();

    error_of(&ran, 2);
}

#[test]
fn a_remote_schema_is_refused_and_not_read_as_a_path() {
    let project = Scratch::project(&NOTES);
    project.file(
        ".typdoc/collections/notes.json",
        r#"{ "match": "*.md", "schema": "https://example.invalid/note.json" }"#,
    );

    let ran = Spawn::args(["get", "a.md", "--json"])
        .cwd(project.path())
        .run();

    error_of(&ran, 2);
}

#[test]
fn a_file_matched_by_two_collections_exits_2() {
    let project = Scratch::project(&NOTES);
    project.file(
        ".typdoc/collections/all.json",
        r#"{ "match": "a*", "schema": "note.json" }"#,
    );
    project.file("a.md", "");

    let ran = Spawn::args(["get", "a.md", "--json"])
        .cwd(project.path())
        .run();

    error_of(&ran, 2);
}

#[test]
fn a_symbolic_link_that_a_collection_matches_is_refused_and_not_followed() {
    let project = Scratch::project(&NOTES);
    project.file("real.txt", "---\ntitle: x\n---\n");
    project.symlink("link.md", "real.txt");

    let ran = Spawn::args(["get", "link.md", "--json"])
        .cwd(project.path())
        .run();

    error_of(&ran, 6);
}

#[test]
fn a_symbolic_link_that_no_collection_matches_is_left_alone() {
    let project = Scratch::project(&NOTES);
    project.file("a.md", "---\ntitle: x\n---\n");
    project.file("real.txt", "");
    project.symlink("link.txt", "real.txt");

    let ran = Spawn::args(["get", "a.md", "--json"])
        .cwd(project.path())
        .run();

    assert_eq!(ran.code, 0, "stderr: {}", ran.stderr);
}

#[test]
fn a_folder_that_a_collection_matches_is_not_a_document_and_is_not_entered() {
    let project = Scratch::project(&NOTES);
    project.file("folder.md/inner.md", "---\ntitle: x\n---\n");

    let outer = Spawn::args(["get", "folder.md", "--json"])
        .cwd(project.path())
        .run();
    let inner = Spawn::args(["get", "folder.md/inner.md", "--json"])
        .cwd(project.path())
        .run();

    error_of(&outer, 5);
    error_of(&inner, 5);
}

#[test]
fn a_file_below_the_project_folder_is_not_in_a_collection_that_matches_names_there() {
    let project = Scratch::project(&NOTES);
    project.file("deeper/a.md", "---\ntitle: x\n---\n");

    let ran = Spawn::args(["get", "deeper/a.md", "--json"])
        .cwd(project.path())
        .run();

    error_of(&ran, 5);
}

#[test]
fn a_name_that_is_not_utf8_and_that_a_collection_matches_is_refused() {
    let project = Scratch::project(&NOTES);
    project.file("a.md", "---\ntitle: x\n---\n");
    project.file_named_by_bytes(b"\xff.md", "");

    let ran = Spawn::args(["get", "a.md", "--json"])
        .cwd(project.path())
        .run();

    error_of(&ran, 6);
}

#[test]
fn a_name_that_is_not_utf8_and_that_no_collection_matches_is_left_alone() {
    let project = Scratch::project(&NOTES);
    project.file("a.md", "---\ntitle: x\n---\n");
    project.file_named_by_bytes(b"\xff.txt", "");

    let ran = Spawn::args(["get", "a.md", "--json"])
        .cwd(project.path())
        .run();

    assert_eq!(ran.code, 0, "stderr: {}", ran.stderr);
}

#[test]
fn a_folder_with_no_project_above_it_is_not_found() {
    let scratch = tempfile::tempdir().expect("a scratch folder");
    for ancestor in scratch.path().ancestors() {
        assert!(
            !ancestor.join(".typdoc/config.json").is_file(),
            "the temporary folder is below a project, at {}, so this test cannot run",
            ancestor.display()
        );
    }

    let ran = Spawn::args(["get", "note.md", "--json"])
        .cwd(scratch.path())
        .run();

    error_of(&ran, 5);
}

#[test]
fn a_path_whose_first_segment_starts_with_a_dot_is_read_from_the_project_folder() {
    for argument in [".chief/note.md", ".hidden.md", "..two/note.md"] {
        let ran = Spawn::args(["get", argument, "--json"])
            .cwd(fixture("valid/minimal"))
            .run();

        error_of(&ran, 5);
    }
}

#[test]
fn a_schema_that_does_not_exist_is_a_config_error_that_names_the_collection_file() {
    let project = Scratch::project(&NOTES);
    project.file(
        ".typdoc/collections/notes.json",
        r#"{ "match": "*.md", "schema": "absent.json" }"#,
    );

    let ran = Spawn::args(["get", "a.md", "--json"])
        .cwd(project.path())
        .run();

    let object = error_of(&ran, 2);
    assert!(
        object["error"].as_str().unwrap().contains("notes.json"),
        "{object}"
    );
}

#[test]
fn a_test_cannot_override_the_constant_path_or_the_fresh_home() {
    for name in ["PATH", "HOME"] {
        let outcome = std::panic::catch_unwind(|| Spawn::args(["--version"]).var(name, "/x"));
        assert!(outcome.is_err(), "{name} was accepted");
    }
}
