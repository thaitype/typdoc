//! Covers SPC-2, SPC-5, SPC-8.
//!
//! `--renumber` shares plain `mv`'s prepare-then-rename commit and its refusals, which `mv.rs` and
//! `crates/typdoc-core/tests/mv_seam.rs` cover; this file covers what is its own.

#[allow(dead_code, reason = "each test file uses part of the shared helper")]
mod common;

use common::{Ran, Scratch};
use serde_json::json;

/// A namespace named without a glob must exist on disk (SPC-7), so each folder is seeded with a
/// file no collection matches.
fn project() -> Scratch {
    let project = Scratch::project(&[]);
    project.file(
        ".typdoc/config.json",
        r#"{ "version": 1, "namespaces": ["story-1", "story-3"] }"#,
    );
    project.file(
        ".typdoc/collections/tickets.json",
        r#"{ "match": "tickets/{key}.md", "schema": "ticket.json" }"#,
    );
    project.file(
        "ticket.json",
        r#"{ "name": "ticket", "code": "WF", "fields": { "title": { "type": "string" } } }"#,
    );
    project.file("story-1/.keep", "");
    project.file("story-3/.keep", "");
    project
}

fn project_with_moves() -> Scratch {
    let project = Scratch::project(&[]);
    project.file(
        ".typdoc/config.json",
        r#"{ "version": 1, "namespaces": ["story-1", "story-3"] }"#,
    );
    project.file(
        ".typdoc/collections/tickets.json",
        r#"{ "match": "tickets/{key}.md", "schema": "ticket.json" }"#,
    );
    project.file(
        "ticket.json",
        r#"{ "name": "ticket", "code": "WF", "fields": {
            "title": { "type": "string" },
            "moved_from": { "type": "list", "auto": "moves" }
        } }"#,
    );
    project.file("story-1/.keep", "");
    project.file("story-3/.keep", "");
    project
}

fn renumber(project: &Scratch, from: &str, namespace: &str) -> Ran {
    common::Spawn::args(["mv", from, "--renumber", namespace, "--json"])
        .cwd(project.path())
        .run()
}

fn renumber_bare(project: &Scratch, from: &str, namespace: &str) -> Ran {
    common::Spawn::args(["mv", from, "--renumber", namespace])
        .cwd(project.path())
        .run()
}

fn read_bytes(project: &Scratch, path: &str) -> Vec<u8> {
    std::fs::read(project.path().join(path)).unwrap_or_else(|e| panic!("{path}: {e}"))
}

// --- Allocation, the move and the ref rewrite ---

#[test]
fn renumber_allocates_the_next_key_from_the_destination_and_prints_the_labeled_block() {
    let project = project();
    project.file("story-1/tickets/WF-5.md", "---\ntitle: One\n---\n");
    // Seeds the destination's `last`, so the key shows the allocation read the destination's
    // state and not the source's.
    project.file("story-3/tickets/WF-1.md", "---\ntitle: Existing\n---\n");
    project.file(
        ".typdoc/state/story-3.json",
        "{\n  \"tickets\": {\n    \"last\": 1\n  }\n}\n",
    );

    let bare = renumber_bare(&project, "WF-5", "story-3");
    assert_eq!(bare.code, 0, "{}", bare.stderr);
    assert_eq!(
        bare.stdout,
        "path: story-3/tickets/WF-2.md\n\
         collection: tickets\n\
         schema: ticket\n\
         namespace: story-3\n\
         key: WF-2\n\
         title: One\n\
         rewritten: 0 refs in 0 documents\n\
         unrewritten: none\n\
         findings: none\n"
    );
    assert!(!project.path().join("story-1/tickets/WF-5.md").exists());
    assert!(project.path().join("story-3/tickets/WF-2.md").exists());
    assert_eq!(
        project.read("story-3/tickets/WF-2.md"),
        "---\ntitle: One\n---\n"
    );
}

