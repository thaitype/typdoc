//! Covers SPC-2, SPC-6, SPC-7, SPC-12, SPC-13, SPC-14, SPC-15.
//!
//! Where the machine file `imports.json` is found is unit-tested in `typdoc-core`'s `imports`
//! module; this file reads and merges it through the binary.

#[allow(dead_code, reason = "each test file uses part of the shared helper")]
mod common;

use common::{NOTES, Ran, Scratch, Spawn, fixture};
use serde_json::{Value, json};
use typdoc::registry;

fn run(args: &[&str], cwd: &std::path::Path) -> Ran {
    Spawn::args(args.iter().copied()).cwd(cwd).run()
}

fn error_of(ran: &Ran, code: i32) -> Value {
    assert_eq!(
        ran.code, code,
        "stdout: {} stderr: {}",
        ran.stdout, ran.stderr
    );
    let object = ran.stderr_json();
    assert_eq!(object["code"], json!(code));
    object
}

// ---------------------------------------------------------------------------------------------
// `${NAME}` substitution: an unset or empty variable never becomes an empty string.
// ---------------------------------------------------------------------------------------------

/// In `fixtures/valid/imports/decoy`, `imports.trap_import` is `${TYPDOC_TEST_UNSET_VAR}notes`
/// and a real project sits at `decoy/notes`, so an unset variable read as an empty string would
/// resolve the import.
#[test]
fn an_unset_variable_in_an_import_path_never_becomes_an_empty_string() {
    let ran = run(&["refs", "a.md", "--json"], &fixture("valid/imports/decoy"));

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    let refs = ran.stdout_json()["refs"].clone();
    let refs = refs.as_array().unwrap();
    assert_eq!(refs.len(), 1, "{refs:?}");
    assert_eq!(refs[0]["unresolved"], json!("import-absent"));
    assert!(refs[0].get("path").is_none(), "{:?}", refs[0]);
}

#[test]
fn a_variable_set_to_an_empty_value_behaves_as_unset() {
    let ran = Spawn::args(["refs", "a.md", "--json"])
        .cwd(fixture("valid/imports/decoy"))
        .var("TYPDOC_TEST_UNSET_VAR", "")
        .run();

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    assert_eq!(
        ran.stdout_json()["refs"][0]["unresolved"],
        json!("import-absent")
    );
}

/// The ref then names a real project with no `x.md` in it, so `not-found` and `import-absent`
/// are shown to be different outcomes.
#[test]
fn a_variable_set_to_a_real_value_is_substituted_and_the_import_is_then_present() {
    let ran = Spawn::args(["refs", "a.md", "--json"])
        .cwd(fixture("valid/imports/decoy"))
        .var("TYPDOC_TEST_UNSET_VAR", "")
        .run();
    assert_eq!(ran.code, 0);
    let empty_case = ran.stdout_json()["refs"][0]["unresolved"].clone();

    let ran = Spawn::args(["refs", "a.md", "--json"])
        .cwd(fixture("valid/imports/decoy"))
        .var("TYPDOC_TEST_UNSET_VAR", "./")
        .run();
    assert_eq!(ran.code, 0, "{}", ran.stderr);
    let present_case = ran.stdout_json()["refs"][0]["unresolved"].clone();

    assert_eq!(empty_case, json!("import-absent"));
    assert_eq!(present_case, json!("not-found"));
}

// ---------------------------------------------------------------------------------------------
// `imports.absent`: `error` gives exit 2; an absent import nothing refers to is never reported,
// even at `error`.
// ---------------------------------------------------------------------------------------------

fn absent_import_project(level: &str) -> Scratch {
    let project = Scratch::project(&[
        (
            ".typdoc/collections/notes.json",
            r#"{ "match": "*.md", "schema": "note.json" }"#,
        ),
        (
            "note.json",
            r#"{ "name": "note", "fields": { "see": { "type": "ref", "target": "*" } } }"#,
        ),
    ]);
    project.file(
        ".typdoc/config.json",
        &format!(
            r#"{{ "version": 1, "imports": {{ "gone_away": "./nowhere" }},
                  "validation": {{ "global": {{ "imports.absent": {{ "level": "{level}" }} }} }} }}"#
        ),
    );
    project
}

#[test]
fn imports_absent_at_error_gives_exit_2_when_a_ref_names_the_absent_import() {
    let project = absent_import_project("error");
    project.file("a.md", "---\nsee: gone_away::x.md\n---\n");

    let ran = run(&["validate", "--json"], project.path());

    // Exit 2 from `validate` is a verdict, printed on stdout, not an error object on stderr.
    assert_eq!(ran.code, 2, "{}", ran.stderr);
    assert_eq!(ran.stderr, "");
    let report = ran.stdout_json();
    assert_eq!(report["summary"]["findings"]["error"], json!(1));
    let findings = report["findings"].as_array().unwrap();
    assert_eq!(findings.len(), 1, "{findings:?}");
    assert_eq!(findings[0]["rule"], json!("imports.absent"));
    assert_eq!(findings[0]["level"], json!("error"));
}

