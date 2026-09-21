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

// ---------------------------------------------------------------------------------------------
// `refs.target` and `refs.codedByPath` for a ref that crosses into an import, left unchecked by
// ticket 17 and reachable now that every import is loaded before either runs
// (`Project::schema_info_of`, shared with the schema drift check of `schemas.rs`).
// ---------------------------------------------------------------------------------------------

/// A tiny imported project of two schemas: `learning`, coded `LRN`, with one document to
/// reference; and `other`, uncoded, used by nothing — a real schema of the imported project, so
/// a `target` that names it is not itself a drift fault, and `refs.target`'s own refusal is the
/// only thing a test built on it can be about.
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

/// `target` is `"*"` for `Target::Any` (bare string, the design's own shape for it) and, for
/// anything else, one schema name, wrapped here into the one-element list `Target::Schemas`
/// reads: `["*"]` is a different, stricter value from `"*"` (a list of one schema literally
/// named `*`, which nothing is), so the two are not interchangeable.
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
    // design.md, Target names: "A bare name... means a schema in this project." A bare
    // `learning` must not let the ref through just because the imported project happens to have
    // a schema of that name too.
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
// A gap `registry::KNOWN_GAPS` names: the design says a reverse lookup scans this project's own
// namespaces *and the namespaces of every project it imports*; `refs --reverse` scans this
// project only (`Project::refs`'s own doc comment). Two mutually-importing scratch projects: `a`
// imports `b` and `b` imports `a` back, so `b`'s own document can point into `a` the same way
// any other cross-import ref does, using nothing `refs --reverse` with a `project::` argument
// (refused outright, see `refs_reverse_with_a_project_prefix_is_refused` above) is needed for.
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
    // `a` holds the target document, `x.md`, and imports `b` under the alias `b_import` only so
    // that a fixed implementation would have somewhere to look; `a`'s own schema needs no ref
    // field, since nothing in `a` points anywhere.
    write(a.path(), "b_import", b.path(), false);
    std::fs::write(a.path().join("x.md"), "").unwrap();
    // `b` imports `a` back under `a_import`, and its one document, `y.md`, points at `a`'s
    // `x.md` through that alias — an ordinary cross-import ref, read exactly as
    // `an_alias_configured_and_present_resolves_and_carries_project_in_its_name` above reads
    // one, just from the other side of the pair.
    write(b.path(), "a_import", a.path(), true);
    std::fs::write(b.path().join("y.md"), "---\nsee: a_import::x.md\n---\n").unwrap();

    // The premise: `b`'s own ref genuinely resolves. If it did not, a reverse scan omitting it
    // would prove nothing (a fixture that would pass under the wrong behaviour proves nothing).
    let from_b = run(&["refs", "y.md", "--json"], b.path());
    assert_eq!(from_b.code, 0, "{}", from_b.stderr);
    let out = from_b.stdout_json();
    assert_eq!(out["refs"][0]["path"], json!("x.md"));
    assert_eq!(out["refs"][0]["project"], json!("a_import"));

    // The gap: run from `a`, a reverse lookup on its own `x.md` does not see that ref, though
    // the design's reverse lookup is meant to scan `a`'s imports too.
    let from_a = run(&["refs", "x.md", "--reverse", "--json"], a.path());
    assert_eq!(from_a.code, 0, "{}", from_a.stderr);
    assert_eq!(from_a.stdout_json()["refs"], json!([]));
}

// ---------------------------------------------------------------------------------------------
// A third gap `registry::KNOWN_GAPS` names, alongside the two above: a body link that crosses
// an import has its target's existence checked (`body.links`) but not the `#anchor` after it
// (`body.anchors`) — `Project::check_body_destination`'s own comment on the `Import` branch
// says `resolved_path` stays `None` there, "so the anchor check below never runs for this
// destination". `target.md` is given exactly one real heading, so a link to a different one
// would be caught inside one project; across this import, it is not.
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

/// The pair of projects: `a` (returned first, with `pointer_text` as `notes/pointer.md`) imports
/// `b` under the alias `b`. Both hold `notes/target.md`, and both have the same collection and
/// the same schema, with a ref field `see` that may point anywhere.
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

/// What `refs notes/target.md --reverse --json` reports as `refs`, run in `a`.
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

/// The paths `list --where <condition> --ids` prints, run in `a`.
fn listed_by(a: &Scratch, condition: &str) -> Vec<String> {
    let ran = run(&["list", "--where", condition, "--ids"], a.path());
    assert_eq!(ran.code, 0, "{}", ran.stderr);
    ran.stdout.lines().map(str::to_owned).collect()
}

/// The premise of the fault shape: the ref of `pointer.md` resolves, into `b`.
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