#[test]
fn renumber_json_reports_the_document_under_its_new_key() {
    let project = project();
    project.file("story-1/tickets/WF-5.md", "---\ntitle: One\n---\n");

    let ran = renumber(&project, "WF-5", "story-3");

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    let out = ran.stdout_json();
    assert_eq!(out["document"]["path"], json!("story-3/tickets/WF-1.md"));
    assert_eq!(out["document"]["namespace"], json!("story-3"));
    assert_eq!(out["document"]["key"], json!("WF-1"));
    assert_eq!(out["document"]["code"], json!("WF"));
    assert_eq!(out["document"]["collection"], json!("tickets"));
    assert_eq!(out["rewritten"], json!([]));
    assert_eq!(out["unrewritten"], json!([]));
    assert_eq!(out["findings"], json!([]));
}

/// The destination's state file is created in the form `new` creates one in.
#[test]
fn renumber_writes_only_the_destinations_state_file() {
    let project = project();
    project.file("story-1/tickets/WF-5.md", "---\ntitle: One\n---\n");

    let ran = renumber(&project, "WF-5", "story-3");
    assert_eq!(ran.code, 0, "{}", ran.stderr);

    assert_eq!(
        project.read(".typdoc/state/story-3.json"),
        "{\n  \"tickets\": {\n    \"last\": 1\n  }\n}\n"
    );
    assert!(
        !project.path().join(".typdoc/state/story-1.json").exists(),
        "the source's `last` never goes down and this move issues nothing there, so nothing is \
         written for it"
    );
}

// --- Allocation refuses as `new`'s does: `allocate_key` is shared ---

#[test]
fn a_malformed_last_in_the_destinations_state_refuses_and_writes_nothing() {
    let project = project();
    project.file("story-1/tickets/WF-5.md", "---\ntitle: One\n---\n");
    project.file(
        ".typdoc/state/story-3.json",
        "{\n  \"tickets\": {\n    \"last\": \"three\"\n  }\n}\n",
    );
    let before = read_bytes(&project, ".typdoc/state/story-3.json");

    let ran = renumber(&project, "WF-5", "story-3");

    assert_eq!(ran.code, 2, "{}", ran.stderr);
    assert_eq!(
        ran.stderr_json()["details"][0]["rule"],
        json!("state.malformed")
    );
    assert_eq!(
        read_bytes(&project, ".typdoc/state/story-3.json"),
        before,
        "not a single byte changed"
    );
    assert_eq!(
        project.read("story-1/tickets/WF-5.md"),
        "---\ntitle: One\n---\n",
        "the document never moved"
    );
}

#[test]
fn a_missing_state_record_for_a_destination_that_already_has_documents_refuses() {
    let project = project();
    project.file("story-1/tickets/WF-5.md", "---\ntitle: One\n---\n");
    project.file("story-3/tickets/WF-1.md", "---\ntitle: Existing\n---\n");
    // No `.typdoc/state/story-3.json`.

    let ran = renumber(&project, "WF-5", "story-3");

    assert_eq!(ran.code, 2, "{}", ran.stderr);
    assert_eq!(
        ran.stderr_json()["details"][0]["rule"],
        json!("state.missing")
    );
    assert!(!project.path().join(".typdoc/state/story-3.json").exists());
    assert_eq!(
        project.read("story-1/tickets/WF-5.md"),
        "---\ntitle: One\n---\n",
        "the document never moved"
    );
}

