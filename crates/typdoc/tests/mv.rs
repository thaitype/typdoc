//! Covers SPC-1, SPC-2, SPC-5, SPC-10, SPC-12, SPC-17, SPC-18.

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

/// A mention is always a key (`links::mentions`), so a move of a document without a code has no
/// mention to report.
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

/// Two links to the moved document on one line, one written with `./`: each keeps its form and
/// its anchor, and the project validates clean after the move. The new name is shorter, so a
/// splice of the first link would move the second back from the column it was found at.
#[test]
fn every_body_link_on_a_line_is_rewritten_keeping_its_dot_slash_and_its_anchor() {
    let project = Scratch::project(&NOTES);
    project.file("a-much-longer-name.md", "---\ntitle: B\n---\n\n## Part\n");
    project.file(
        "a.md",
        "---\ntitle: A\n---\n\n[one](./a-much-longer-name.md#part) and [two](a-much-longer-name.md#part)\n",
    );

    let ran = mv(&project, "a-much-longer-name.md", "c.md");

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    assert_eq!(
        project.read("a.md"),
        "---\ntitle: A\n---\n\n[one](./c.md#part) and [two](c.md#part)\n"
    );
    let validated = common::Spawn::args(["validate", "--json"])
        .cwd(project.path())
        .run();
    assert_eq!(validated.code, 0, "{}", validated.stdout);
}

