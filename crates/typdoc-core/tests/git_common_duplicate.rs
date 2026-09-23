//! The goal's second criterion, the sentence it carves out on purpose: "Two worktrees under the
//! `git-common` lock mode are outside this criterion on purpose: the design says each worktree
//! keeps its own copy of the state file, so two of them can allocate the same number and the
//! duplicate is caught by `validate` after the merge rather than prevented at the write" (the
//! story's own goal), matching `docs/design.md`'s own sentence under the lock table: "two
//! worktrees can allocate the same key, and `validate` catches the duplicate after merge."
//!
//! **What this test builds, and why not more.** `Project::acquire_lock_for`
//! (`crates/typdoc-core/src/project.rs`) refuses `LockMode::GitCommon` outright — "this
//! project's `lock` is `git-common`, which no write command supports yet" — so no write command
//! can exercise the `git-common` lock path today, and this test does not pretend otherwise. The
//! claim it tests is narrower, and is exactly what the sentence above actually asserts: not that
//! the `git-common` lock mechanism itself prevents nothing (there is no such mechanism wired to
//! a command yet to test), but its *consequence* — that two projects with nothing at all
//! coordinating them (which is what "each worktree keeps its own copy of the state file" means,
//! whether or not `git-common` locking is the reason) land on the same number by construction,
//! and `validate` reports the duplicate once both are visible together.
//!
//! Two ordinary temporary directories stand in for the two worktrees, seeded with the same
//! state, and demonstrate the allocation collision directly — no real git repository is needed
//! to show that nothing coordinates two independent `state/default.json` files, which is the
//! whole of what "each worktree keeps its own copy" means here; `project_hash`/
//! `git_common_namespace_lock_path` (`crates/typdoc-core/src/namespace_lock.rs`, decision 14)
//! name where a `git-common` lock *would* live and are irrelevant to this test, since the mode
//! is refused before any lock path is ever built. The "after the merge" half is built as a
//! third, hand-assembled project holding both worktrees' own resulting documents side by side —
//! the one construction that lets two *different*, really coexisting files carry the identical
//! key in this design: two collections of one namespace, each with its own schema file, the two
//! schemas happening to declare the same code (`config.coded-schema-shared` refuses one *schema*
//! shared by two collections, but two separate schemas may declare the same code — reported as
//! `schema.valid`, a finding, not a load error — exactly the shape the pre-existing fixture
//! `fixtures/broken/keys.unique` already uses, reused here rather than invented fresh: one
//! schema, one collection, bound and wildcard-free, is a bijection between key and path, so two
//! coexisting files can never legitimately share a key within *one* schema's own collection).
//! This is not a claim about what a real `git merge` of a same-path add/add conflict resolves to
//! (nothing in the design says), only about `validate`'s own `keys.unique` rule, which groups by
//! namespace and key text and does not care which collection or schema a key came from.

#[allow(dead_code, reason = "each test file uses part of the shared helper")]
mod common;

use std::path::Path;

use typdoc_core::{DocumentArg, NewTarget, Project};
use typdoc_testkit::fake::FixedClock;

use common::{FixedEnv, WF_SCHEMA, write_file};

/// A standalone project — standing in for one worktree — with one coded collection
/// (`tickets/{key}.md`, code `WF`) and its state seeded to `last`, with nothing else in it: two
/// of these, built the same way, share no file and no lock, exactly "each worktree keeps its own
/// copy of the state file."
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

/// **The allocation half.** Two worktrees, seeded identically, nothing coordinating them: each
/// independently allocates the same next number, by construction — not by an unlucky race, but
/// because there is nothing between them that could ever disagree.
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

/// **The "after the merge" half.** A third project holds both worktrees' own resulting
/// documents, side by side, under two collections bound to the same code with different match
/// templates (see this file's own top doc comment for why two *different*, physically
/// coexisting files can legitimately carry one identical key in this design). `validate` reports
/// `keys.unique` for it, naming both.
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
    // Two *separate* schema files, each with its own code binding its own, single collection
    // (`config.coded-schema-shared` refuses one schema shared by two collections), that happen
    // to declare the identical code `WF`: exactly the shape `fixtures/broken/keys.unique` already
    // uses to make two really coexisting documents carry one identical key, reused here rather
    // than invented fresh. Two schemas sharing a code is `schema.valid`, a finding, not a load
    // error, so the project still loads; both keep the same `title` field the worktree-produced
    // text already carries.
    write_file(
        &merged.path().join("wf-a.json"),
        r#"{ "name": "ticket-a", "code": "WF", "fields": { "title": { "type": "string", "required": true } } }"#,
    );
    write_file(
        &merged.path().join("wf-b.json"),
        r#"{ "name": "ticket-b", "code": "WF", "fields": { "title": { "type": "string", "required": true } } }"#,
    );
    // The state each worktree happened to have does not matter to `validate`, which reads
    // documents and their keys, not the state file's own `last`; a plausible merged state is
    // written anyway so this project is not otherwise malformed.
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