/// As in `new.rs`, an overlapping collection hides the file from the index, so allocation hands
/// out the name it sits at. The destination is checked before the state file is written, so no
/// number is spent.
#[test]
fn a_name_the_allocation_produces_that_already_exists_is_refused_at_exit_7_with_nothing_written() {
    let project = project();
    // `story-3/tickets/WF-1.md` is the next number: `max(0, 0) + 1`.
    project.file(
        ".typdoc/collections/collides.json",
        r#"{ "match": "tickets/WF-1.md", "schema": "note.json" }"#,
    );
    project.file("note.json", r#"{ "name": "note", "fields": {} }"#);
    project.file("story-1/tickets/WF-5.md", "---\ntitle: One\n---\n");
    project.file(
        "story-3/tickets/WF-1.md",
        "---\ntitle: Hand-made, in the way\n---\n",
    );
    let before_document = read_bytes(&project, "story-3/tickets/WF-1.md");

    let ran = renumber(&project, "WF-5", "story-3");

    assert_eq!(ran.code, 7, "{}", ran.stderr);
    assert_eq!(ran.stderr_json()["code"], json!(7));
    assert_eq!(
        read_bytes(&project, "story-3/tickets/WF-1.md"),
        before_document,
        "the hand-made file must be byte-identical"
    );
    assert_eq!(
        project.read("story-1/tickets/WF-5.md"),
        "---\ntitle: One\n---\n",
        "the source document never moved"
    );
    assert!(
        !project.path().join(".typdoc/state/story-3.json").exists(),
        "the exists check runs before state::write, so a refused run burns no number"
    );
}

/// A bare key means the holder's own namespace, which the document leaves, so it gains the new
/// namespace's prefix; the body link keeps its prefixed path form.
#[test]
fn renumber_rewrites_refs_held_by_other_documents_in_the_project() {
    let project = project();
    project.file(
        ".typdoc/collections/notes.json",
        r#"{ "match": "notes/*.md", "schema": "note.json" }"#,
    );
    project.file(
        "note.json",
        r#"{ "name": "note", "fields": {
            "title": { "type": "string" },
            "see": { "type": "ref", "target": "*" }
        } }"#,
    );
    project.file("story-1/tickets/WF-5.md", "---\ntitle: One\n---\n");
    project.file(
        "story-1/notes/holder.md",
        "---\ntitle: Holder\nsee: WF-5\n---\n\nSee [it](story-1:tickets/WF-5.md).\n",
    );

    let ran = renumber(&project, "WF-5", "story-3");
    assert_eq!(ran.code, 0, "{}", ran.stderr);
    assert_eq!(
        project.read("story-1/notes/holder.md"),
        "---\ntitle: Holder\nsee: story-3:WF-1\n---\n\nSee [it](story-3:tickets/WF-1.md).\n"
    );
}

/// A bare key would not say which namespace the document was in.
#[test]
fn renumber_records_the_previous_namespace_prefixed_key_in_the_moves_field() {
    let project = project_with_moves();
    project.file("story-1/tickets/WF-5.md", "---\ntitle: One\n---\n");

    let ran = renumber(&project, "WF-5", "story-3");

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    let out = ran.stdout_json();
    assert_eq!(
        out["document"]["fields"]["moved_from"],
        json!(["story-1:WF-5"])
    );
    assert_eq!(
        project.read("story-3/tickets/WF-1.md"),
        "---\ntitle: One\nmoved_from:\n- story-1:WF-5\n---\n"
    );
}

#[test]
fn renumbering_into_the_documents_own_namespace_is_refused_and_leaves_last_unchanged() {
    let project = project();
    project.file("story-1/tickets/WF-5.md", "---\ntitle: One\n---\n");
    project.file(
        ".typdoc/state/story-1.json",
        "{\n  \"tickets\": {\n    \"last\": 5\n  }\n}\n",
    );
    let before = read_bytes(&project, ".typdoc/state/story-1.json");

    let ran = renumber(&project, "WF-5", "story-1");

    assert_eq!(ran.code, 1, "{}", ran.stderr);
    assert_eq!(ran.stderr_json()["code"], json!(1));
    assert_eq!(
        read_bytes(&project, ".typdoc/state/story-1.json"),
        before,
        "not a single byte changed"
    );
    assert_eq!(
        project.read("story-1/tickets/WF-5.md"),
        "---\ntitle: One\n---\n",
        "the document never moved"
    );
    assert!(!project.path().join("story-1/tickets/WF-6.md").exists());
}

// --- A stopped run ---
//
// The state a stop between the state write and the move leaves is built by hand;
// `crates/typdoc-core/tests/mv_renumber_seam.rs` shows that a real stop leaves exactly this state.

