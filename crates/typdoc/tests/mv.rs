//! Covers SPC-2, SPC-5, SPC-10, SPC-12.

#[allow(dead_code, reason = "each test file uses part of the shared helper")]
mod common;

use common::{NOTES, Ran, Scratch};
use serde_json::json;

const REF_NOTES: [(&str, &str); 2] = [
    (
        ".typdoc/collections/notes.json",
        r#"{ "match": "*.md", "schema": "note.json" }"#,
    ),
    (
        "note.json",
        r#"{ "name": "note", "fields": { "title": { "type": "string", "required": true }, "see": { "type": "ref", "target": "*" } } }"#,
    ),
];

fn mv(project: &Scratch, from: &str, to: &str) -> Ran {
    common::Spawn::args(["mv", from, to, "--json"])
        .cwd(project.path())
        .run()
}

fn mv_text(project: &Scratch, from: &str, to: &str) -> Ran {
    common::Spawn::args(["mv", from, to])
        .cwd(project.path())
        .run()
}

// --- A stopped run is its own way back ---

/// The stop itself is staged at the seam, in `crates/typdoc-core/tests/mv_seam.rs`, where a fake
/// file system stops after a chosen operation. Here the state such a stop leaves is built by
/// hand: a holder already naming the new path, and the document still at the old one.
#[test]
fn the_same_command_run_again_finishes_a_run_a_stop_left_half_done() {
    let project = Scratch::project(&REF_NOTES);
    project.file("old.md", "---\ntitle: A\n---\n");
    // As if an earlier run of `mv old.md new.md` had already rewritten this ref and then
    // stopped before moving the document itself.
    project.file("holder.md", "---\ntitle: B\nsee: new.md\n---\n");

    let before = common::Spawn::args(["validate", "--json"])
        .cwd(project.path())
        .run();
    assert_eq!(
        before.code, 2,
        "an error-level finding exits 2: {}",
        before.stderr
    );
    let before_json = before.stdout_json();
    assert_eq!(before_json["summary"]["findings"]["error"], json!(1));
    assert_eq!(before_json["findings"][0]["rule"], json!("refs.resolve"));
    assert_eq!(before_json["findings"][0]["path"], json!("holder.md"));

    let ran = mv(&project, "old.md", "new.md");
    assert_eq!(ran.code, 0, "{}", ran.stderr);
    let out = ran.stdout_json();
    assert_eq!(out["document"]["path"], json!("new.md"));
    assert_eq!(out["unrewritten"], json!([]));
    assert_eq!(out["findings"], json!([]));
    assert_eq!(
        project.read("holder.md"),
        "---\ntitle: B\nsee: new.md\n---\n",
        "already correct, so untouched"
    );
    assert!(!project.path().join("old.md").exists());
    assert!(project.path().join("new.md").exists());

    let after = common::Spawn::args(["validate", "--json"])
        .cwd(project.path())
        .run();
    assert_eq!(after.code, 0, "{}", after.stderr);
    assert_eq!(
        after.stdout_json()["summary"]["findings"]["error"],
        json!(0),
        "no finding is left"
    );
}

// --- Refusals leave every file as it was ---

#[test]
fn a_destination_that_already_exists_is_refused_at_exit_7_and_nothing_changes() {
    let project = Scratch::project(&NOTES);
    project.file("a.md", "---\ntitle: A\n---\n");
    project.file("b.md", "---\ntitle: B\n---\n");

    let ran = mv(&project, "a.md", "b.md");

    assert_eq!(ran.code, 7, "{}", ran.stderr);
    assert_eq!(ran.stderr_json()["code"], json!(7));
    assert_eq!(project.read("a.md"), "---\ntitle: A\n---\n");
    assert_eq!(project.read("b.md"), "---\ntitle: B\n---\n");
}

/// File identity says the two paths are one file here and in a case-only rename on a
/// case-insensitive file system; only the case-only rename has a case difference to explain, so
/// this case gets a message of its own.
#[test]
fn the_same_path_given_twice_is_refused_at_exit_7_with_its_own_message() {
    let project = Scratch::project(&NOTES);
    project.file("a.md", "---\ntitle: A\n---\n");

    let ran = mv(&project, "a.md", "a.md");

    assert_eq!(ran.code, 7, "{}", ran.stderr);
    assert_eq!(project.read("a.md"), "---\ntitle: A\n---\n");
    assert!(
        ran.stderr_json()["error"]
            .as_str()
            .unwrap()
            .contains("already names this document"),
        "the identical-path case gets its own wording, not the case-only-rename message: {}",
        ran.stderr
    );
}

