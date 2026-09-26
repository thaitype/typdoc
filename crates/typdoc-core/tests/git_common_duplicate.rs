//! Two worktrees under the `git-common` lock mode each keep their own copy of the state file,
//! so both can issue the same number, and `validate` reports the duplicate after the merge.
//!
//! No write command accepts `git-common` (`Project::acquire_lock_for` refuses it), so this tests
//! the consequence only: two projects with nothing coordinating them issue the same key, and
//! `validate` reports `keys.unique` once both documents are in one project. Two temporary
//! directories stand in for the worktrees. Nothing here says how `git merge` would resolve an
//! add/add conflict at one path.

#[allow(dead_code, reason = "each test file uses part of the shared helper")]
mod common;

use std::path::Path;

use typdoc_core::{DocumentArg, NewTarget, Project};
use typdoc_testkit::fake::FixedClock;

use common::{FixedEnv, WF_SCHEMA, write_file};

/// One worktree: a project of its own, with its state seeded to `last`.
fn worktree(last: u64) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("a scratch folder");
    write_file(
        &dir.path().join(".typdoc/config.json"),
        r#"{ "version": 1 }"#,
    );
    write_file(
        &dir.path().join(".typdoc/collections/tickets.json"),
        r#"{ "match": "tickets/{key}.md", "schema": "wf.json" }"#,
    );
    write_file(&dir.path().join("wf.json"), WF_SCHEMA);
    write_file(
        &dir.path().join(".typdoc/state/default.json"),
        &format!("{{ \"tickets\": {{ \"last\": {last} }} }}"),
    );
    dir
}

fn allocate(root: &Path, title: &str) -> String {
    let env = FixedEnv {
        cwd: root.to_path_buf(),
    };
    let clock = FixedClock::new();
    let fs = typdoc_fs::SystemFs;
    let deps = typdoc_core::Deps {
        env: &env,
        fs: &fs,
        clock: &clock,
    };
    let project = Project::load(root, &env).expect("the worktree project loads");
    let target = NewTarget::Coded {
        code: "WF".to_owned(),
        title: title.to_owned(),
    };
    let document = project
        .new_document(&target, &deps, &[], std::time::Duration::from_secs(5), None)
        .unwrap_or_else(|e| panic!("allocating in {root:?} failed: {e}"));
    document
        .key
        .expect("a coded allocation always returns a key")
}

#[test]
fn two_worktrees_with_nothing_coordinating_them_allocate_the_same_key() {
    let worktree_a = worktree(5);
    let worktree_b = worktree(5);

    let key_a = allocate(worktree_a.path(), "From worktree A");
    let key_b = allocate(worktree_b.path(), "From worktree B");

    assert_eq!(key_a, "WF-6");
    assert_eq!(key_b, "WF-6");
    assert_eq!(
        key_a, key_b,
        "two worktrees with nothing coordinating them land on the same key by construction"
    );
}

#[test]
fn validate_reports_the_duplicate_once_both_worktrees_own_documents_are_visible_together() {
    let worktree_a = worktree(5);
    let worktree_b = worktree(5);
    let key_a = allocate(worktree_a.path(), "From worktree A");
    let key_b = allocate(worktree_b.path(), "From worktree B");
    assert_eq!((key_a.as_str(), key_b.as_str()), ("WF-6", "WF-6"));
    let text_a = std::fs::read_to_string(worktree_a.path().join("tickets/WF-6.md"))
        .expect("worktree A's own document");
    let text_b = std::fs::read_to_string(worktree_b.path().join("tickets/WF-6.md"))
        .expect("worktree B's own document");

    let merged = tempfile::tempdir().expect("a scratch folder");
    write_file(
        &merged.path().join(".typdoc/config.json"),
        r#"{ "version": 1 }"#,
    );
    write_file(
        &merged.path().join(".typdoc/collections/tickets.json"),
        r#"{ "match": "tickets/{key}.md", "schema": "wf-a.json" }"#,
    );
    write_file(
        &merged
            .path()
            .join(".typdoc/collections/from-worktree-b.json"),
        r#"{ "match": "from-worktree-b/{key}.md", "schema": "wf-b.json" }"#,
    );
    // Two collections, since within one a key names one path. Two schemas, since one schema
    // shared by two collections is `config.coded-schema-shared`, while two schemas with one code
    // are only a `schema.valid` finding and the project still loads.
    write_file(
        &merged.path().join("wf-a.json"),
        r#"{ "name": "ticket-a", "code": "WF", "fields": { "title": { "type": "string", "required": true } } }"#,
    );
    write_file(
        &merged.path().join("wf-b.json"),
        r#"{ "name": "ticket-b", "code": "WF", "fields": { "title": { "type": "string", "required": true } } }"#,
    );
    // `validate` does not read `last`; the state file only keeps the project well formed.
    write_file(
        &merged.path().join(".typdoc/state/default.json"),
        r#"{ "tickets": { "last": 6 } }"#,
    );
    write_file(&merged.path().join("tickets/WF-6.md"), &text_a);
    write_file(&merged.path().join("from-worktree-b/WF-6.md"), &text_b);

    let env = FixedEnv {
        cwd: merged.path().to_path_buf(),
    };
    let project = Project::load(merged.path(), &env).expect("the merged project loads");
    let args: &[DocumentArg] = &[];
    let report = project
        .validate(args, false, false, false, None, &env)
        .expect("validate runs");

    let duplicates: Vec<_> = report
        .findings
        .iter()
        .filter(|finding| finding.rule == "keys.unique")
        .collect();
    assert_eq!(
        duplicates.len(),
        2,
        "both documents carrying the duplicate key are reported, findings: {:#?}",
        report.findings
    );
    let paths: std::collections::BTreeSet<&str> =
        duplicates.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(
        paths,
        std::collections::BTreeSet::from(["tickets/WF-6.md", "from-worktree-b/WF-6.md"]),
        "the duplicate is reported against both of the worktrees' own documents"
    );
}