/// `story-3`'s `last` is already `1`, as if a run had written it and stopped before the document
/// arrived. `story-3`'s `last` never goes down, so `WF-1` stays skipped and the ref to it stays a
/// `refs.resolve` finding.
#[test]
fn a_ref_left_behind_by_a_stopped_run_fails_validation_instead_of_resolving_elsewhere() {
    let project = project();
    project.file(
        ".typdoc/collections/notes.json",
        r#"{ "match": "notes/*.md", "schema": "note.json" }"#,
    );
    project.file(
        "note.json",
        r#"{ "name": "note", "fields": {
            "title": { "type": "string" },
            "see": { "type": "ref", "target": "*" }
        } }"#,
    );
    project.file("story-1/tickets/WF-5.md", "---\ntitle: One\n---\n");
    project.file(
        "story-1/notes/holder.md",
        "---\ntitle: Holder\nsee: story-3:tickets/WF-1.md\n---\n",
    );
    project.file(
        ".typdoc/state/story-1.json",
        "{\n  \"tickets\": {\n    \"last\": 5\n  }\n}\n",
    );
    project.file(
        ".typdoc/state/story-3.json",
        "{\n  \"tickets\": {\n    \"last\": 1\n  }\n}\n",
    );

    let ran = common::Spawn::args(["validate", "--json"])
        .cwd(project.path())
        .run();

    assert_eq!(ran.code, 2, "{}", ran.stderr);
    let findings = ran.stdout_json()["findings"].clone();
    let findings = findings.as_array().unwrap();
    assert_eq!(findings.len(), 1, "{findings:?}");
    assert_eq!(findings[0]["rule"], json!("refs.resolve"));
    assert_eq!(findings[0]["path"], json!("story-1/notes/holder.md"));

    // The number is never reissued: a fresh `mv --renumber` into `story-3` allocates `WF-2`,
    // not the `WF-1` the interrupted run already spent.
    let ran = renumber(&project, "WF-5", "story-3");
    assert_eq!(ran.code, 0, "{}", ran.stderr);
    assert_eq!(
        ran.stdout_json()["document"]["path"],
        json!("story-3/tickets/WF-2.md")
    );
}

// --- Argument shape ---

#[test]
fn renumber_given_with_no_value_is_exit_1() {
    let project = project();
    project.file("story-1/tickets/WF-5.md", "---\ntitle: One\n---\n");

    let ran = common::Spawn::args(["mv", "WF-5", "--renumber"])
        .cwd(project.path())
        .run();

    assert_eq!(ran.code, 1, "{}", ran.stderr);
    assert!(!project.path().join("story-1/tickets/WF-6.md").exists());
}

#[test]
fn a_project_prefix_on_from_is_exit_1() {
    let project = project();
    project.file("story-1/tickets/WF-5.md", "---\ntitle: One\n---\n");

    let ran = renumber(&project, "other::WF-5", "story-3");

    assert_eq!(ran.code, 1, "{}", ran.stderr);
    assert!(!project.path().join("story-3/tickets/WF-1.md").exists());
}

#[test]
fn a_project_prefix_on_the_namespace_value_is_exit_1() {
    let project = project();
    project.file("story-1/tickets/WF-5.md", "---\ntitle: One\n---\n");

    let ran = renumber(&project, "WF-5", "other::story-3");

    assert_eq!(ran.code, 1, "{}", ran.stderr);
    assert_eq!(
        project.read("story-1/tickets/WF-5.md"),
        "---\ntitle: One\n---\n"
    );
}

#[test]
fn giving_both_a_destination_path_and_renumber_is_exit_1() {
    let project = project();
    project.file("story-1/tickets/WF-5.md", "---\ntitle: One\n---\n");

    let ran = common::Spawn::args([
        "mv",
        "WF-5",
        "story-1/tickets/moved.md",
        "--renumber",
        "story-3",
        "--json",
    ])
    .cwd(project.path())
    .run();

    assert_eq!(ran.code, 1, "{}", ran.stderr);
}

#[test]
fn giving_neither_a_destination_nor_renumber_is_exit_1() {
    let project = project();
    project.file("story-1/tickets/WF-5.md", "---\ntitle: One\n---\n");

    let ran = common::Spawn::args(["mv", "WF-5", "--json"])
        .cwd(project.path())
        .run();

    assert_eq!(ran.code, 1, "{}", ran.stderr);
}