/// A written `./` is kept even where the new path climbs out of the folder.
#[test]
fn a_dot_slash_is_kept_in_front_of_a_path_that_climbs_out_of_the_folder() {
    let project = Scratch::project(&[
        (
            ".typdoc/collections/notes.json",
            r#"{ "match": "**/*.md", "schema": "note.json" }"#,
        ),
        ("note.json", r#"{ "name": "note", "fields": {} }"#),
    ]);
    project.file("a.md", "---\n---\n");
    project.file("sub/h.md", "---\n---\n\n[a](./../a.md)\n");

    let ran = mv(&project, "a.md", "b.md");

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    assert_eq!(project.read("sub/h.md"), "---\n---\n\n[a](./../b.md)\n");
}

/// Under `refBase: namespace`, a path with no prefix cannot reach a document moved out of the
/// namespace folder, so `mv` changes its form: a frontmatter ref takes the namespace prefix, and a
/// body link, which a Markdown reader follows with no prefix, the path from the document.
#[test]
fn a_no_prefix_path_to_a_document_moved_out_of_the_namespace_becomes_prefixed_or_a_link_path() {
    let project = Scratch::project(&[
        (
            ".typdoc/config.json",
            r#"{ "version": 1, "namespaces": ["story-1", "story-2"] }"#,
        ),
        (
            ".typdoc/collections/notes.json",
            r#"{ "match": "notes/*.md", "schema": "note.json", "refBase": "namespace" }"#,
        ),
        (
            "note.json",
            r#"{ "name": "note", "fields": { "see": { "type": "ref[]", "target": "*" } } }"#,
        ),
    ]);
    project.file("story-1/notes/q.md", "---\n---\n");
    project.file("story-2/.keep", "");
    project.file(
        "story-1/notes/h.md",
        "---\nsee: [notes/q.md]\n---\n\n[q](notes/q.md)\n",
    );

    let ran = mv(&project, "story-1:notes/q.md", "story-2:notes/q.md");

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    assert_eq!(
        project.read("story-1/notes/h.md"),
        "---\nsee:\n- story-2:notes/q.md\n---\n\n[q](../../story-2/notes/q.md)\n"
    );
    let validated = common::Spawn::args(["validate", "--json"])
        .cwd(project.path())
        .run();
    assert_eq!(validated.code, 0, "{}", validated.stdout);
}

/// No prefix names a folder outside every namespace, so a prefixed ref becomes a path from the
/// document that holds it.
#[test]
fn a_prefixed_ref_to_a_document_moved_outside_every_namespace_becomes_a_path() {
    let project = Scratch::project(&[
        (
            ".typdoc/config.json",
            r#"{ "version": 1, "namespaces": ["story-1", "story-2"] }"#,
        ),
        (
            ".typdoc/collections/notes.json",
            r#"{ "match": "notes/*.md", "schema": "note.json" }"#,
        ),
        (
            "note.json",
            r#"{ "name": "note", "fields": { "see": { "type": "ref[]", "target": "*" } } }"#,
        ),
    ]);
    project.file("story-1/notes/a.md", "---\n---\n");
    project.file(
        "story-2/notes/h.md",
        "---\nsee: [story-1:notes/a.md]\n---\n\n[l](story-1:notes/a.md)\n",
    );

    let ran = mv(&project, "story-1:notes/a.md", "outside/a.md");

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    assert_eq!(
        project.read("story-2/notes/h.md"),
        "---\nsee:\n- ../../outside/a.md\n---\n\n[l](../../outside/a.md)\n"
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
         ref: default:renamed.md\n\
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
         ref: default:renamed.md\n\
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
         ref: default:new.md\n\
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
         ref: default:new.md\n\
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
         ref: default:b.md\n\
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
         ref: default:tasks/plain.md\n\
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

// --- Changing a slug ---

/// Two namespaces: `story-2` holds the tickets, under `slug` when one is given, and `story-1`
/// holds the notes whose refs a test reads. The ticket schema records moves.
fn slug_project(slug: Option<&str>) -> Scratch {
    let slug = slug
        .map(|mode| format!(r#", "slug": "{mode}""#))
        .unwrap_or_default();
    let project = Scratch::project(&[]);
    project.file(
        ".typdoc/config.json",
        r#"{ "version": 1, "namespaces": ["story-1", "story-2"] }"#,
    );
    project.file(
        ".typdoc/collections/tickets.json",
        &format!(r#"{{ "match": "tickets/{{key}}.md", "schema": "ticket.json"{slug} }}"#),
    );
    project.file(
        ".typdoc/collections/notes.json",
        r#"{ "match": "notes/*.md", "schema": "note.json" }"#,
    );
    project.file(
        "ticket.json",
        r#"{ "name": "ticket", "code": "WF", "fields": {
            "title": { "type": "string" },
            "moved_from": { "type": "list", "auto": "moves" }
        } }"#,
    );
    project.file(
        "note.json",
        r#"{ "name": "note", "fields": { "see": { "type": "ref[]", "target": "*" } } }"#,
    );
    project.file(
        ".typdoc/state/story-2.json",
        "{\n  \"tickets\": {\n    \"last\": 5\n  }\n}\n",
    );
    project.file("story-1/notes/.keep", "");
    project
}

fn validate_json(project: &Scratch) -> Ran {
    common::Spawn::args(["validate", "--json"])
        .cwd(project.path())
        .run()
}

#[test]
fn a_slug_changes_under_the_same_key_and_the_output_names_the_key_alone() {
    let project = slug_project(None);
    project.file(
        "story-2/tickets/WF-5-json-output-shape.md",
        "---\ntitle: Five\n---\n",
    );

    let ran = mv(
        &project,
        "story-2:WF-5",
        "story-2/tickets/WF-5-json-shapes.md",
    );

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    let out = ran.stdout_json();
    assert_eq!(out["document"]["key"], json!("WF-5"));
    assert_eq!(
        out["document"]["path"],
        json!("story-2/tickets/WF-5-json-shapes.md")
    );
    assert_eq!(out["document"]["collection"], json!("tickets"));
    assert_eq!(out["findings"], json!([]));
    assert!(
        !project
            .path()
            .join("story-2/tickets/WF-5-json-output-shape.md")
            .exists()
    );
    assert_eq!(
        project.read("story-2/tickets/WF-5-json-shapes.md"),
        "---\ntitle: Five\nmoved_from:\n- story-2/tickets/WF-5-json-output-shape.md\n---\n"
    );
    assert_eq!(validate_json(&project).code, 0);
}

#[test]
fn a_slug_is_added_to_a_name_that_had_none() {
    let project = slug_project(None);
    project.file("story-2/tickets/WF-5.md", "---\ntitle: Five\n---\n");

    let ran = mv(
        &project,
        "story-2:WF-5",
        "story-2/tickets/WF-5-lock-order.md",
    );

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    let out = ran.stdout_json();
    assert_eq!(out["document"]["key"], json!("WF-5"));
    assert_eq!(
        out["document"]["path"],
        json!("story-2/tickets/WF-5-lock-order.md")
    );
    assert!(!project.path().join("story-2/tickets/WF-5.md").exists());
    assert!(
        project
            .path()
            .join("story-2/tickets/WF-5-lock-order.md")
            .exists()
    );
}

#[test]
fn a_slug_is_removed_and_a_ref_written_with_it_becomes_the_key_alone_with_its_prefix() {
    let project = slug_project(None);
    project.file("story-2/tickets/WF-5-old.md", "---\ntitle: Five\n---\n");
    project.file("story-1/notes/a.md", "---\nsee:\n- story-2:WF-5-old\n---\n");

    let ran = mv(
        &project,
        "story-2/tickets/WF-5-old.md",
        "story-2/tickets/WF-5.md",
    );

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    assert_eq!(
        ran.stdout_json()["document"]["path"],
        json!("story-2/tickets/WF-5.md")
    );
    assert_eq!(
        project.read("story-1/notes/a.md"),
        "---\nsee:\n- story-2:WF-5\n---\n"
    );
    assert_eq!(validate_json(&project).code, 0);
}

/// Each ref keeps the form it was written in: the key alone does not change, a key written with
/// the old slug gets the new one under the prefix it had, and a body link, `<…>` form kept, names
/// the new file. A key written with a slug that does not match the key is another document's and
/// is left alone.
#[test]
fn refs_are_rewritten_in_the_form_each_was_written_in_across_namespaces() {
    let project = slug_project(None);
    project.file("story-2/tickets/WF-4.md", "---\ntitle: Four\n---\n");
    project.file("story-2/tickets/WF-5-old.md", "---\ntitle: Five\n---\n");
    project.file(
        "story-1/notes/a.md",
        "---\nsee:\n- story-2:WF-5\n- story-2:WF-5-old\n- story-2:WF-4\n---\n\n\
         See [five](../../story-2/tickets/WF-5-old.md) and [again](<../../story-2/tickets/WF-5-old.md>).\n",
    );
    project.file(
        "story-2/notes/b.md",
        "---\nsee:\n- WF-5-old\n- WF-5\n---\n\nWF-5 is mentioned here.\n",
    );

    let ran = mv(&project, "story-2:WF-5-old", "story-2/tickets/WF-5-new.md");

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    assert_eq!(
        project.read("story-1/notes/a.md"),
        "---\nsee:\n- story-2:WF-5\n- story-2:WF-5-new\n- story-2:WF-4\n---\n\n\
         See [five](../../story-2/tickets/WF-5-new.md) and [again](<../../story-2/tickets/WF-5-new.md>).\n"
    );
    assert_eq!(
        project.read("story-2/notes/b.md"),
        "---\nsee:\n- WF-5-new\n- WF-5\n---\n\nWF-5 is mentioned here.\n"
    );
    let out = ran.stdout_json();
    let rewritten = out["rewritten"].as_array().expect("an array");
    assert_eq!(
        rewritten.len(),
        4,
        "a ref by the key alone is not rewritten, so it is not listed: {rewritten:?}"
    );
    assert!(
        rewritten
            .iter()
            .any(|r| r["document"] == json!("story-2/notes/b.md")
                && r["before"] == json!("WF-5-old")
                && r["after"] == json!("WF-5-new")),
        "{rewritten:?}"
    );
    assert_eq!(
        out["unrewritten"],
        json!([]),
        "the key did not change, so no mention is stale"
    );
    assert_eq!(validate_json(&project).code, 0);
}

/// `auto: moves` records the previous path, so a body link written later to the old file name is
/// `refs.moved`, naming the new path, rather than a missing file.
#[test]
fn auto_moves_records_the_previous_path_and_refs_moved_finds_a_link_left_on_the_old_name() {
    let project = slug_project(None);
    project.file("story-2/tickets/WF-5-old.md", "---\ntitle: Five\n---\n");

    let ran = mv(&project, "story-2:WF-5", "story-2/tickets/WF-5-new.md");
    assert_eq!(ran.code, 0, "{}", ran.stderr);
    assert_eq!(
        ran.stdout_json()["document"]["fields"]["moved_from"],
        json!(["story-2/tickets/WF-5-old.md"])
    );

    project.file(
        "story-1/notes/late.md",
        "---\nsee: []\n---\n\nSee [five](../../story-2/tickets/WF-5-old.md).\n",
    );
    let validated = validate_json(&project);
    assert_eq!(validated.code, 2, "{}", validated.stdout);
    let findings = validated.stdout_json()["findings"].clone();
    assert_eq!(findings.as_array().map(Vec::len), Some(1), "{findings}");
    assert_eq!(findings[0]["rule"], json!("refs.moved"));
    assert_eq!(findings[0]["path"], json!("story-1/notes/late.md"));
    assert_eq!(
        findings[0]["message"],
        json!(
            "the ref `../../story-2/tickets/WF-5-old.md` no longer resolves: it was moved to \
             `story-2/tickets/WF-5-new.md`"
        )
    );
}

#[test]
fn another_key_in_the_same_folder_is_refused_and_nothing_changes() {
    let project = slug_project(None);
    project.file("story-2/tickets/WF-5-old.md", "---\ntitle: Five\n---\n");

    for to in [
        "story-2/tickets/WF-6-old.md",
        "story-2/tickets/WF-55.md",
        "story-2/other/WF-5-old.md",
    ] {
        let ran = mv(&project, "story-2:WF-5", to);

        assert_eq!(ran.code, 1, "{to}: {}", ran.stderr);
        assert!(
            ran.stderr_json()["error"]
                .as_str()
                .unwrap()
                .contains("fixed by its key `WF-5`"),
            "{to}: {}",
            ran.stderr
        );
        assert!(!project.path().join(to).exists(), "{to}");
    }
    assert_eq!(
        project.read("story-2/tickets/WF-5-old.md"),
        "---\ntitle: Five\n---\n"
    );
}

#[test]
fn the_same_key_in_another_namespace_is_still_refused_with_the_renumber_message() {
    let project = slug_project(None);
    project.file("story-2/tickets/WF-5-old.md", "---\ntitle: Five\n---\n");

    let ran = mv(&project, "story-2:WF-5", "story-1/tickets/WF-5-old.md");

    assert_eq!(ran.code, 1, "{}", ran.stderr);
    assert!(
        ran.stderr_json()["error"]
            .as_str()
            .unwrap()
            .contains("mv --renumber"),
        "{}",
        ran.stderr
    );
    assert!(!project.path().join("story-1/tickets/WF-5-old.md").exists());
}

/// typdoc never writes an invalid slug, in any `slug` mode: under `none` a name with text after
/// the key reads as the form the collection does not expect, and an excluded character there is
/// refused all the same.
#[test]
fn a_destination_with_an_invalid_slug_is_refused_and_nothing_is_written() {
    for (mode, to) in [
        (None, "story-2/tickets/WF-5-a b.md"),
        (None, "story-2/tickets/WF-5-.md"),
        (None, "story-2/tickets/WF-5-a#b.md"),
        (Some("required"), "./story-2/tickets/WF-5-a:b.md"),
        (Some("none"), "story-2/tickets/WF-5-a#b.md"),
        (Some("none"), "story-2/tickets/WF-5-.md"),
    ] {
        let project = slug_project(mode);
        project.file("story-2/tickets/WF-5-old.md", "---\ntitle: Five\n---\n");
        project.file("story-1/notes/a.md", "---\nsee:\n- story-2:WF-5-old\n---\n");

        let ran = mv(&project, "story-2:WF-5", to);

        assert_eq!(ran.code, 1, "{mode:?} {to}: {}", ran.stderr);
        assert!(
            ran.stderr_json()["error"]
                .as_str()
                .unwrap()
                .contains("a slug is not empty and holds no whitespace, `/`, `#` or `:`"),
            "{mode:?} {to}: {}",
            ran.stderr
        );
        assert!(!project.path().join(to).exists(), "{mode:?} {to}");
        assert_eq!(
            project.read("story-2/tickets/WF-5-old.md"),
            "---\ntitle: Five\n---\n"
        );
        assert_eq!(
            project.read("story-1/notes/a.md"),
            "---\nsee:\n- story-2:WF-5-old\n---\n"
        );
    }
}

#[test]
fn a_move_to_the_form_the_collection_does_not_expect_is_carried_out_and_reported() {
    for (mode, from, to, message) in [
        (
            "none",
            "story-2/tickets/WF-5.md",
            "story-2/tickets/WF-5-x.md",
            "the file name has the slug `x` after the key `WF-5`, and the collection `tickets` \
             has `slug` set to `none`: it takes no slug",
        ),
        (
            "required",
            "story-2/tickets/WF-5-x.md",
            "story-2/tickets/WF-5.md",
            "the file name has no slug after the key `WF-5`, and the collection `tickets` has \
             `slug` set to `required`: it needs one",
        ),
    ] {
        let project = slug_project(Some(mode));
        project.file(from, "---\ntitle: Five\n---\n");

        let ran = mv(&project, from, to);

        assert_eq!(ran.code, 0, "{mode}: {}", ran.stderr);
        let out = ran.stdout_json();
        assert_eq!(out["document"]["path"], json!(to), "{mode}");
        assert_eq!(out["document"]["key"], json!("WF-5"), "{mode}");
        assert_eq!(
            out["findings"],
            json!([{
                "path": to,
                "namespace": "story-2",
                "collection": "tickets",
                "key": "WF-5",
                "rule": "filename.pattern",
                "level": "error",
                "message": message,
            }]),
            "{mode}"
        );
        assert!(!project.path().join(from).exists(), "{mode}");
        assert!(project.path().join(to).exists(), "{mode}");
    }
}

/// As if an earlier run had rewritten the ref and stopped before renaming the document: the ref
/// already names the new slug, resolves by its key, and is not rewritten again.
#[test]
fn a_slug_change_run_again_finishes_a_run_a_stop_left_half_done() {
    let project = slug_project(None);
    project.file("story-2/tickets/WF-5-old.md", "---\ntitle: Five\n---\n");
    project.file("story-1/notes/a.md", "---\nsee:\n- story-2:WF-5-new\n---\n");
    project.file("story-1/notes/b.md", "---\nsee:\n- story-2:WF-5-old\n---\n");

    let ran = mv(&project, "story-2:WF-5", "story-2/tickets/WF-5-new.md");

    assert_eq!(ran.code, 0, "{}", ran.stderr);
    let out = ran.stdout_json();
    assert_eq!(
        out["rewritten"],
        json!([{
            "document": "story-1/notes/b.md",
            "field": "see",
            "before": "story-2:WF-5-old",
            "after": "story-2:WF-5-new",
        }])
    );
    assert_eq!(
        project.read("story-1/notes/a.md"),
        "---\nsee:\n- story-2:WF-5-new\n---\n"
    );
    assert_eq!(
        project.read("story-1/notes/b.md"),
        "---\nsee:\n- story-2:WF-5-new\n---\n"
    );
    assert!(project.path().join("story-2/tickets/WF-5-new.md").exists());
    assert_eq!(validate_json(&project).code, 0);
}
