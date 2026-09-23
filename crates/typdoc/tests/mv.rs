//! `typdoc mv` within one project: refs rewritten in frontmatter and in body links, the four
//! refusals, the schema-mismatch case that is carried out rather than refused, and the recovery
//! a stopped run leaves for the same command to finish.

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

// ---------------------------------------------------------------------------------------------
// Done when (a): a stopped run is its own way back.
// ---------------------------------------------------------------------------------------------

/// The two-phase mechanism itself — that a stop partway through the renames leaves some done and
/// the rest not, and the document itself moved last of all — is proved deterministically at the
/// seam, against a fake file system staged to stop after a fixed number of operations, in
/// `crates/typdoc-core/tests/mv_seam.rs`; that is the level decision 1's own promise is about,
/// and a fake is what lets the test choose exactly where to stop.
///
/// What this test proves instead is the behaviour that promise is *for*: the state such a stop
/// leaves — a holder already naming the new path, the document still at the old one — is exactly
/// what `validate` reports as broken and what running the identical command again repairs,
/// without re-touching the holder that was already correct. The state is built by hand rather
/// than by racing a live interrupt, which is the same state a real stop leaves (`mv_seam.rs`
/// shows that directly) and is what a user actually meets and re-runs against.
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

// ---------------------------------------------------------------------------------------------
// Done when (b): each refusal, produced by a test, leaves every file byte-identical.
// ---------------------------------------------------------------------------------------------

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

#[test]
fn the_same_path_given_twice_is_one_file_and_is_refused_at_exit_7() {
    let project = Scratch::project(&NOTES);
    project.file("a.md", "---\ntitle: A\n---\n");

    let ran = mv(&project, "a.md", "a.md");

    assert_eq!(ran.code, 7, "{}", ran.stderr);
    assert_eq!(project.read("a.md"), "---\ntitle: A\n---\n");
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

// ---------------------------------------------------------------------------------------------
// Done when (c): a move that fails the destination schema is carried out, not refused.
// ---------------------------------------------------------------------------------------------

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
        "the move happened; validation is a separate concern (decision 16): {}",
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

// ---------------------------------------------------------------------------------------------
// Done when (d): refs held by another project are named, not counted — the reachable half.
// See the completion report for why `imported-project` itself is not reachable by any fixture
// this ticket can build (it needs the reverse-into-imports scan `[reverse-scope]` leaves as a
// known, story-wide gap); `links-rule-off` is the other of the three reasons this ticket can
// actually produce, and is tested below.
// ---------------------------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------------------------
// Refs are rewritten in frontmatter and in body links, keeping each one's own written form.
// ---------------------------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------------------------
// `auto: moves` and the mode carried across.
// ---------------------------------------------------------------------------------------------

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

/// Unlike the document's own move, which is a plain rename and so carries its mode for free, a
/// holder whose ref is rewritten is a genuine content replacement — a temp file prepared and
/// renamed over the original (`prepare_replacement`) — which is exactly the case decision 4's
/// mode-carrying rule is for: without it, the rewritten file would take whatever mode the umask
/// hands a new file, silently.
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