#[test]
fn an_uncoded_document_cannot_be_renumbered() {
    let project = project();
    project.file(
        ".typdoc/collections/notes.json",
        r#"{ "match": "notes/*.md", "schema": "note.json" }"#,
    );
    project.file("note.json", r#"{ "name": "note", "fields": {} }"#);
    project.file("story-1/notes/a.md", "---\ntitle: A\n---\n");

    let ran = renumber(&project, "story-1/notes/a.md", "story-3");

    assert_eq!(ran.code, 1, "{}", ran.stderr);
    assert_eq!(project.read("story-1/notes/a.md"), "---\ntitle: A\n---\n");
}

#[test]
fn an_unknown_destination_namespace_is_exit_1() {
    let project = project();
    project.file("story-1/tickets/WF-5.md", "---\ntitle: One\n---\n");

    let ran = renumber(&project, "WF-5", "nowhere");

    assert_eq!(ran.code, 1, "{}", ran.stderr);
    assert_eq!(
        project.read("story-1/tickets/WF-5.md"),
        "---\ntitle: One\n---\n"
    );
}

// --- Text output, and `rewritten` in `--json` ---

fn project_with_notes() -> Scratch {
    let project = project();
    project.file(
        ".typdoc/collections/notes.json",
        r#"{ "match": "notes/*.md", "schema": "note.json" }"#,
    );
    project.file(
        "note.json",
        r#"{ "name": "note", "fields": {
            "title": { "type": "string", "required": true },
            "see": { "type": "ref", "target": "*" }
        } }"#,
    );
    project
}

/// The frontmatter `see` and the body link are in one holder: `2 refs in 1 document`.
#[test]
fn a_renumber_that_rewrites_refs_prints_the_labeled_block_and_the_rewritten_count() {
    let project = project_with_notes();
    project.file("story-1/tickets/WF-5.md", "---\ntitle: One\n---\n");
    project.file(
        "story-1/notes/holder.md",
        "---\ntitle: Holder\nsee: WF-5\n---\n\nSee [it](story-1:tickets/WF-5.md).\n",
    );

    let ran = renumber_bare(&project, "WF-5", "story-3");

    assert_eq!(ran.code, 0, "stderr: {}", ran.stderr);
    assert_eq!(
        ran.stdout,
        "path: story-3/tickets/WF-1.md\n\
         collection: tickets\n\
         schema: ticket\n\
         namespace: story-3\n\
         key: WF-1\n\
         title: One\n\
         rewritten: 2 refs in 1 document\n\
         unrewritten: none\n\
         findings: none\n"
    );
}

#[test]
fn renumber_json_carries_the_full_rewritten_list_behind_the_text_count() {
    let project = project_with_notes();
    project.file("story-1/tickets/WF-5.md", "---\ntitle: One\n---\n");
    project.file(
        "story-1/notes/holder.md",
        "---\ntitle: Holder\nsee: WF-5\n---\n\nSee [it](story-1:tickets/WF-5.md).\n",
    );

    let ran = renumber(&project, "WF-5", "story-3");

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    let out = ran.stdout_json();
    let rewritten = out["rewritten"].as_array().expect("an array");
    assert_eq!(rewritten.len(), 2, "{rewritten:?}");
    assert!(
        rewritten
            .iter()
            .any(|r| r["document"] == json!("story-1/notes/holder.md")
                && r["field"] == json!("see")
                && r["before"] == json!("WF-5")
                && r["after"] == json!("story-3:WF-1")),
        "{rewritten:?}"
    );
    assert!(
        rewritten
            .iter()
            .any(|r| r["document"] == json!("story-1/notes/holder.md")
                && r["field"] == json!("$body")
                && r["before"] == json!("story-1:tickets/WF-5.md")
                && r["after"] == json!("story-3:tickets/WF-1.md")),
        "{rewritten:?}"
    );
}

