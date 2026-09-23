//! `typdoc mv --renumber`: the argument shape (one positional, a namespace value), the
//! allocation from the destination's state, the same-namespace and cross-project refusals, and
//! the recovery an interruption between the state write and the document's own move leaves.
//!
//! The two-phase prepare-then-rename mechanism and the four within-project refusals are already
//! proved in `mv.rs` and `crates/typdoc-core/tests/mv_seam.rs`; `--renumber` reuses that
//! machinery (`Project::mv_reverse_scan`, `Project::mv_lock`, `Project::mv_rewrite_changes`,
//! `mv::commit`, `Project::mv_result`), so this file covers only what is new: the flag's own
//! argument shape, the allocation, and the refusals decision 11 gives.

#[allow(dead_code, reason = "each test file uses part of the shared helper")]
mod common;

use common::{Ran, Scratch};
use serde_json::json;

/// A project with two namespaces, `story-1` and `story-3`, and one coded collection,
/// `tickets/{key}.md` under each, whose schema's code is `WF`. An entry in `namespaces` without
/// a glob must already exist on disk (design, Namespaces), so both folders are seeded with a
/// file the `tickets` collection does not match, before any test adds a document of its own.
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

/// The same project, whose schema also carries a field with `auto: moves`.
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

// ---------------------------------------------------------------------------------------------
// The allocation, the move, and the ref rewrite; ticket 21's text-mode shape without `--json`.
// ---------------------------------------------------------------------------------------------