#[test]
fn an_absent_import_that_nothing_refers_to_is_not_reported_even_at_error() {
    let project = absent_import_project("error");
    project.file("a.md", "");

    let ran = run(&["validate", "--json"], project.path());

    assert_eq!(
        ran.code, 0,
        "warn or error, an unused import is silent: {}",
        ran.stderr
    );
    let findings = ran.stdout_json()["findings"].clone();
    assert_eq!(findings.as_array().unwrap().len(), 0, "{findings}");
}

#[test]
fn imports_absent_default_is_warn_and_exits_0() {
    let project = absent_import_project("warn");
    project.file("a.md", "---\nsee: gone_away::x.md\n---\n");

    let ran = run(&["validate", "--json"], project.path());

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    let findings = ran.stdout_json()["findings"].clone();
    let findings = findings.as_array().unwrap();
    assert_eq!(findings.len(), 1, "{findings:?}");
    assert_eq!(findings[0]["rule"], json!("imports.absent"));
    assert_eq!(findings[0]["level"], json!("warn"));
}

// ---------------------------------------------------------------------------------------------
// `name::` resolution: unknown alias is `bad-prefix`, absent is `import-absent`, present
// resolves — through `fixtures/valid/imports/main`, which imports `memory_import` (present,
// one namespace), `several_import` (present, several namespaces — reuses `valid/several-
// namespaces`), and `gone_import` (configured but absent: `./not-a-real-project` does not
// exist).
// ---------------------------------------------------------------------------------------------

fn main_project() -> std::path::PathBuf {
    fixture("valid/imports/main")
}

#[test]
fn an_alias_this_project_does_not_configure_is_bad_prefix() {
    let project = Scratch::project(&[
        (
            ".typdoc/collections/notes.json",
            r#"{ "match": "*.md", "schema": "note.json" }"#,
        ),
        (
            "note.json",
            r#"{ "name": "note", "fields": { "see": { "type": "ref", "target": "*" } } }"#,
        ),
    ]);
    project.file("a.md", "---\nsee: nosuch::x.md\n---\n");

    let ran = run(&["refs", "a.md", "--json"], project.path());

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    assert_eq!(
        ran.stdout_json()["refs"][0]["unresolved"],
        json!("bad-prefix")
    );
}

#[test]
fn an_alias_configured_but_absent_on_this_machine_is_import_absent() {
    let ran = run(&["refs", "absent.md", "--json"], &main_project());

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    let refs = ran.stdout_json()["refs"].clone();
    let refs = refs.as_array().unwrap();
    assert_eq!(refs.len(), 2, "{refs:?}");
    for reference in refs {
        assert_eq!(
            reference["unresolved"],
            json!("import-absent"),
            "{reference}"
        );
    }
}

#[test]
fn an_alias_configured_and_present_resolves_and_carries_project_in_its_name() {
    let ran = run(&["refs", "note.md", "--json"], &main_project());

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    let refs = ran.stdout_json()["refs"].clone();
    let refs = refs.as_array().unwrap();
    assert_eq!(refs.len(), 4, "{refs:?}");
    for reference in refs {
        assert!(reference["path"].as_str().is_some(), "{reference}");
        assert!(reference.get("unresolved").is_none(), "{reference}");
        assert!(reference.get("project").is_some(), "{reference}");
    }
    let by_field: Vec<(&str, &str)> = refs
        .iter()
        .map(|r| (r["field"].as_str().unwrap(), r["project"].as_str().unwrap()))
        .collect();
    assert!(by_field.contains(&("see", "memory_import")), "{by_field:?}");
    assert!(
        by_field.contains(&("context", "several_import")),
        "{by_field:?}"
    );
}

// ---------------------------------------------------------------------------------------------
// `project::` arguments
// ---------------------------------------------------------------------------------------------

#[test]
fn get_with_a_project_prefix_resolves_inside_the_import_and_names_it() {
    let ran = run(&["get", "memory_import::LRN-1", "--json"], &main_project());

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    let document = ran.stdout_json()["document"].clone();
    assert_eq!(document["path"], json!("learnings/LRN-1.md"));
    assert_eq!(document["key"], json!("LRN-1"));
    assert_eq!(document["project"], json!("memory_import"));
    assert_eq!(document["fields"]["title"], json!("A memory learning"));
}

#[test]
fn toc_with_a_project_prefix_resolves_inside_the_import_and_names_it() {
    let ran = run(&["toc", "memory_import::LRN-1", "--json"], &main_project());

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    let document = ran.stdout_json()["document"].clone();
    assert_eq!(document["project"], json!("memory_import"));
    assert_eq!(
        document,
        json!({ "path": "learnings/LRN-1.md", "namespace": "default", "key": "LRN-1", "project": "memory_import" })
    );
}