#[test]
fn a_renumber_that_leaves_a_ref_unrewritten_prints_its_own_count_and_entry_line() {
    let project = project();
    project.file(
        ".typdoc/collections/notes.json",
        r#"{ "match": "notes/*.md", "schema": "note.json", "validation": { "body.links": { "level": "off" } } }"#,
    );
    project.file(
        "note.json",
        r#"{ "name": "note", "fields": { "title": { "type": "string", "required": true } } }"#,
    );
    project.file("story-1/tickets/WF-5.md", "---\ntitle: One\n---\n");
    project.file(
        "story-1/notes/holder.md",
        "---\ntitle: Holder\n---\n\nSee [it](../tickets/WF-5.md) for details.\n",
    );

    let ran = renumber_bare(&project, "WF-5", "story-3");

    assert_eq!(ran.code, 0, "stderr: {}", ran.stderr);
    assert_eq!(
        ran.stdout,
        "path: story-3/tickets/WF-1.md\n\
         collection: tickets\n\
         schema: ticket\n\
         namespace: story-3\n\
         key: WF-1\n\
         title: One\n\
         rewritten: 0 refs in 0 documents\n\
         unrewritten: 1\n\
         story-1/notes/holder.md  $body  ../tickets/WF-5.md\n\
         findings: none\n"
    );
}

// --- A plain-text mention of the moved key ---

/// Line 5 is the body's first line, after three frontmatter lines and a blank one; column 5
/// follows `See `.
#[test]
fn renumber_lists_a_plain_text_mention_of_the_moved_key_in_unrewritten() {
    let project = project();
    project.file(
        ".typdoc/collections/notes.json",
        r#"{ "match": "notes/*.md", "schema": "note.json" }"#,
    );
    project.file(
        "note.json",
        r#"{ "name": "note", "fields": { "title": { "type": "string" } } }"#,
    );
    project.file("story-1/tickets/WF-5.md", "---\ntitle: One\n---\n");
    project.file(
        "story-1/notes/holder.md",
        "---\ntitle: Holder\n---\n\nSee WF-5 for background, no link here.\n",
    );

    let ran = renumber(&project, "WF-5", "story-3");

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    let out = ran.stdout_json();
    let unrewritten = out["unrewritten"].as_array().expect("array");
    assert_eq!(unrewritten.len(), 1, "{unrewritten:?}");
    assert_eq!(unrewritten[0]["reason"], json!("mention"));
    assert_eq!(unrewritten[0]["path"], json!("story-1/notes/holder.md"));
    assert_eq!(unrewritten[0]["field"], json!("$body"));
    assert_eq!(unrewritten[0]["written"], json!("WF-5"));
    assert_eq!(unrewritten[0]["line"], json!(5));
    assert_eq!(unrewritten[0]["col"], json!(5));
    assert_eq!(
        project.read("story-1/notes/holder.md"),
        "---\ntitle: Holder\n---\n\nSee WF-5 for background, no link here.\n",
        "a mention is never rewritten, only reported"
    );
}

#[test]
fn renumber_prints_the_mention_entry_in_the_text_golden() {
    let project = project();
    project.file(
        ".typdoc/collections/notes.json",
        r#"{ "match": "notes/*.md", "schema": "note.json" }"#,
    );
    project.file(
        "note.json",
        r#"{ "name": "note", "fields": { "title": { "type": "string" } } }"#,
    );
    project.file("story-1/tickets/WF-5.md", "---\ntitle: One\n---\n");
    project.file(
        "story-1/notes/holder.md",
        "---\ntitle: Holder\n---\n\nSee WF-5 for background, no link here.\n",
    );

    let ran = renumber_bare(&project, "WF-5", "story-3");

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    assert_eq!(
        ran.stdout,
        "path: story-3/tickets/WF-1.md\n\
         collection: tickets\n\
         schema: ticket\n\
         namespace: story-3\n\
         key: WF-1\n\
         title: One\n\
         rewritten: 0 refs in 0 documents\n\
         unrewritten: 1\n\
         story-1/notes/holder.md  $body  WF-5\n\
         findings: none\n"
    );
}