/// **Changed (M-10h, ticket 21), deliberate:** without `--json`, `--renumber` no longer prints
/// the new key bare — it prints the same `get`-shaped labeled block every other write command
/// prints, plus `rewritten:`/`unrewritten:`/`findings:`. A caller that only wants the bare key
/// reads it out of `--json`'s `document.key` instead (`renumber_json_reports_the_document_under_its_new_key`,
/// below, already covers that).
#[test]
fn renumber_allocates_the_next_key_from_the_destination_and_prints_the_labeled_block() {
    let project = project();
    project.file("story-1/tickets/WF-5.md", "---\ntitle: One\n---\n");
    // Seeds the destination so the allocated key differs, in text, from the source's: this
    // rules out a fixture that would also pass if `mv_renumber` allocated from the source's own
    // state by mistake.
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

/// Also the hand-written golden for a clean `--renumber`'s `--json` `rewritten` (ticket 21,
/// testing-decisions.md, "Text output": the clean-move case, the third of the three `mv --json`
/// is asked to cover for `rewritten` — the rewrite and unrewritten cases are covered by
/// `renumber_json_carries_the_full_rewritten_list_behind_the_text_count` below).
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

/// The destination's state file is created in canonical form (decision 13), the same shape
/// `new` creates one in, and the source's own state is untouched: `--renumber` writes one state
/// file, not two (decision 13's closing sentence).
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

// ---------------------------------------------------------------------------------------------
// The allocation itself refuses exactly as `new`'s own does (`allocate_key` is shared), and
// decision 15's collision guard still runs even for a number this run just allocated.
// ---------------------------------------------------------------------------------------------

/// `state.malformed`: the destination's `last` is present but not a usable whole number. Refused
/// before anything is written, the same reason `new` refuses it — a guessed number is a key that
/// already belongs to a document.
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

/// `state.missing`: the destination namespace already has a document of this collection and no
/// `last` recorded for it at all. Refused for the same reason `state.malformed` is: a guessed
/// number is a key that already belongs to a document.
#[test]
fn a_missing_state_record_for_a_destination_that_already_has_documents_refuses() {
    let project = project();
    project.file("story-1/tickets/WF-5.md", "---\ntitle: One\n---\n");
    project.file("story-3/tickets/WF-1.md", "---\ntitle: Existing\n---\n");
    // No `.typdoc/state/story-3.json` at all: the collection has documents there already and no
    // record of the highest number ever issued for it.

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

/// Decision 15's own guard still runs for a key this very call just allocated: `Index::build`
/// drops a path more than one collection's `match` reaches out of `entries` (`collections.overlap`),
/// so `highest_existing` never sees the hand-made file sitting at `story-3/tickets/WF-1.md` and
/// allocates `WF-1` anyway — the same "should not be reachable" case `new`'s own
/// `a_name_a_template_produces_that_exists_is_refused_at_exit_7_with_last_unchanged` test
/// constructs, reused here for `--renumber`'s own allocation. Exit 7, the hand-made file and the
/// source document are both untouched, and — the part a mere "the document is unharmed" check
/// would miss — no state file is written at all: the exists check runs before `state::write`.
#[test]
fn a_name_the_allocation_produces_that_already_exists_is_refused_at_exit_7_with_nothing_written() {
    let project = project();
    // Overlaps `story-3/tickets/WF-1.md` exactly, the path this run's own allocation
    // (`max(0, 0) + 1`) is about to compute, so that path is invisible to `highest_existing`
    // even though a file sits there.
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

/// A bare-key frontmatter ref is promoted to the target's sibling-prefixed key form (a bare key
/// always means the holder's own namespace, which this move always leaves); a body link, always
/// a path and never a key, keeps its own sibling-prefixed path form, updated to the new path —
/// two different written-form categories, kept apart, both correctly updated (design.md, `typdoc
/// mv`: "keeping each ref's written form (key, prefixed reference or relative path)").
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

/// `auto: moves` records the previous name with its own namespace's prefix (`story-1:WF-5`),
/// since a bare key alone would not say which namespace it belonged to.
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

// ---------------------------------------------------------------------------------------------
// Done when (a): the same-namespace refusal writes nothing and leaves `last` unchanged.
// ---------------------------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------------------------
// Done when (c): a ref left behind by an interrupted run fails validation rather than silently
// resolving to another document. The state a stop between the state write and the document's
// own move leaves is built by hand, the same technique `mv.rs`'s own
// `the_same_command_run_again_finishes_a_run_a_stop_left_half_done` uses for a plain `mv`; the
// seam-level proof that a real interrupt produces exactly this state is
// `crates/typdoc-core/tests/mv_renumber_seam.rs`.
// ---------------------------------------------------------------------------------------------

/// `story-3`'s state already records `last: 1` (as if `mv --renumber` had written it and then
/// stopped before the document arrived at `WF-1`), a holder already names `story-3:WF-1`, and
/// the source document is still sitting at its old name: the number `WF-1` is permanently
/// skipped (the source's `last` never goes down, so a plain `mv --renumber` run afterwards would
/// allocate `WF-2` there, never `WF-1` again) and the dangling ref is an ordinary, reported
/// `refs.resolve` finding, not something that resolves to `WF-5` or to anything else.
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

// ---------------------------------------------------------------------------------------------
// Done when (d): `--renumber` with no value, and a `project::` prefix on either argument, exit 1.
// ---------------------------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------------------------
// Argument-shape refusals beyond decision 11's own four, needed for the flag to make sense as
// one positional in this mode and two without it.
// ---------------------------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------------------------
// Ticket 21: `--renumber`'s text output without `--json`, and `--json`'s new `rewritten` list —
// the same shape plain `mv` gets, built from the same `typdoc-core` data (`mv.rs`'s own "Ticket
// 21" section covers the plain-`mv` half of this).
// ---------------------------------------------------------------------------------------------

/// A `notes` collection, added to [`project`], with a `see` ref field — for the tests below that
/// need a holder somewhere other than `tickets/` to point back at the renumbered document.
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

/// A move that rewrites at least one ref: the same fixture
/// `renumber_rewrites_refs_held_by_other_documents_in_the_project` already proves the file
/// content for, read here as the hand-written golden for `--renumber`'s text-mode shape
/// (contract, text-output shapes, `mv --renumber`) — `rewritten: 2 refs in 1 document` (the
/// frontmatter `see` and the body link, both held by the one holder).
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

/// The same move's `--json`: `rewritten` carries one entry per ref actually rewritten, matching
/// the new content `renumber_rewrites_refs_held_by_other_documents_in_the_project` already
/// checks by reading the holder's file back.
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

/// A `--renumber` that leaves a ref unrewritten (`body.links` off): the text-mode golden shows
/// `unrewritten:`'s own count and one line naming the holder, `$body`, and the written form left
/// untouched — the same reason plain `mv`'s own version of this test names
/// (testing-decisions.md, "Text output").
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

/// The already-fixed error path (contract decision 4, ticket 21's own item 4): without `--json`,
/// `--renumber`'s own refusal (renumbering into the document's own namespace) prints plain text
/// on stderr, never the `--json` error object — the same case
/// `renumbering_into_the_documents_own_namespace_is_refused_and_leaves_last_unchanged` already
/// proves at exit 1 with `--json`, read here without it.
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