#[test]
fn refs_with_a_project_prefix_reads_the_imported_documents_own_out_refs() {
    // `LRN-1` holds no refs, so this checks only that the command runs inside the import.
    let ran = run(&["refs", "memory_import::LRN-1", "--json"], &main_project());

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    let out = ran.stdout_json();
    assert_eq!(
        out["document"],
        json!({ "path": "learnings/LRN-1.md", "namespace": "default", "key": "LRN-1", "project": "memory_import" })
    );
    assert_eq!(out["refs"], json!([]));
}

#[test]
fn refs_reverse_with_a_project_prefix_is_refused() {
    let ran = run(
        &["refs", "memory_import::LRN-1", "--reverse", "--json"],
        &main_project(),
    );

    let object = error_of(&ran, 1);
    assert!(
        object["error"].as_str().unwrap().contains("--reverse"),
        "{object}"
    );
}

#[test]
fn validate_with_a_project_prefix_is_refused() {
    let ran = run(
        &["validate", "memory_import::LRN-1", "--json"],
        &main_project(),
    );

    let object = error_of(&ran, 1);
    assert!(
        object["error"]
            .as_str()
            .unwrap()
            .contains("validated only by the project that owns it"),
        "{object}"
    );
}

/// `validate` never reads an import, so a scope that names one is refused rather than giving a
/// clean report for a scope that was never read.
#[test]
fn validate_of_a_named_argument_refuses_a_namespace_flag_naming_an_import() {
    let ran = run(
        &[
            "validate",
            "note.md",
            "--namespace",
            "several_import::*",
            "--json",
        ],
        &main_project(),
    );

    let object = error_of(&ran, 1);
    assert!(
        object["error"]
            .as_str()
            .unwrap()
            .contains("never reads one"),
        "{object}"
    );
}

#[test]
fn validate_of_a_named_argument_refuses_typdoc_namespace_naming_an_import() {
    let ran = Spawn::args(["validate", "note.md", "--json"])
        .cwd(main_project())
        .var("TYPDOC_NAMESPACE", "several_import::*")
        .run();

    let object = error_of(&ran, 1);
    assert!(
        object["error"]
            .as_str()
            .unwrap()
            .contains("never reads one"),
        "{object}"
    );
}

#[test]
fn a_key_form_import_into_a_multi_namespace_project_must_name_the_namespace() {
    let with_namespace = run(
        &["get", "several_import::story-2:WF-9", "--json"],
        &main_project(),
    );
    assert_eq!(with_namespace.code, 0, "{}", with_namespace.stderr);
    let document = with_namespace.stdout_json()["document"].clone();
    assert_eq!(document["key"], json!("WF-9"));
    assert_eq!(document["namespace"], json!("story-2"));
    assert_eq!(document["project"], json!("several_import"));

    let without_namespace = run(&["get", "several_import::WF-9", "--json"], &main_project());
    let object = error_of(&without_namespace, 1);
    assert!(
        object["error"].as_str().unwrap().contains("namespace"),
        "{object}"
    );
}

#[test]
fn a_project_prefix_naming_no_import_of_this_project_is_bad_arguments() {
    let ran = run(&["get", "nosuch::WF-1", "--json"], &main_project());

    let object = error_of(&ran, 1);
    assert!(
        object["error"].as_str().unwrap().contains("nosuch"),
        "{object}"
    );
}

#[test]
fn a_project_prefix_naming_an_import_absent_on_this_machine_is_bad_arguments() {
    let ran = run(&["get", "gone_import::x.md", "--json"], &main_project());

    let object = error_of(&ran, 1);
    assert!(
        object["error"].as_str().unwrap().contains("gone_import"),
        "{object}"
    );
}

// ---------------------------------------------------------------------------------------------
// `--namespace 'alias::*'`
// ---------------------------------------------------------------------------------------------

#[test]
fn namespace_flag_with_an_import_alias_lists_that_imports_documents_tagged_with_project() {
    let ran = run(
        &["list", "--namespace", "several_import::*", "--json"],
        &main_project(),
    );

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    let out = ran.stdout_json();
    assert_eq!(out["total"], json!(6));
    let documents = out["documents"].as_array().unwrap();
    assert!(
        documents
            .iter()
            .all(|d| d["project"] == json!("several_import")),
        "{documents:?}"
    );
}

#[test]
fn namespace_flag_can_combine_this_projects_own_namespace_with_an_import() {
    let ran = run(
        &["list", "--namespace", "default,several_import::*", "--json"],
        &main_project(),
    );

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    let out = ran.stdout_json();
    // 3 documents of `main` itself (`note.md`, `absent.md`, `uncollected.md`) plus 6 of
    // `several_import`.
    assert_eq!(out["total"], json!(9));
    let documents = out["documents"].as_array().unwrap();
    let own = documents.iter().filter(|d| d["project"].is_null()).count();
    let imported = documents
        .iter()
        .filter(|d| d["project"] == json!("several_import"))
        .count();
    assert_eq!((own, imported), (3, 6), "{documents:?}");
}