#[test]
fn renumber_does_not_report_a_mention_of_an_unrelated_key() {
    let project = project();
    project.file(
        ".typdoc/collections/notes.json",
        r#"{ "match": "notes/*.md", "schema": "note.json" }"#,
    );
    project.file(
        "note.json",
        r#"{ "name": "note", "fields": { "title": { "type": "string" } } }"#,
    );
    project.file("story-1/tickets/WF-5.md", "---\ntitle: One\n---\n");
    project.file("story-1/tickets/WF-9.md", "---\ntitle: Nine\n---\n");
    project.file(
        "story-1/notes/holder.md",
        "---\ntitle: Holder\n---\n\nSee WF-9 for background, unrelated to this move.\n",
    );

    let ran = renumber(&project, "WF-5", "story-3");

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    assert_eq!(ran.stdout_json()["unrewritten"], json!([]));
}

#[test]
fn renumber_still_rewrites_a_formal_ref_while_separately_reporting_a_mention() {
    let project = project();
    project.file(
        ".typdoc/collections/notes.json",
        r#"{ "match": "notes/*.md", "schema": "note.json" }"#,
    );
    project.file(
        "note.json",
        r#"{ "name": "note", "fields": {
            "title": { "type": "string" },
            "see": { "type": "ref", "target": "*" }
        } }"#,
    );
    project.file("story-1/tickets/WF-5.md", "---\ntitle: One\n---\n");
    project.file(
        "story-1/notes/holder.md",
        "---\ntitle: Holder\nsee: WF-5\n---\n\nAlso mentioned in prose as WF-5 here.\n",
    );

    let ran = renumber(&project, "WF-5", "story-3");

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    assert_eq!(
        project.read("story-1/notes/holder.md"),
        "---\ntitle: Holder\nsee: story-3:WF-1\n---\n\nAlso mentioned in prose as WF-5 here.\n",
        "the formal `see` ref is rewritten to the new key; the plain-text mention is left as \
         written"
    );
    let out = ran.stdout_json();
    let unrewritten = out["unrewritten"].as_array().expect("array");
    assert_eq!(unrewritten.len(), 1, "{unrewritten:?}");
    assert_eq!(unrewritten[0]["reason"], json!("mention"));
    assert_eq!(unrewritten[0]["written"], json!("WF-5"));
}

#[test]
fn a_subsequent_validate_run_agrees_with_the_mention_mv_already_reported() {
    let project = project();
    project.file(
        ".typdoc/collections/notes.json",
        r#"{ "match": "notes/*.md", "schema": "note.json", "validation": { "body.mentions": { "level": "warn" } } }"#,
    );
    project.file(
        "note.json",
        r#"{ "name": "note", "fields": { "title": { "type": "string" } } }"#,
    );
    project.file("story-1/tickets/WF-5.md", "---\ntitle: One\n---\n");
    project.file(
        "story-1/notes/holder.md",
        "---\ntitle: Holder\n---\n\nSee WF-5 for background, no link here.\n",
    );

    let ran = renumber(&project, "WF-5", "story-3");
    assert_eq!(ran.code, 0, "{}", ran.stderr);
    let mv_out = ran.stdout_json();
    assert_eq!(mv_out["unrewritten"][0]["reason"], json!("mention"));

    let validated = common::Spawn::args(["validate", "--json"])
        .cwd(project.path())
        .run();
    let findings = validated.stdout_json()["findings"].clone();
    let findings = findings.as_array().expect("array");
    assert!(
        findings.iter().any(|f| f["rule"] == json!("body.mentions")
            && f["path"] == json!("story-1/notes/holder.md")
            && f["message"] == json!("WF-5 not found")),
        "validate should independently agree that the mention `mv` already reported now points \
         nowhere: {findings:?}"
    );
}

#[test]
fn renumber_without_json_prints_a_plain_text_error() {
    let project = project();
    project.file("story-1/tickets/WF-5.md", "---\ntitle: One\n---\n");

    let ran = renumber_bare(&project, "WF-5", "story-1");

    assert_eq!(ran.code, 1, "{}", ran.stderr);
    assert_eq!(ran.stdout, "");
    assert!(
        ran.stderr.starts_with("typdoc: ") && !ran.stderr.starts_with("typdoc: {"),
        "{}",
        ran.stderr
    );
    assert_eq!(
        project.read("story-1/tickets/WF-5.md"),
        "---\ntitle: One\n---\n"
    );
}