#[test]
fn a_coded_document_cannot_move_and_nothing_changes() {
    let project = Scratch::project(&[
        (
            ".typdoc/collections/tickets.json",
            r#"{ "match": "tickets/{key}.md", "schema": "ticket.json" }"#,
        ),
        (
            "ticket.json",
            r#"{ "name": "ticket", "code": "WF", "fields": { "title": { "type": "string" } } }"#,
        ),
    ]);
    project.file("tickets/WF-1.md", "---\ntitle: One\n---\n");

    let ran = mv(&project, "WF-1", "tickets/moved.md");

    assert_eq!(ran.code, 1, "{}", ran.stderr);
    assert!(
        ran.stderr_json()["error"]
            .as_str()
            .unwrap()
            .contains("fixed by its key"),
        "same namespace, so the message is about the key rather than about --renumber \
         (which cannot help here either): {}",
        ran.stderr
    );
    assert_eq!(project.read("tickets/WF-1.md"), "---\ntitle: One\n---\n");
    assert!(!project.path().join("tickets/moved.md").exists());
}

#[test]
fn an_uncoded_document_cannot_move_into_a_coded_collection() {
    let project = Scratch::project(&[
        (
            ".typdoc/collections/notes.json",
            r#"{ "match": "*.md", "schema": "note.json" }"#,
        ),
        (
            ".typdoc/collections/tickets.json",
            r#"{ "match": "tickets/{key}.md", "schema": "ticket.json" }"#,
        ),
        ("note.json", r#"{ "name": "note", "fields": {} }"#),
        (
            "ticket.json",
            r#"{ "name": "ticket", "code": "WF", "fields": {} }"#,
        ),
    ]);
    project.file("a.md", "---\ntitle: A\n---\n");

    let ran = mv(&project, "a.md", "tickets/WF-9.md");

    assert_eq!(ran.code, 1, "{}", ran.stderr);
    assert_eq!(project.read("a.md"), "---\ntitle: A\n---\n");
    assert!(!project.path().join("tickets/WF-9.md").exists());
}

// --- The collection a document lands in ---

#[test]
fn a_move_onto_a_schema_the_document_fails_is_carried_out_and_reported_not_refused() {
    let project = Scratch::project(&[
        (
            ".typdoc/collections/notes.json",
            r#"{ "match": "*.md", "schema": "note.json" }"#,
        ),
        (
            ".typdoc/collections/tasks.json",
            r#"{ "match": "tasks/*.md", "schema": "task.json" }"#,
        ),
        (
            "note.json",
            r#"{ "name": "note", "fields": { "title": { "type": "string", "required": true } } }"#,
        ),
        (
            "task.json",
            r#"{ "name": "task", "fields": { "title": { "type": "string", "required": true }, "status": { "type": "enum", "values": ["open", "done"], "required": true } } }"#,
        ),
    ]);
    project.file("plain.md", "---\ntitle: Plain\n---\n");

    let ran = mv(&project, "plain.md", "tasks/plain.md");

    assert_eq!(
        ran.code, 0,
        "the move happened; validation is a separate concern: {}",
        ran.stderr
    );
    let out = ran.stdout_json();
    assert_eq!(out["document"]["path"], json!("tasks/plain.md"));
    assert_eq!(out["findings"][0]["field"], json!("status"));
    assert_eq!(out["findings"][0]["rule"], json!("frontmatter.types"));
    assert!(project.path().join("tasks/plain.md").exists());
    assert!(!project.path().join("plain.md").exists());
}

#[test]
fn moving_out_of_every_collection_is_allowed_and_the_document_carries_no_collection() {
    let project = Scratch::project(&NOTES);
    project.file("a.md", "---\ntitle: A\n---\n");

    // `*.md` matches only a top-level file, so a file below `elsewhere/` fits no collection.
    let ran = mv(&project, "a.md", "elsewhere/a.md");

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    let out = ran.stdout_json();
    assert_eq!(out["document"]["path"], json!("elsewhere/a.md"));
    assert_eq!(
        out["document"].get("collection"),
        None,
        "no collection to report, so the field is left out entirely rather than printed empty"
    );
    assert_eq!(out["document"].get("schema"), None);
    assert_eq!(out["document"].get("code"), None);
    assert_eq!(out["document"]["fields"]["title"], json!("A"));
}

// --- Refs `mv` leaves unrewritten ---
//
// The `imported-project` reason is not reached here: it needs the reverse scan into imports that
// `[reverse-scope]` in `registry::KNOWN_GAPS` records as missing.

#[test]
fn a_body_link_in_a_document_whose_body_links_rule_is_off_is_reported_unrewritten_not_rewritten() {
    let project = Scratch::project(&[
        (
            ".typdoc/collections/notes.json",
            r#"{ "match": "*.md", "schema": "note.json", "validation": { "body.links": { "level": "off" } } }"#,
        ),
        ("note.json", r#"{ "name": "note", "fields": {} }"#),
    ]);
    project.file("old.md", "---\ntitle: A\n---\n");
    project.file(
        "holder.md",
        "---\ntitle: B\n---\n\nSee [a](old.md) for details.\n",
    );

    let ran = mv(&project, "old.md", "new.md");

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    let out = ran.stdout_json();
    assert_eq!(out["unrewritten"][0]["reason"], json!("links-rule-off"));
    assert_eq!(out["unrewritten"][0]["path"], json!("holder.md"));
    assert_eq!(out["unrewritten"][0]["written"], json!("old.md"));
    assert_eq!(
        project.read("holder.md"),
        "---\ntitle: B\n---\n\nSee [a](old.md) for details.\n",
        "left exactly as written, per the rule being off"
    );
}

/// A mention is always a key (`links::mentions`), and a plain `mv` never moves a coded document,
/// so it has no mention to report.
#[test]
fn a_plain_mv_never_reports_a_mention_since_the_moved_document_has_no_key() {
    let project = Scratch::project(&[
        (
            ".typdoc/collections/notes.json",
            r#"{ "match": "*.md", "schema": "note.json" }"#,
        ),
        (
            ".typdoc/collections/tickets.json",
            r#"{ "match": "tickets/{key}.md", "schema": "ticket.json" }"#,
        ),
        ("note.json", r#"{ "name": "note", "fields": {} }"#),
        (
            "ticket.json",
            r#"{ "name": "ticket", "code": "WF", "fields": {} }"#,
        ),
    ]);
    project.file("tickets/WF-1.md", "---\ntitle: One\n---\n");
    project.file("old.md", "---\ntitle: A\n---\n");
    project.file(
        "holder.md",
        "---\ntitle: B\n---\n\nSee WF-1 in passing, unrelated to this move.\n",
    );

    let ran = mv(&project, "old.md", "new.md");

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    assert_eq!(
        ran.stdout_json()["unrewritten"],
        json!([]),
        "old.md has no key at all, so no mention of any key could ever be about it"
    );
    assert_eq!(
        project.read("holder.md"),
        "---\ntitle: B\n---\n\nSee WF-1 in passing, unrelated to this move.\n"
    );
}

// --- Rewriting refs, each in its own written form ---

#[test]
fn frontmatter_and_body_refs_are_both_rewritten_and_an_unrelated_link_is_left_alone() {
    let project = Scratch::project(&REF_NOTES);
    project.file("old.md", "---\ntitle: A\n---\n");
    project.file("other.md", "---\ntitle: Other\n---\n");
    project.file(
        "holder.md",
        "---\ntitle: B\nsee: old.md\n---\n\nSee [a](old.md) and [another](other.md).\n",
    );

    let ran = mv(&project, "old.md", "renamed.md");

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    assert_eq!(
        project.read("holder.md"),
        "---\ntitle: B\nsee: renamed.md\n---\n\nSee [a](renamed.md) and [another](other.md).\n"
    );
}

#[test]
fn only_the_matching_item_of_a_ref_list_field_is_rewritten_the_rest_keep_their_position() {
    let project = Scratch::project(&[
        (
            ".typdoc/collections/notes.json",
            r#"{ "match": "*.md", "schema": "note.json" }"#,
        ),
        (
            "note.json",
            r#"{ "name": "note", "fields": { "title": { "type": "string" }, "see": { "type": "ref[]", "target": "*" } } }"#,
        ),
    ]);
    project.file("old.md", "---\ntitle: A\n---\n");
    project.file("other.md", "---\ntitle: Other\n---\n");
    project.file(
        "holder.md",
        "---\ntitle: B\nsee:\n  - other.md\n  - old.md\n---\n",
    );

    let ran = mv(&project, "old.md", "renamed.md");

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    assert_eq!(
        project.read("holder.md"),
        "---\ntitle: B\nsee:\n- other.md\n- renamed.md\n---\n",
        "the first item, which never named old.md, keeps its own position"
    );
}

#[test]
fn a_body_link_written_with_percent_encoding_keeps_that_convention() {
    let project = Scratch::project(&NOTES);
    project.file("old file.md", "---\ntitle: A\n---\n");
    project.file(
        "holder.md",
        "---\ntitle: B\n---\n\nSee [a](old%20file.md).\n",
    );

    let ran = mv(&project, "old file.md", "new file.md");

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    assert_eq!(
        project.read("holder.md"),
        "---\ntitle: B\n---\n\nSee [a](new%20file.md).\n"
    );
}

// --- `auto: moves` and the file mode ---

#[test]
fn auto_moves_appends_the_previous_path_and_a_second_run_with_the_same_state_does_not_double_it() {
    let project = Scratch::project(&[
        (
            ".typdoc/collections/notes.json",
            r#"{ "match": "*.md", "schema": "note.json" }"#,
        ),
        (
            "note.json",
            r#"{ "name": "note", "fields": { "title": { "type": "string" }, "moved_from": { "type": "list", "auto": "moves" } } }"#,
        ),
    ]);
    project.file("old.md", "---\ntitle: A\n---\n");

    let ran = mv(&project, "old.md", "new.md");

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    let out = ran.stdout_json();
    assert_eq!(out["document"]["fields"]["moved_from"], json!(["old.md"]));
    assert_eq!(
        project.read("new.md"),
        "---\ntitle: A\nmoved_from:\n- old.md\n---\n"
    );
}

#[cfg(unix)]
#[test]
fn the_mode_of_an_existing_file_is_carried_to_its_new_name() {
    use std::os::unix::fs::PermissionsExt;

    let project = Scratch::project(&NOTES);
    project.file("a.md", "---\ntitle: A\n---\n");
    std::fs::set_permissions(
        project.path().join("a.md"),
        std::fs::Permissions::from_mode(0o600),
    )
    .unwrap();

    let ran = mv(&project, "a.md", "b.md");

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    let mode = std::fs::metadata(project.path().join("b.md"))
        .unwrap()
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(mode, 0o600);
}

/// The moved document keeps its mode through a plain rename. A rewritten holder is replaced by a
/// temp file (`prepare_replacement`), which has the holder's mode only because the write carries
/// it across.
#[cfg(unix)]
#[test]
fn the_mode_of_a_rewritten_holder_is_carried_across_its_own_content_replacement() {
    use std::os::unix::fs::PermissionsExt;

    let project = Scratch::project(&REF_NOTES);
    project.file("old.md", "---\ntitle: A\n---\n");
    project.file("holder.md", "---\ntitle: B\nsee: old.md\n---\n");
    std::fs::set_permissions(
        project.path().join("holder.md"),
        std::fs::Permissions::from_mode(0o640),
    )
    .unwrap();

    let ran = mv(&project, "old.md", "new.md");

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    let mode = std::fs::metadata(project.path().join("holder.md"))
        .unwrap()
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(mode, 0o640);
}

// --- Text output, and `rewritten` in `--json` ---

/// One ref in `see` and one body link, both in the same holder: `2 refs in 1 document`.
#[test]
fn a_move_that_rewrites_refs_prints_the_labeled_block_and_the_rewritten_count() {
    let project = Scratch::project(&REF_NOTES);
    project.file("old.md", "---\ntitle: A\n---\n");
    project.file(
        "holder.md",
        "---\ntitle: B\nsee: old.md\n---\n\nSee [a](old.md).\n",
    );

    let ran = mv_text(&project, "old.md", "renamed.md");

    assert_eq!(ran.code, 0, "stderr: {}", ran.stderr);
    assert_eq!(ran.stderr, "");
    assert_eq!(
        ran.stdout,
        "path: renamed.md\n\
         collection: notes\n\
         schema: note\n\
         namespace: default\n\
         title: A\n\
         rewritten: 2 refs in 1 document\n\
         unrewritten: none\n\
         findings: none\n"
    );
}

#[test]
fn mv_json_carries_the_full_rewritten_list_behind_the_text_count() {
    let project = Scratch::project(&REF_NOTES);
    project.file("old.md", "---\ntitle: A\n---\n");
    project.file(
        "holder.md",
        "---\ntitle: B\nsee: old.md\n---\n\nSee [a](old.md).\n",
    );

    let ran = mv(&project, "old.md", "renamed.md");

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    let out = ran.stdout_json();
    let rewritten = out["rewritten"].as_array().expect("an array");
    assert_eq!(rewritten.len(), 2, "{rewritten:?}");
    assert!(
        rewritten.iter().any(|r| r["document"] == json!("holder.md")
            && r["field"] == json!("see")
            && r["before"] == json!("old.md")
            && r["after"] == json!("renamed.md")),
        "{rewritten:?}"
    );
    assert!(
        rewritten.iter().any(|r| r["document"] == json!("holder.md")
            && r["field"] == json!("$body")
            && r["before"] == json!("old.md")
            && r["after"] == json!("renamed.md")),
        "{rewritten:?}"
    );
    assert_eq!(out["document"]["path"], json!("renamed.md"));
    assert_eq!(out["unrewritten"], json!([]));
    assert_eq!(out["findings"], json!([]));
}

#[test]
fn a_move_that_rewrites_exactly_one_ref_in_one_document_uses_the_singular_form() {
    let project = Scratch::project(&REF_NOTES);
    project.file("old.md", "---\ntitle: A\n---\n");
    project.file("holder.md", "---\ntitle: B\nsee: old.md\n---\n");

    let ran = mv_text(&project, "old.md", "renamed.md");

    assert_eq!(ran.code, 0, "stderr: {}", ran.stderr);
    assert_eq!(
        ran.stdout,
        "path: renamed.md\n\
         collection: notes\n\
         schema: note\n\
         namespace: default\n\
         title: A\n\
         rewritten: 1 ref in 1 document\n\
         unrewritten: none\n\
         findings: none\n"
    );
}

#[test]
fn a_move_that_leaves_a_ref_unrewritten_prints_its_own_count_and_entry_line() {
    // `title` is declared so that `findings` stays empty: under `common::NOTES`, which declares no
    // field, it would report `frontmatter.unknown`.
    let project = Scratch::project(&[
        (
            ".typdoc/collections/notes.json",
            r#"{ "match": "*.md", "schema": "note.json", "validation": { "body.links": { "level": "off" } } }"#,
        ),
        (
            "note.json",
            r#"{ "name": "note", "fields": { "title": { "type": "string", "required": true } } }"#,
        ),
    ]);
    project.file("old.md", "---\ntitle: A\n---\n");
    project.file(
        "holder.md",
        "---\ntitle: B\n---\n\nSee [a](old.md) for details.\n",
    );

    let ran = mv_text(&project, "old.md", "new.md");

    assert_eq!(ran.code, 0, "stderr: {}", ran.stderr);
    assert_eq!(
        ran.stdout,
        "path: new.md\n\
         collection: notes\n\
         schema: note\n\
         namespace: default\n\
         title: A\n\
         rewritten: 0 refs in 0 documents\n\
         unrewritten: 1\n\
         holder.md  $body  old.md\n\
         findings: none\n"
    );
}

/// A holder is named as `refs` names a document (`ref_name_text`): a key is qualified with its
/// namespace only when the project has several.
#[test]
fn unrewritten_names_a_coded_holder_by_its_bare_key_in_a_single_namespace_project() {
    let project = Scratch::project(&[
        (
            ".typdoc/collections/notes.json",
            r#"{ "match": "*.md", "schema": "note.json" }"#,
        ),
        (
            ".typdoc/collections/tickets.json",
            r#"{ "match": "tickets/{key}.md", "schema": "ticket.json", "validation": { "body.links": { "level": "off" } } }"#,
        ),
        (
            "note.json",
            r#"{ "name": "note", "fields": { "title": { "type": "string", "required": true } } }"#,
        ),
        (
            "ticket.json",
            r#"{ "name": "ticket", "code": "WF", "fields": { "title": { "type": "string" } } }"#,
        ),
    ]);
    project.file("old.md", "---\ntitle: A\n---\n");
    project.file(
        "tickets/WF-1.md",
        "---\ntitle: Holder\n---\n\nSee [it](../old.md) for details.\n",
    );

    let ran = mv_text(&project, "old.md", "new.md");

    assert_eq!(ran.code, 0, "stderr: {}", ran.stderr);
    assert_eq!(
        ran.stdout,
        "path: new.md\n\
         collection: notes\n\
         schema: note\n\
         namespace: default\n\
         title: A\n\
         rewritten: 0 refs in 0 documents\n\
         unrewritten: 1\n\
         WF-1  $body  ../old.md\n\
         findings: none\n"
    );
}

/// `title` is declared for the same reason as in the test above.
#[test]
fn a_clean_move_prints_zero_rewritten_and_none_unrewritten() {
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
    project.file("a.md", "---\ntitle: A\n---\n");

    let ran = mv_text(&project, "a.md", "b.md");

    assert_eq!(ran.code, 0, "stderr: {}", ran.stderr);
    assert_eq!(
        ran.stdout,
        "path: b.md\n\
         collection: notes\n\
         schema: note\n\
         namespace: default\n\
         title: A\n\
         rewritten: 0 refs in 0 documents\n\
         unrewritten: none\n\
         findings: none\n"
    );
}

#[test]
fn mv_json_reports_an_empty_rewritten_list_for_a_clean_move() {
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
    project.file("a.md", "---\ntitle: A\n---\n");

    let ran = mv(&project, "a.md", "b.md");

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    let out = ran.stdout_json();
    assert_eq!(out["rewritten"], json!([]));
    assert_eq!(out["unrewritten"], json!([]));
    assert_eq!(out["findings"], json!([]));
}

#[test]
fn a_move_that_fails_the_destination_schema_lists_the_finding_in_text_mode() {
    let project = Scratch::project(&[
        (
            ".typdoc/collections/notes.json",
            r#"{ "match": "*.md", "schema": "note.json" }"#,
        ),
        (
            ".typdoc/collections/tasks.json",
            r#"{ "match": "tasks/*.md", "schema": "task.json" }"#,
        ),
        (
            "note.json",
            r#"{ "name": "note", "fields": { "title": { "type": "string", "required": true } } }"#,
        ),
        (
            "task.json",
            r#"{ "name": "task", "fields": { "title": { "type": "string", "required": true }, "status": { "type": "enum", "values": ["open", "done"], "required": true } } }"#,
        ),
    ]);
    project.file("plain.md", "---\ntitle: Plain\n---\n");

    let ran = mv_text(&project, "plain.md", "tasks/plain.md");

    assert_eq!(ran.code, 0, "stderr: {}", ran.stderr);
    assert_eq!(
        ran.stdout,
        "path: tasks/plain.md\n\
         collection: tasks\n\
         schema: task\n\
         namespace: default\n\
         title: Plain\n\
         rewritten: 0 refs in 0 documents\n\
         unrewritten: none\n\
         findings:\n\
         tasks/plain.md#status: frontmatter.types error: the field `status` is required and is missing\n"
    );
}

#[test]
fn mv_without_json_prints_a_plain_text_error() {
    let project = Scratch::project(&NOTES);
    project.file("a.md", "---\ntitle: A\n---\n");
    project.file("b.md", "---\ntitle: B\n---\n");

    let ran = mv_text(&project, "a.md", "b.md");

    assert_eq!(ran.code, 7, "{}", ran.stderr);
    assert_eq!(ran.stdout, "");
    assert!(
        ran.stderr.starts_with("typdoc: ") && !ran.stderr.starts_with("typdoc: {"),
        "{}",
        ran.stderr
    );
    assert_eq!(project.read("a.md"), "---\ntitle: A\n---\n");
    assert_eq!(project.read("b.md"), "---\ntitle: B\n---\n");
}
