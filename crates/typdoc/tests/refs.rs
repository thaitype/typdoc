//! Covers SPC-2, SPC-5, SPC-12, SPC-14.

#[allow(dead_code, reason = "each test file uses part of the shared helper")]
mod common;

use common::{NOTES, Ran, Scratch, Spawn, fixture};
use serde_json::{Value, json};

fn refs(project: &std::path::Path, args: &[&str]) -> Ran {
    Spawn::args(["refs"].into_iter().chain(args.iter().copied()))
        .cwd(project)
        .run()
}

fn error_of(ran: &Ran, code: i32) -> Value {
    assert_eq!(ran.code, code, "stderr: {}", ran.stderr);
    assert_eq!(ran.stdout, "", "a failure prints nothing on stdout");
    let object = ran.stderr_json();
    assert_eq!(object["code"], json!(code));
    object
}

#[test]
fn outgoing_refs_are_in_document_field_order_then_body_by_position() {
    // The schema declares `blocked_by` before `context` and `WF-1.md` writes them the other way
    // round, so the two orders differ.
    let ran = refs(&fixture("valid/refs"), &["tickets/WF-1.md", "--json"]);

    assert_eq!(ran.code, 0, "stderr: {}", ran.stderr);
    assert_eq!(ran.stderr, "");
    let out = ran.stdout_json();
    assert_eq!(
        out["document"],
        json!({ "path": "tickets/WF-1.md", "namespace": "default", "key": "WF-1" })
    );
    assert_eq!(out["direction"], json!("out"));
    let fields: Vec<&str> = out["refs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["field"].as_str().unwrap())
        .collect();
    assert_eq!(
        fields,
        [
            "context",
            "blocked_by",
            "blocked_by",
            "$body",
            "$body",
            "$body"
        ]
    );
}

#[test]
fn a_resolved_ref_carries_path_and_never_unresolved() {
    let ran = refs(&fixture("valid/refs"), &["tickets/WF-1.md", "--json"]);
    let out = ran.stdout_json();

    let resolved = &out["refs"][1];
    assert_eq!(resolved["field"], json!("blocked_by"));
    assert_eq!(resolved["written"], json!("WF-2"));
    assert_eq!(resolved["path"], json!("tickets/WF-2.md"));
    assert_eq!(resolved["key"], json!("WF-2"));
    assert!(resolved.get("unresolved").is_none(), "{resolved}");
}

#[test]
fn a_dangling_ref_carries_unresolved_and_never_path() {
    let ran = refs(&fixture("valid/refs"), &["tickets/WF-1.md", "--json"]);
    let out = ran.stdout_json();

    let dangling = &out["refs"][2];
    assert_eq!(dangling["field"], json!("blocked_by"));
    assert_eq!(dangling["written"], json!("WF-99"));
    assert_eq!(dangling["unresolved"], json!("not-found"));
    assert!(dangling.get("path").is_none(), "{dangling}");

    for reference in out["refs"].as_array().unwrap() {
        let has_path = reference.get("path").is_some();
        let has_unresolved = reference.get("unresolved").is_some();
        assert!(
            has_path != has_unresolved,
            "exactly one of path/unresolved: {reference}"
        );
    }
}

#[test]
fn a_bad_prefix_is_reported_in_frontmatter_and_in_the_body() {
    let ran = refs(&fixture("valid/refs"), &["tickets/WF-1.md", "--json"]);
    let out = ran.stdout_json();

    assert_eq!(out["refs"][0]["field"], json!("context"));
    assert_eq!(out["refs"][0]["unresolved"], json!("bad-prefix"));
    assert_eq!(out["refs"][4]["field"], json!("$body"));
    assert_eq!(out["refs"][4]["unresolved"], json!("bad-prefix"));
}

#[test]
fn a_body_ref_carries_line_and_col_and_a_frontmatter_ref_does_not() {
    let ran = refs(&fixture("valid/refs"), &["tickets/WF-1.md", "--json"]);
    let out = ran.stdout_json();

    assert_eq!(out["refs"][0].get("line"), None, "a frontmatter ref");
    let body = &out["refs"][3];
    assert_eq!(body["written"], json!("../notes/a.md"));
    assert_eq!(body["path"], json!("notes/a.md"));
    assert_eq!(body["line"], json!(7));
    assert_eq!(body["col"], json!(8));
}

#[test]
fn a_body_link_with_no_path_at_all_is_not_a_ref_and_is_left_out_of_body() {
    let project = Scratch::project(&NOTES);
    project.file(
        "a.md",
        "---\ntitle: A\n---\n\n[top](#a), [empty]().\n\n# A\n",
    );

    let ran = refs(project.path(), &["a.md", "--json"]);

    assert_eq!(ran.code, 0, "stderr: {}", ran.stderr);
    assert_eq!(ran.stdout_json()["refs"], json!([]));
}

#[test]
fn reverse_scans_every_namespace_and_orders_by_the_holders_path() {
    let ran = refs(
        &fixture("valid/refs"),
        &["tickets/WF-2.md", "--reverse", "--json"],
    );

    assert_eq!(ran.code, 0, "stderr: {}", ran.stderr);
    let out = ran.stdout_json();
    assert_eq!(out["direction"], json!("in"));
    let holders: Vec<&str> = out["refs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["path"].as_str().unwrap())
        .collect();
    assert_eq!(
        holders,
        ["notes/a.md", "tickets/WF-1.md", "tickets/WF-3.md"]
    );
    assert_eq!(out["refs"][0]["field"], json!("$body"));
    assert_eq!(out["refs"][0]["line"], json!(5));
    assert_eq!(out["refs"][1]["field"], json!("blocked_by"));
    assert_eq!(out["refs"][1]["key"], json!("WF-1"));
    assert_eq!(out["refs"][2]["field"], json!("context"));
    assert_eq!(out["refs"][2]["key"], json!("WF-3"));
}

#[test]
fn reverse_never_lists_an_unresolved_reference() {
    let ran = refs(
        &fixture("valid/refs"),
        &["tickets/WF-2.md", "--reverse", "--json"],
    );
    let out = ran.stdout_json();

    for reference in out["refs"].as_array().unwrap() {
        assert!(reference.get("unresolved").is_none(), "{reference}");
    }
}

#[test]
fn field_keeps_only_the_refs_held_in_that_field_in_either_direction() {
    let out = refs(
        &fixture("valid/refs"),
        &["tickets/WF-1.md", "--field", "blocked_by", "--json"],
    )
    .stdout_json();
    let fields: Vec<&str> = out["refs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["field"].as_str().unwrap())
        .collect();
    assert_eq!(fields, ["blocked_by", "blocked_by"]);

    let reverse = refs(
        &fixture("valid/refs"),
        &[
            "tickets/WF-2.md",
            "--reverse",
            "--field",
            "context",
            "--json",
        ],
    )
    .stdout_json();
    let holders: Vec<&str> = reverse["refs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["path"].as_str().unwrap())
        .collect();
    assert_eq!(holders, ["tickets/WF-3.md"]);
}

#[test]
fn field_body_keeps_only_the_body_link_in_either_direction() {
    let out = refs(
        &fixture("valid/refs"),
        &["tickets/WF-1.md", "--field", "$body", "--json"],
    )
    .stdout_json();
    assert_eq!(out["refs"].as_array().unwrap().len(), 3);

    let reverse = refs(
        &fixture("valid/refs"),
        &["tickets/WF-2.md", "--reverse", "--field", "$body", "--json"],
    )
    .stdout_json();
    let holders: Vec<&str> = reverse["refs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["path"].as_str().unwrap())
        .collect();
    assert_eq!(holders, ["notes/a.md"]);
}

#[test]
fn a_coded_key_argument_names_the_same_document_as_its_path() {
    let by_path = refs(&fixture("valid/refs"), &["tickets/WF-1.md", "--json"]).stdout_json();
    let by_key = refs(&fixture("valid/refs"), &["WF-1", "--json"]).stdout_json();

    assert_eq!(by_path, by_key);
}

#[test]
fn a_document_that_is_not_there_exits_5() {
    let ran = refs(&fixture("valid/refs"), &["absent.md", "--json"]);

    error_of(&ran, 5);
}

#[test]
fn an_argument_that_is_neither_a_path_nor_a_key_exits_1() {
    let ran = refs(&fixture("valid/refs"), &["nope", "--json"]);

    error_of(&ran, 1);
}

/// The `context` row's `written` equals its `document` and the `$body` row's does not, so the two
/// rows show that `written` is more than a copy of `document`.
#[test]
fn refs_without_json_prints_the_designs_worked_example() {
    let ran = refs(&fixture("valid/refs-worked-example"), &["team/doc.md"]);

    assert_eq!(ran.code, 0, "stderr: {}", ran.stderr);
    assert_eq!(ran.stderr, "");
    assert_eq!(
        ran.stdout,
        "document             field    written\n\
         chief:WF-7           context  chief:WF-7\n\
         team/learnings/x.md  $body    learnings/x.md\n"
    );
}

/// `valid/refs` has one namespace, so a coded holder prints its bare key.
#[test]
fn refs_reverse_without_json_prints_the_holders_name_and_field_per_line() {
    let ran = refs(&fixture("valid/refs"), &["tickets/WF-2.md", "--reverse"]);

    assert_eq!(ran.code, 0, "stderr: {}", ran.stderr);
    assert_eq!(ran.stderr, "");
    assert_eq!(
        ran.stdout,
        "document    field\n\
         notes/a.md  $body\n\
         WF-1        blocked_by\n\
         WF-3        context\n"
    );
}

#[test]
fn refs_field_without_json_keeps_only_that_fields_refs() {
    let ran = refs(
        &fixture("valid/refs"),
        &["tickets/WF-1.md", "--field", "blocked_by"],
    );

    assert_eq!(ran.code, 0, "stderr: {}", ran.stderr);
    assert_eq!(ran.stderr, "");
    assert_eq!(
        ran.stdout,
        "document                 field       written\n\
         WF-2                     blocked_by  WF-2\n\
         (unresolved: not-found)  blocked_by  WF-99\n"
    );
}

#[test]
fn refs_without_json_prints_nothing_when_there_are_no_refs() {
    let ran = refs(&fixture("valid/refs"), &["tickets/WF-2.md"]);

    assert_eq!(ran.code, 0, "stderr: {}", ran.stderr);
    assert_eq!(ran.stderr, "");
    assert_eq!(ran.stdout, "");
}

#[test]
fn refs_without_json_prints_a_plain_text_error() {
    let ran = refs(&fixture("valid/refs"), &["absent.md"]);

    assert_eq!(ran.code, 5);
    assert_eq!(ran.stdout, "");
    assert!(
        ran.stderr.starts_with("typdoc: ") && !ran.stderr.starts_with("typdoc: {"),
        "{}",
        ran.stderr
    );
}

#[test]
fn a_document_with_no_outgoing_ref_has_an_empty_list() {
    let ran = refs(&fixture("valid/refs"), &["tickets/WF-2.md", "--json"]);

    assert_eq!(ran.code, 0, "stderr: {}", ran.stderr);
    assert_eq!(ran.stdout_json()["refs"], json!([]));
}