#[test]
fn namespace_flag_naming_an_unknown_import_is_bad_arguments() {
    let ran = run(
        &["list", "--namespace", "nosuch::*", "--json"],
        &main_project(),
    );

    let object = error_of(&ran, 1);
    assert!(
        object["error"].as_str().unwrap().contains("nosuch"),
        "{object}"
    );
}

#[test]
fn namespace_flag_naming_an_absent_import_is_bad_arguments() {
    let ran = run(
        &["list", "--namespace", "gone_import::*", "--json"],
        &main_project(),
    );

    let object = error_of(&ran, 1);
    assert!(
        object["error"].as_str().unwrap().contains("gone_import"),
        "{object}"
    );
}

// ---------------------------------------------------------------------------------------------
// A `ref.*(f)` condition across an import
// ---------------------------------------------------------------------------------------------

#[test]
fn a_ref_condition_follows_an_arrow_across_an_import_to_a_real_document() {
    let ran = run(
        &[
            "list",
            "--where",
            "ref.any(see).collection=learnings",
            "--json",
        ],
        &main_project(),
    );

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    let out = ran.stdout_json();
    assert_eq!(out["total"], json!(1));
    assert_eq!(out["documents"][0]["path"], json!("note.md"));
}

#[test]
fn a_ref_condition_reaches_a_document_of_an_import_that_is_in_no_collection() {
    // `named_import`'s one namespace is `only`, and `main`'s is `default`. Read under `main`,
    // `named_import::only/uncollected.md` would still carry the right `path`, so the namespace
    // is what shows which project it was read under.
    let ran = run(
        &[
            "list",
            "--where",
            "ref.any(context).namespace=only",
            "--json",
        ],
        &main_project(),
    );

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    let out = ran.stdout_json();
    assert_eq!(out["total"], json!(1));
    assert_eq!(out["documents"][0]["path"], json!("uncollected.md"));
}

// ---------------------------------------------------------------------------------------------
// One level of import only. `memory_import` imports `nowhere-nested`, whose config is not valid
// JSON, so following it would fail to load `main`.
// ---------------------------------------------------------------------------------------------

#[test]
fn an_imports_own_import_is_never_followed() {
    let ran = run(&["get", "note.md", "--json"], &main_project());

    assert_eq!(ran.code, 0, "{}", ran.stderr);
}

// ---------------------------------------------------------------------------------------------
// The machine file `imports.json`
// ---------------------------------------------------------------------------------------------

