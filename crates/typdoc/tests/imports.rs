//! Imports through the built binary: `${NAME}` substitution never giving an empty-string
//! path, `imports.absent`'s exit code and its silence when nothing names an absent import,
//! `name::` resolving through a real import (`bad-prefix` for an unknown alias, `import-absent`
//! for one absent on this machine, resolution for one that is there), `project::` arguments,
//! `project` in the name of a document reached through one, `--namespace 'alias::*'`, the
//! machine file `imports.json` actually merged in and read (path *discovery* for its five cases
//! is unit-tested in `typdoc-core`'s own `imports` module; this file is the disk-touching,
//! through-the-binary half, the same split every other index-reading part of this crate already
//! uses), and one level of import only.

#[allow(dead_code, reason = "each test file uses part of the shared helper")]
mod common;

use common::{Ran, Scratch, Spawn, fixture};
use serde_json::{Value, json};

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

/// `fixtures/valid/imports/decoy`: `imports.trap_import` is `${TYPDOC_TEST_UNSET_VAR}notes`. A
/// real project sits at `decoy/notes`, so a wrong implementation that substituted an empty
/// string for the unset variable would turn the path into the plain, existing folder `notes`
/// and the import would wrongly resolve — the fixture would pass under the wrong behaviour if
/// it did not carry this trap (rule: a fixture that would pass under the wrong behaviour proves
/// nothing).
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

/// With the variable set to a real value, the import is no longer absent: the ref now names a
/// real project (`decoy/notes`) that simply has no `x.md` in it, so the outcome is `not-found`,
/// not `import-absent` — proof that these are genuinely different reasons, not one dressed up
/// as the other.
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

    // `validate`'s own exit 2 is a verdict on the project, not a config error: the report is
    // printed on stdout regardless of whether it is favourable (design, JSON output), unlike
    // the generic error object other failures print on stderr.
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
// `project::` arguments: `get`, `toc` and `refs` resolve them; `project` appears in the name of
// a document that belongs to one; a multi-namespace import needs the namespace named, and is
// bad arguments otherwise.
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
    // `LRN-1` itself holds no refs, so this only checks that the command runs inside the
    // import and names the document asked about with `project` set; `document_json`'s own
    // `project` field is exercised more directly by `get`.
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

/// `validate` never reads an import: not for the whole project, not for `--schemas` (both
/// already covered by `namespace_flag_naming_...` above via `list`'s own analogues), and, this
/// case specifically, not for a *named argument* either — `--namespace`/`TYPDOC_NAMESPACE`
/// naming an import must be refused there too, or `validate note.md --namespace 'alias::*'`
/// would silently validate `note.md` under a scope that named something `validate` never reads,
/// giving a clean report that looks like "checked and clean" rather than "the scope you asked
/// for was never read".
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
// `--namespace 'alias::*'` reaching an imported project (`list`), including combined with this
// project's own namespaces, and refusing an unknown or absent alias explicitly named this way.
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
// A `ref.*(f)` query condition follows an arrow across an import: ticket 15's carried-forward
// doubt ("a ref that resolves to a file in no collection is read under an empty schema... no
// fixture reaches that branch") is reached here, through `memory_import::README.md`
// (`fixtures/valid/imports/memory/README.md`, matched by no collection).
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

/// The carried-forward branch: `uncollected.md`'s `see` reaches `memory_import::README.md`, a
/// file in no collection of the imported project. Read under an empty schema, its `path`
/// pseudo-field is still real (the file genuinely exists), so `ref.any(see).path=README.md`
/// matches — the same reasoning `evaluate_reached`'s own doc gives for a same-project target
/// outside every collection, now shown reached across an import too.
#[test]
fn a_ref_condition_reaches_a_document_of_an_import_that_is_in_no_collection() {
    // `named_import` (`fixtures/valid/imports/named`) has one namespace, `only`, not `default`.
    // Reading `named_import::only/uncollected.md` under the *wrong* project (this project,
    // `main`, whose own only namespace is `default`) would report `namespace: default` for it;
    // reading it correctly, under `named_import`'s own namespaces, reports `only`. This is the
    // check `ref.any(see).path=README.md` alone could not make: a wrong implementation that
    // silently read `main`'s own (empty) index still reports the right-looking `path`, since the
    // written path text is carried through either way, so that alone proves nothing about which
    // project's `namespace` was actually consulted.
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
// One level of import only: `memory_import`'s own `.typdoc/config.json` configures an import of
// its own (`loop_back`, pointing at `nowhere-nested`, whose `.typdoc/config.json` is not valid
// JSON). Loading `main` succeeds regardless — proof that `memory_import`'s own import is never
// followed, since a wrong implementation that did follow it would fail to load `main` at all
// (the same "would fail under the wrong behaviour" shape the `${NAME}` trap above uses).
// ---------------------------------------------------------------------------------------------

#[test]
fn an_imports_own_import_is_never_followed() {
    let ran = run(&["get", "note.md", "--json"], &main_project());

    assert_eq!(ran.code, 0, "{}", ran.stderr);
}

// ---------------------------------------------------------------------------------------------
// The machine file `imports.json`: not only found (`typdoc-core`'s own unit tests), but read
// and merged with the project's own `imports`, with the project's own entry winning when both
// name the same alias.
// ---------------------------------------------------------------------------------------------

/// A minimal project of one file, for the machine file to import.
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

// ---------------------------------------------------------------------------------------------
// `config.config-dir`: an invalid `TYPDOC_CONFIG_DIR` is refused loudly, whether or not the
// project's own `imports` is even set. `fixtures/broken/config.config-dir` covers the relative
// case through the generic coverage harness; the two cases below are explicit about which of
// the two conditions ("absolute", "exists") each one fails.
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
    // Unlike `config.parse` or an unknown `version`, an invalid `TYPDOC_CONFIG_DIR` does not
    // stop the rest of `config.json` from being read: `complete` is `true`, the same as any
    // other config error that leaves checking possible (design, Config errors).
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