fn tiny_project(dir: &std::path::Path, title: &str) {
    std::fs::create_dir_all(dir.join(".typdoc/collections")).unwrap();
    std::fs::write(dir.join(".typdoc/config.json"), r#"{ "version": 1 }"#).unwrap();
    std::fs::write(
        dir.join(".typdoc/collections/notes.json"),
        r#"{ "match": "*.md", "schema": "note.json" }"#,
    )
    .unwrap();
    std::fs::write(
        dir.join("note.json"),
        r#"{ "name": "note", "fields": { "title": { "type": "string" } } }"#,
    )
    .unwrap();
    std::fs::write(dir.join("x.md"), format!("---\ntitle: {title}\n---\n")).unwrap();
}

#[test]
fn the_machine_file_supplies_an_alias_the_project_does_not_configure() {
    let config_dir = tempfile::tempdir().unwrap();
    let imported = tempfile::tempdir().unwrap();
    tiny_project(imported.path(), "from the machine file");
    std::fs::write(
        config_dir.path().join("imports.json"),
        format!(
            r#"{{ "viaMachine_file": {} }}"#,
            json!(imported.path().to_str().unwrap())
        ),
    )
    .unwrap();

    let project = Scratch::project(&[]);

    let ran = Spawn::args(["get", "viaMachine_file::x.md", "--json"])
        .cwd(project.path())
        .var("TYPDOC_CONFIG_DIR", config_dir.path().to_str().unwrap())
        .run();

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    assert_eq!(
        ran.stdout_json()["document"]["fields"]["title"],
        json!("from the machine file")
    );
}

#[test]
fn the_projects_own_imports_entry_wins_over_the_machine_files_for_the_same_alias() {
    let config_dir = tempfile::tempdir().unwrap();
    let from_project = tempfile::tempdir().unwrap();
    let from_machine = tempfile::tempdir().unwrap();
    tiny_project(from_project.path(), "from the project's own config");
    tiny_project(from_machine.path(), "from the machine file");
    std::fs::write(
        config_dir.path().join("imports.json"),
        format!(
            r#"{{ "sameAlias_import": {} }}"#,
            json!(from_machine.path().to_str().unwrap())
        ),
    )
    .unwrap();

    let project = Scratch::project(&[]);
    project.file(
        ".typdoc/config.json",
        &format!(
            r#"{{ "version": 1, "imports": {{ "sameAlias_import": {} }} }}"#,
            json!(from_project.path().to_str().unwrap())
        ),
    );

    let ran = Spawn::args(["get", "sameAlias_import::x.md", "--json"])
        .cwd(project.path())
        .var("TYPDOC_CONFIG_DIR", config_dir.path().to_str().unwrap())
        .run();

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    assert_eq!(
        ran.stdout_json()["document"]["fields"]["title"],
        json!("from the project's own config")
    );
}

fn validate_with_machine_file(project: &Scratch, machine: &str) -> Ran {
    let config_dir = tempfile::tempdir().unwrap();
    std::fs::write(config_dir.path().join("imports.json"), machine).unwrap();
    Spawn::args(["validate", "--json"])
        .cwd(project.path())
        .var("TYPDOC_CONFIG_DIR", config_dir.path().to_str().unwrap())
        .run()
}

#[test]
fn an_alias_the_machine_file_supplies_is_refused_when_it_is_a_url_scheme() {
    let project = Scratch::project(&NOTES);

    let ran = validate_with_machine_file(&project, r#"{ "mailto": "/nowhere" }"#);

    assert_eq!(ran.code, 2, "{}", ran.stderr);
    let findings = ran.stdout_json()["findings"].clone();
    let findings = findings.as_array().unwrap();
    assert_eq!(findings.len(), 1, "{findings:?}");
    assert_eq!(findings[0]["rule"], json!("schema.valid"));
    assert!(
        findings[0]["path"]
            .as_str()
            .unwrap()
            .ends_with("imports.json"),
        "{findings:?}"
    );
    assert!(
        findings[0]["message"]
            .as_str()
            .unwrap()
            .contains("`mailto`"),
        "{findings:?}"
    );
}

#[test]
fn an_alias_named_by_the_project_and_the_machine_file_is_refused_once_at_the_project() {
    let project = Scratch::project(&NOTES);
    project.file(
        ".typdoc/config.json",
        r#"{ "version": 1, "imports": { "file": "/nowhere" } }"#,
    );

    let ran = validate_with_machine_file(&project, r#"{ "file": "/elsewhere" }"#);

    let findings = ran.stdout_json()["findings"].clone();
    let findings = findings.as_array().unwrap();
    assert_eq!(findings.len(), 1, "{findings:?}");
    assert_eq!(findings[0]["path"], json!(".typdoc/config.json"));
}

// ---------------------------------------------------------------------------------------------
// `config.config-dir`. `fixtures/broken/config.config-dir` covers the relative case through the
// coverage harness; each case below fails one of the two conditions, absolute and exists.
// ---------------------------------------------------------------------------------------------

#[test]
fn typdoc_config_dir_that_is_not_absolute_is_config_dot_config_dir() {
    let project = Scratch::project(&[]);

    let ran = Spawn::args(["validate", "--json"])
        .cwd(project.path())
        .var("TYPDOC_CONFIG_DIR", "relative/place")
        .run();

    let object = error_of(&ran, 2);
    assert_eq!(object["details"][0]["rule"], json!("config.config-dir"));
    // Unlike `config.parse`, it does not stop the rest of `config.json` from being read.
    assert_eq!(object["complete"], json!(true));
}

#[test]
fn typdoc_config_dir_naming_a_directory_that_does_not_exist_is_config_dot_config_dir() {
    let project = Scratch::project(&[]);
    let missing = project.path().join("no-such-directory");

    let ran = Spawn::args(["validate", "--json"])
        .cwd(project.path())
        .var("TYPDOC_CONFIG_DIR", missing.to_str().unwrap())
        .run();

    let object = error_of(&ran, 2);
    assert_eq!(object["details"][0]["rule"], json!("config.config-dir"));
}

// ---------------------------------------------------------------------------------------------
// `refs.target` and `refs.codedByPath` for a ref that crosses into an import
// ---------------------------------------------------------------------------------------------

fn coded_import_project(dir: &std::path::Path) {
    std::fs::create_dir_all(dir.join(".typdoc/collections")).unwrap();
    std::fs::write(dir.join(".typdoc/config.json"), r#"{ "version": 1 }"#).unwrap();
    std::fs::write(
        dir.join(".typdoc/collections/learnings.json"),
        r#"{ "match": "{key}.md", "schema": "learning.json" }"#,
    )
    .unwrap();
    std::fs::write(
        dir.join("learning.json"),
        r#"{ "name": "learning", "code": "LRN", "fields": {} }"#,
    )
    .unwrap();
    std::fs::write(dir.join("LRN-1.md"), "").unwrap();
    std::fs::write(
        dir.join(".typdoc/collections/other.json"),
        r#"{ "match": "other/*.md", "schema": "other.json" }"#,
    )
    .unwrap();
    std::fs::write(
        dir.join("other.json"),
        r#"{ "name": "other", "fields": {} }"#,
    )
    .unwrap();
}

/// `"*"` is written bare: `["*"]` would be a list of one schema named `*`, which nothing is.
fn importer_project(imported: &std::path::Path, target: &str) -> Scratch {
    let project = Scratch::project(&[]);
    project.file(
        ".typdoc/config.json",
        &json!({ "version": 1, "imports": { "memory_import": imported.to_str().unwrap() } })
            .to_string(),
    );
    project.file(
        ".typdoc/collections/notes.json",
        r#"{ "match": "*.md", "schema": "note.json" }"#,
    );
    let target_json = if target == "*" {
        json!("*")
    } else {
        json!([target])
    };
    project.file(
        "note.json",
        &json!({
            "name": "note",
            "fields": { "see": { "type": "ref", "target": target_json } }
        })
        .to_string(),
    );
    project
}

fn validate_json(project: &std::path::Path) -> Ran {
    Spawn::args(["validate", "--json"]).cwd(project).run()
}

#[test]
fn refs_target_allows_a_ref_that_crosses_an_import_to_the_qualified_schema_it_names() {
    let imported = tempfile::tempdir().unwrap();
    coded_import_project(imported.path());
    let project = importer_project(imported.path(), "memory_import::learning");
    project.file("a.md", "---\nsee: memory_import::LRN-1\n---\n");

    let ran = validate_json(project.path());

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    assert_eq!(ran.stdout_json()["findings"], json!([]));
}

#[test]
fn refs_target_now_refuses_a_ref_that_crosses_an_import_to_a_schema_the_qualified_target_excludes()
{
    let imported = tempfile::tempdir().unwrap();
    coded_import_project(imported.path());
    // `other` is a real schema of the imported project (so this is not also a schema-drift
    // fault), but the document actually named is a `learning`, which `target` does not list.
    let project = importer_project(imported.path(), "memory_import::other");
    project.file("a.md", "---\nsee: memory_import::LRN-1\n---\n");

    let ran = validate_json(project.path());

    assert_eq!(ran.code, 2, "{}", ran.stderr);
    let findings = ran.stdout_json()["findings"].clone();
    let findings = findings.as_array().unwrap();
    assert_eq!(findings.len(), 1, "{findings:?}");
    assert_eq!(findings[0]["rule"], json!("refs.target"));
}

#[test]
fn a_bare_name_in_target_does_not_reach_a_same_named_schema_across_an_import() {
    let imported = tempfile::tempdir().unwrap();
    coded_import_project(imported.path());
    let project = importer_project(imported.path(), "learning");
    project.file("a.md", "---\nsee: memory_import::LRN-1\n---\n");

    let ran = validate_json(project.path());

    assert_eq!(ran.code, 2, "{}", ran.stderr);
    let findings = ran.stdout_json()["findings"].clone();
    let findings = findings.as_array().unwrap();
    assert_eq!(findings.len(), 1, "{findings:?}");
    assert_eq!(findings[0]["rule"], json!("refs.target"));
}

#[test]
fn refs_coded_by_path_now_warns_for_a_coded_document_of_an_import_referenced_by_path() {
    let imported = tempfile::tempdir().unwrap();
    coded_import_project(imported.path());
    let project = importer_project(imported.path(), "*");
    project.file("a.md", "---\nsee: memory_import::LRN-1.md\n---\n");

    let ran = validate_json(project.path());

    assert_eq!(ran.code, 0, "warn does not fail the run: {}", ran.stderr);
    let findings = ran.stdout_json()["findings"].clone();
    let findings = findings.as_array().unwrap();
    assert_eq!(findings.len(), 1, "{findings:?}");
    assert_eq!(findings[0]["rule"], json!("refs.codedByPath"));
}

// ---------------------------------------------------------------------------------------------
// A gap `registry::KNOWN_GAPS` names: `refs --reverse` scans this project only, not the projects
// it imports. `a` and `b` import each other, so a document of `b` can point into `a` as any
// cross-import ref does.
// ---------------------------------------------------------------------------------------------

#[test]
fn a_reverse_lookup_does_not_see_a_ref_from_an_imported_project() {
    assert!(
        registry::KNOWN_GAPS
            .iter()
            .any(|gap| gap.starts_with("[reverse-scope]")),
        "this test pins a gap that `registry::KNOWN_GAPS` no longer lists"
    );

    let a = tempfile::tempdir().unwrap();
    let b = tempfile::tempdir().unwrap();
    let note_schema = |see: bool| {
        if see {
            json!({ "name": "note", "fields": { "see": { "type": "ref", "target": "*" } } })
                .to_string()
        } else {
            json!({ "name": "note", "fields": {} }).to_string()
        }
    };
    let write = |dir: &std::path::Path, imports_alias: &str, imported: &std::path::Path, see| {
        std::fs::create_dir_all(dir.join(".typdoc/collections")).unwrap();
        std::fs::write(
            dir.join(".typdoc/config.json"),
            json!({ "version": 1, "imports": { imports_alias: imported.to_str().unwrap() } })
                .to_string(),
        )
        .unwrap();
        std::fs::write(
            dir.join(".typdoc/collections/notes.json"),
            r#"{ "match": "*.md", "schema": "note.json" }"#,
        )
        .unwrap();
        std::fs::write(dir.join("note.json"), note_schema(see)).unwrap();
    };
    // `a` imports `b` only so that a reverse scan of imports would have somewhere to look.
    write(a.path(), "b_import", b.path(), false);
    std::fs::write(a.path().join("x.md"), "").unwrap();
    write(b.path(), "a_import", a.path(), true);
    std::fs::write(b.path().join("y.md"), "---\nsee: a_import::x.md\n---\n").unwrap();

    // The premise: `b`'s ref resolves. If it did not, an empty reverse scan would prove nothing.
    let from_b = run(&["refs", "y.md", "--json"], b.path());
    assert_eq!(from_b.code, 0, "{}", from_b.stderr);
    let out = from_b.stdout_json();
    assert_eq!(out["refs"][0]["path"], json!("x.md"));
    assert_eq!(out["refs"][0]["project"], json!("a_import"));

    // The gap: from `a`, a reverse lookup on `x.md` does not see that ref.
    let from_a = run(&["refs", "x.md", "--reverse", "--json"], a.path());
    assert_eq!(from_a.code, 0, "{}", from_a.stderr);
    assert_eq!(from_a.stdout_json()["refs"], json!([]));
}

// ---------------------------------------------------------------------------------------------
// A gap `registry::KNOWN_GAPS` names: a body link that crosses an import has its target's
// existence checked (`body.links`) but not its `#anchor` (`body.anchors`).
// ---------------------------------------------------------------------------------------------

#[test]
fn a_body_link_across_an_import_has_its_anchor_left_unchecked() {
    assert!(
        registry::KNOWN_GAPS
            .iter()
            .any(|gap| gap.starts_with("[import-anchor]")),
        "this test pins a gap that `registry::KNOWN_GAPS` no longer lists"
    );

    let imported = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(imported.path().join(".typdoc/collections")).unwrap();
    std::fs::write(
        imported.path().join(".typdoc/config.json"),
        r#"{ "version": 1 }"#,
    )
    .unwrap();
    std::fs::write(
        imported.path().join(".typdoc/collections/notes.json"),
        r#"{ "match": "*.md", "schema": "note.json" }"#,
    )
    .unwrap();
    std::fs::write(
        imported.path().join("note.json"),
        r#"{ "name": "note", "fields": {} }"#,
    )
    .unwrap();
    // The one real heading here is `real-heading`; `missing-heading` names nothing.
    std::fs::write(imported.path().join("target.md"), "# Real Heading\n").unwrap();

    let project = Scratch::project(&NOTES);
    project.file(
        ".typdoc/config.json",
        &json!({
            "version": 1,
            "imports": { "held_import": imported.path().to_str().unwrap() }
        })
        .to_string(),
    );
    project.file(
        "a.md",
        "[broken anchor](held_import::target.md#missing-heading)\n",
    );

    let ran = validate_json(project.path());

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    assert_eq!(ran.stdout_json()["findings"], json!([]));
}

// ---------------------------------------------------------------------------------------------
// The reverse index tells a document of this project from a document of an imported project
// that has the same namespace and path. `refs --reverse` and `refby.*` in `list` read it.
//
// Two projects, `a` importing `b`, each with `notes/target.md`. `a/notes/pointer.md` holds the
// ref under test. In the fault shape the ref is `b::notes/target.md`: it resolves, into `b`, and
// nothing in `a` points at `a/notes/target.md`. In the control it is `target.md`, which resolves
// from the folder of `pointer.md` to `a/notes/target.md`, so that document is pointed at. Only
// the project differs between the two, and a ref that resolves the same way in both would make
// the pair pass under any implementation that ignores the project.
// ---------------------------------------------------------------------------------------------

fn importer_and_imported(pointer_text: &str) -> (Scratch, Scratch) {
    let make = |imports: &str| {
        let project = Scratch::project(&[]);
        project.file(
            ".typdoc/config.json",
            &json!({ "version": 1, "imports": { "b": imports } }).to_string(),
        );
        project.file(
            ".typdoc/collections/notes.json",
            r#"{ "match": "notes/*.md", "schema": "note.json" }"#,
        );
        project.file(
            "note.json",
            r#"{ "name": "note", "fields": { "see": { "type": "ref", "target": "*" } } }"#,
        );
        project.file("notes/target.md", "---\n---\n");
        project
    };
    let imported = make("unused");
    let importer = make(imported.path().to_str().unwrap());
    importer.file("notes/pointer.md", pointer_text);
    (importer, imported)
}

fn reverse_refs_of_target(a: &Scratch, extra: &[&str]) -> Value {
    let mut args = vec!["refs", "notes/target.md", "--reverse", "--json"];
    args.extend_from_slice(extra);
    let ran = run(&args, a.path());
    assert_eq!(ran.code, 0, "{}", ran.stderr);
    let out = ran.stdout_json();
    assert_eq!(out["direction"], json!("in"));
    assert_eq!(out["document"]["path"], json!("notes/target.md"));
    out["refs"].clone()
}

fn listed_by(a: &Scratch, condition: &str) -> Vec<String> {
    let ran = run(&["list", "--where", condition, "--ids"], a.path());
    assert_eq!(ran.code, 0, "{}", ran.stderr);
    ran.stdout.lines().map(str::to_owned).collect()
}

fn assert_pointer_resolves_into_b(a: &Scratch, field: &str) {
    let ran = run(&["refs", "notes/pointer.md", "--json"], a.path());
    assert_eq!(ran.code, 0, "{}", ran.stderr);
    let refs = ran.stdout_json()["refs"].clone();
    assert_eq!(refs.as_array().map(Vec::len), Some(1), "{refs}");
    assert_eq!(refs[0]["field"], json!(field));
    assert_eq!(refs[0]["path"], json!("notes/target.md"));
    assert_eq!(refs[0]["project"], json!("b"));
    assert!(refs[0].get("unresolved").is_none(), "{refs}");
}

#[test]
fn a_ref_into_an_import_is_not_a_reverse_ref_of_the_same_path_here() {
    let (a, _b) = importer_and_imported("---\nsee: b::notes/target.md\n---\n");
    assert_pointer_resolves_into_b(&a, "see");

    assert_eq!(reverse_refs_of_target(&a, &[]), json!([]));
    assert_eq!(reverse_refs_of_target(&a, &["--field", "see"]), json!([]));
}

#[test]
fn a_ref_inside_this_project_is_a_reverse_ref_when_an_import_has_the_same_path() {
    let (a, _b) = importer_and_imported("---\nsee: target.md\n---\n");

    assert_eq!(
        reverse_refs_of_target(&a, &[]),
        json!([{
            "path": "notes/pointer.md",
            "namespace": "default",
            "field": "see",
            "written": "target.md"
        }])
    );
}

#[test]
fn a_body_link_into_an_import_is_not_a_reverse_ref_of_the_same_path_here() {
    let (a, _b) = importer_and_imported("---\n---\n[the target](b::notes/target.md)\n");
    assert_pointer_resolves_into_b(&a, "$body");

    assert_eq!(reverse_refs_of_target(&a, &[]), json!([]));
    assert_eq!(reverse_refs_of_target(&a, &["--field", "$body"]), json!([]));
}

#[test]
fn a_body_link_inside_this_project_is_a_reverse_ref_when_an_import_has_the_same_path() {
    let (a, _b) = importer_and_imported("---\n---\n[the target](target.md)\n");

    assert_eq!(
        reverse_refs_of_target(&a, &[]),
        json!([{
            "path": "notes/pointer.md",
            "namespace": "default",
            "field": "$body",
            "written": "target.md",
            "line": 3,
            "col": 1
        }])
    );
}

#[test]
fn a_ref_into_an_import_does_not_satisfy_refby_of_the_same_path_here() {
    let (a, _b) = importer_and_imported("---\nsee: b::notes/target.md\n---\n");
    assert_pointer_resolves_into_b(&a, "see");

    assert_eq!(listed_by(&a, "refby.any(see)"), Vec::<String>::new());
    // Nothing is cited, so `none` holds for both documents of `a`.
    assert_eq!(
        listed_by(&a, "refby.none(see)"),
        vec!["notes/pointer.md", "notes/target.md"]
    );
}

#[test]
fn a_ref_inside_this_project_satisfies_refby_when_an_import_has_the_same_path() {
    let (a, _b) = importer_and_imported("---\nsee: target.md\n---\n");

    assert_eq!(listed_by(&a, "refby.any(see)"), vec!["notes/target.md"]);
    assert_eq!(listed_by(&a, "refby.none(see)"), vec!["notes/pointer.md"]);
}

#[test]
fn a_body_link_into_an_import_does_not_satisfy_refby_body_of_the_same_path_here() {
    let (a, _b) = importer_and_imported("---\n---\n[the target](b::notes/target.md)\n");
    assert_pointer_resolves_into_b(&a, "$body");

    assert_eq!(listed_by(&a, "refby.any($body)"), Vec::<String>::new());
}

#[test]
fn a_body_link_inside_this_project_satisfies_refby_body_when_an_import_has_the_same_path() {
    let (a, _b) = importer_and_imported("---\n---\n[the target](target.md)\n");

    assert_eq!(listed_by(&a, "refby.any($body)"), vec!["notes/target.md"]);
}

#[test]
fn a_condition_after_refby_reads_only_refs_that_stayed_in_this_project() {
    let (a, _b) = importer_and_imported("---\nsee: b::notes/target.md\n---\n");
    assert_pointer_resolves_into_b(&a, "see");

    assert_eq!(
        listed_by(&a, "refby.any(see).path=notes/pointer.md"),
        Vec::<String>::new()
    );

    let (a, _b) = importer_and_imported("---\nsee: target.md\n---\n");

    assert_eq!(
        listed_by(&a, "refby.any(see).path=notes/pointer.md"),
        vec!["notes/target.md"]
    );
}
