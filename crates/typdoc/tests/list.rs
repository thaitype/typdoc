//! `typdoc list` through the built binary: the default table, `--json`, `--ids`, the collection
//! and code filter, `--where` (including the scope-wide field check this ticket owns), `--sort`,
//! `--limit`, `--fields`, and the scope by namespace.

#[allow(dead_code, reason = "each test file uses part of the shared helper")]
mod common;

use common::{Ran, Scratch, Spawn, fixture};
use serde_json::json;

/// A project with one coded collection, `tickets/{key}.md`, code `T`: `title` (string), `status`
/// (enum, `open` before `closed`, the reverse of the design's own `open`/`resolved` example, on
/// purpose, so an enum-order sort test cannot pass by accident agreeing with key order), and
/// `priority` (number, not required, so a document can leave it unset).
const TICKETS: [(&str, &str); 2] = [
    (
        ".typdoc/collections/tickets.json",
        r#"{ "match": "tickets/{key}.md", "schema": "ticket.json" }"#,
    ),
    (
        "ticket.json",
        r#"{ "name": "ticket", "code": "T", "fields": {
              "title": { "type": "string" },
              "status": { "type": "enum", "values": ["open", "closed"] },
              "priority": { "type": "number" }
            } }"#,
    ),
];

fn list(project: &std::path::Path, args: &[&str]) -> Ran {
    Spawn::args(["list"].into_iter().chain(args.iter().copied()))
        .cwd(project)
        .run()
}

fn error_of(ran: &Ran, code: i32) -> serde_json::Value {
    assert_eq!(ran.code, code, "stderr: {}", ran.stderr);
    assert_eq!(ran.stdout, "", "a failure prints nothing on stdout");
    let object = ran.stderr_json();
    assert_eq!(object["code"], json!(code));
    object
}

// ---------------------------------------------------------------------------------------------
// --json shape: documents, total, truncated
// ---------------------------------------------------------------------------------------------

#[test]
fn json_prints_documents_total_and_truncated() {
    let ran = list(&fixture("valid/refs"), &["--json"]);

    assert_eq!(ran.code, 0, "stderr: {}", ran.stderr);
    assert_eq!(ran.stderr, "");
    let out = ran.stdout_json();
    assert_eq!(out["total"], json!(4));
    assert_eq!(out["truncated"], json!(false));
    assert_eq!(out["documents"].as_array().unwrap().len(), 4);
}

#[test]
fn a_document_in_list_is_the_same_shape_as_get() {
    let ran = list(&fixture("valid/refs"), &["--code", "WF", "--json"]);
    let out = ran.stdout_json();

    let wf1 = out["documents"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["path"] == json!("tickets/WF-1.md"))
        .unwrap();
    assert_eq!(
        wf1,
        &json!({
            "path": "tickets/WF-1.md",
            "namespace": "default",
            "key": "WF-1",
            "code": "WF",
            "collection": "tickets",
            "schema": "ticket",
            "fields": {
                "title": "Ticket one",
                "context": "chief::WF-5",
                "blocked_by": ["WF-2", "WF-99"]
            }
        })
    );
}

#[test]
fn an_empty_result_exits_0() {
    let ran = list(
        &fixture("valid/refs"),
        &["--where", "title=NoSuchTitle", "--json"],
    );

    assert_eq!(ran.code, 0, "stderr: {}", ran.stderr);
    assert_eq!(ran.stdout_json()["documents"], json!([]));
    assert_eq!(ran.stdout_json()["total"], json!(0));
    assert_eq!(ran.stdout_json()["truncated"], json!(false));
}

// ---------------------------------------------------------------------------------------------
// The scope-wide field check this ticket owns (ticket 13's thread)
// ---------------------------------------------------------------------------------------------

#[test]
fn a_field_no_schema_in_scope_declares_is_an_error_not_an_empty_result() {
    // `valid/refs` has two collections, `notes` (schema `note`: only `title`) and `tickets`
    // (schema `ticket`: `title`, `blocked_by`, `context`). No schema in the project declares
    // `nope`, so this must be an error (exit 1), never a well-formed query with an empty result.
    let ran = list(&fixture("valid/refs"), &["--where", "nope=x", "--json"]);

    let object = error_of(&ran, 1);
    assert!(
        object["error"].as_str().unwrap().contains("nope"),
        "{object}"
    );
}

#[test]
fn a_field_one_collection_has_and_another_lacks_is_absent_on_the_one_that_lacks_it_not_an_error() {
    // `blocked_by` is declared by `ticket` (the tickets collection) and not by `note` (the notes
    // collection). Over the whole project (both collections in scope), the field is known
    // somewhere, so the query is not an error; `notes/a.md`, whose own schema lacks the field,
    // must be treated as absent and simply excluded (`blocked_by=WF-2` is a positive condition,
    // which absence always fails), not as a scope-wide unknown-field error.
    let ran = list(
        &fixture("valid/refs"),
        &["--where", "blocked_by=WF-2", "--json"],
    );

    assert_eq!(ran.code, 0, "stderr: {}", ran.stderr);
    let out = ran.stdout_json();
    let paths: Vec<&str> = out["documents"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["path"].as_str().unwrap())
        .collect();
    assert_eq!(paths, ["tickets/WF-1.md"]);
}

#[test]
fn the_same_field_negated_satisfies_the_document_whose_schema_lacks_it() {
    // The complement of the case above: `!=` is satisfied by something absent (design, Absence
    // and negation), so `notes/a.md` (no `blocked_by` at all) must be included by `!=` even
    // though its own schema does not declare the field.
    let ran = list(
        &fixture("valid/refs"),
        &["--where", "blocked_by!=WF-2", "--json"],
    );

    assert_eq!(ran.code, 0, "stderr: {}", ran.stderr);
    let out = ran.stdout_json();
    let paths: Vec<&str> = out["documents"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["path"].as_str().unwrap())
        .collect();
    assert!(paths.contains(&"notes/a.md"), "{paths:?}");
}

// ---------------------------------------------------------------------------------------------
// --collection / --code
// ---------------------------------------------------------------------------------------------

#[test]
fn collection_selects_by_collection_name() {
    let ran = list(&fixture("valid/refs"), &["--collection", "notes", "--ids"]);

    assert_eq!(ran.code, 0, "stderr: {}", ran.stderr);
    assert_eq!(ran.stdout, "notes/a.md\n");
}

#[test]
fn code_is_a_shorthand_for_the_collections_whose_schema_has_that_code() {
    let ran = list(&fixture("valid/refs"), &["--code", "WF", "--ids"]);

    assert_eq!(ran.code, 0, "stderr: {}", ran.stderr);
    assert_eq!(ran.stdout, "WF-1\nWF-2\nWF-3\n");
}

#[test]
fn an_unknown_collection_name_is_a_bad_argument() {
    error_of(
        &list(&fixture("valid/refs"), &["--collection", "bogus", "--json"]),
        1,
    );
}

#[test]
fn an_unknown_code_is_a_bad_argument() {
    error_of(
        &list(&fixture("valid/refs"), &["--code", "ZZ", "--json"]),
        1,
    );
}

// ---------------------------------------------------------------------------------------------
// --sort
// ---------------------------------------------------------------------------------------------

#[test]
fn without_sort_results_are_in_key_or_path_order_numerically_not_lexicographically() {
    let project = Scratch::project(&TICKETS);
    project.file("tickets/T-2.md", "---\ntitle: two\nstatus: open\n---\n");
    project.file("tickets/T-10.md", "---\ntitle: ten\nstatus: open\n---\n");
    project.file("tickets/T-1.md", "---\ntitle: one\nstatus: open\n---\n");

    let ran = list(project.path(), &["--ids"]);

    assert_eq!(ran.code, 0, "stderr: {}", ran.stderr);
    assert_eq!(ran.stdout, "T-1\nT-2\nT-10\n");
}

#[test]
fn sort_by_a_number_field_orders_by_value_not_by_key() {
    let ran = list(
        &fixture("valid/field-types"),
        &["--code", "WF", "--sort", "estimate", "--ids"],
    );

    // WF-1 has estimate 5, WF-2 has estimate 0.5: ascending by value puts WF-2 first, the
    // opposite of key order, so this cannot pass by falling back to the identity tiebreak.
    assert_eq!(ran.code, 0, "stderr: {}", ran.stderr);
    assert_eq!(ran.stdout, "WF-2\nWF-1\n");

    let desc = list(
        &fixture("valid/field-types"),
        &["--code", "WF", "--sort", "estimate:desc", "--ids"],
    );
    assert_eq!(desc.stdout, "WF-1\nWF-2\n");
}

#[test]
fn sort_by_an_enum_field_orders_by_position_in_the_schemas_values() {
    let project = Scratch::project(&TICKETS);
    // The schema's `values` are `["open", "closed"]`, so `open` sorts before `closed`; by key
    // order T-1 (open) would already come before T-2, so give the *later* key the *earlier*
    // enum value to prove the enum's own order is read, not the key's.
    project.file("tickets/T-1.md", "---\ntitle: one\nstatus: closed\n---\n");
    project.file("tickets/T-2.md", "---\ntitle: two\nstatus: open\n---\n");

    let ran = list(project.path(), &["--sort", "status", "--ids"]);

    assert_eq!(ran.code, 0, "stderr: {}", ran.stderr);
    assert_eq!(ran.stdout, "T-2\nT-1\n");
}

#[test]
fn a_document_missing_the_sort_field_sorts_last_whichever_direction() {
    let project = Scratch::project(&TICKETS);
    project.file(
        "tickets/T-1.md",
        "---\ntitle: has priority\nstatus: open\npriority: 5\n---\n",
    );
    project.file(
        "tickets/T-2.md",
        "---\ntitle: no priority\nstatus: open\n---\n",
    );

    let asc = list(project.path(), &["--sort", "priority", "--ids"]);
    assert_eq!(asc.stdout, "T-1\nT-2\n");

    let desc = list(project.path(), &["--sort", "priority:desc", "--ids"]);
    assert_eq!(
        desc.stdout, "T-1\nT-2\n",
        "missing stays last even under :desc"
    );
}

#[test]
fn sort_ties_break_by_key_or_path_order() {
    let project = Scratch::project(&TICKETS);
    // Written to disk in an order that disagrees with key order, so a pass here cannot be
    // explained by an incidental walk order.
    project.file("tickets/T-3.md", "---\ntitle: three\nstatus: open\n---\n");
    project.file("tickets/T-1.md", "---\ntitle: one\nstatus: open\n---\n");

    let ran = list(project.path(), &["--sort", "status", "--ids"]);

    assert_eq!(ran.code, 0, "stderr: {}", ran.stderr);
    assert_eq!(ran.stdout, "T-1\nT-3\n");
}

#[test]
fn an_invalid_sort_direction_is_a_bad_argument() {
    error_of(
        &list(
            &fixture("valid/refs"),
            &["--sort", "title:sideways", "--json"],
        ),
        1,
    );
}

#[test]
fn a_sort_field_no_schema_declares_is_not_an_error_and_falls_back_to_key_or_path_order() {
    // Unlike `--where` (Names and scope: "a field name unknown to every schema in scope is an
    // error"), the design's Sorting table states no such rule, and that error is tied to "a
    // plain condition". An unknown `--sort` field changes nothing about which documents match or
    // their count, only how they are ordered, so every document reads as missing that key and
    // the result falls back to key or path order, still exit 0 with every document present.
    let ran = list(
        &fixture("valid/refs"),
        &["--code", "WF", "--sort", "nope", "--json"],
    );

    assert_eq!(ran.code, 0, "stderr: {}", ran.stderr);
    let out = ran.stdout_json();
    assert_eq!(out["total"], json!(3));
    let keys: Vec<&str> = out["documents"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["key"].as_str().unwrap())
        .collect();
    assert_eq!(keys, ["WF-1", "WF-2", "WF-3"]);
}

// ---------------------------------------------------------------------------------------------
// --limit, total and truncated (the ticket's own criterion)
// ---------------------------------------------------------------------------------------------

fn three_tickets() -> Scratch {
    let project = Scratch::project(&TICKETS);
    project.file("tickets/T-1.md", "---\ntitle: one\nstatus: open\n---\n");
    project.file("tickets/T-2.md", "---\ntitle: two\nstatus: open\n---\n");
    project.file("tickets/T-3.md", "---\ntitle: three\nstatus: open\n---\n");
    project
}

#[test]
fn truncated_is_true_exactly_when_total_is_larger_than_the_number_listed() {
    let project = three_tickets();

    let cut = list(project.path(), &["--limit", "2", "--json"]).stdout_json();
    assert_eq!(cut["total"], json!(3));
    assert_eq!(cut["documents"].as_array().unwrap().len(), 2);
    assert_eq!(cut["truncated"], json!(true));

    let exact = list(project.path(), &["--limit", "3", "--json"]).stdout_json();
    assert_eq!(exact["total"], json!(3));
    assert_eq!(exact["truncated"], json!(false));

    let more = list(project.path(), &["--limit", "10", "--json"]).stdout_json();
    assert_eq!(more["total"], json!(3));
    assert_eq!(more["truncated"], json!(false));

    let zero = list(project.path(), &["--limit", "0", "--json"]).stdout_json();
    assert_eq!(zero["total"], json!(3));
    assert_eq!(zero["documents"], json!([]));
    assert_eq!(zero["truncated"], json!(true));
}

#[test]
fn limit_does_not_change_total_or_the_json_fields_of_a_listed_document() {
    let project = three_tickets();

    let full = list(project.path(), &["--json"]).stdout_json();
    let cut = list(project.path(), &["--limit", "1", "--json"]).stdout_json();

    assert_eq!(cut["total"], full["total"]);
    assert_eq!(cut["documents"][0], full["documents"][0]);
}

#[test]
fn a_value_the_default_table_prints_does_not_change_with_limit() {
    // T-2's title is much longer than T-1's; with the extra `status` column after `title`,
    // `title` is no longer the table's last (unpadded) column, so a column-width computed from
    // only the *listed* rows would give T-1's row a different width under `--limit 1` than it
    // gets when every matching document (T-1 and T-2 both) is measured. Both share the same
    // `status` value, so the last column cannot itself be the source of any difference.
    let project = Scratch::project(&TICKETS);
    project.file("tickets/T-1.md", "---\ntitle: A\nstatus: open\n---\n");
    project.file(
        "tickets/T-2.md",
        "---\ntitle: AAAAAAAAAA\nstatus: open\n---\n",
    );

    let full = list(project.path(), &["--fields", "status"]);
    let cut = list(project.path(), &["--fields", "status", "--limit", "1"]);

    assert_eq!(full.code, 0, "stderr: {}", full.stderr);
    assert_eq!(cut.code, 0, "stderr: {}", cut.stderr);
    let full_lines: Vec<&str> = full.stdout.lines().collect();
    let cut_lines: Vec<&str> = cut.stdout.lines().collect();
    // The header (driven off the whole matched set, not the listed one) is identical either way.
    assert_eq!(
        cut_lines[0], full_lines[0],
        "header must not depend on --limit"
    );
    // T-1's own row (the first row under the header) must also be identical either way.
    let t1_line_of_full = full_lines[1];
    assert_eq!(cut_lines.get(1), Some(&t1_line_of_full));
    // And, concretely, `title` is padded to fit T-2's ten-character title (9 padding spaces
    // after T-1's one-character title, plus the 2-space column separator) even though `--limit
    // 1` never prints T-2 at all: a width taken from the listed rows alone would pad `A` to only
    // its own width, giving 2 spaces before `open`, not 11.
    let gap = t1_line_of_full.split("A").nth(1).unwrap();
    let spaces_before_open = gap.len() - gap.trim_start_matches(' ').len();
    assert_eq!(
        spaces_before_open, 11,
        "{t1_line_of_full:?} is not padded to T-2's title width"
    );
}

// ---------------------------------------------------------------------------------------------
// The default table and --fields
// ---------------------------------------------------------------------------------------------

#[test]
fn the_default_table_shows_identity_title_and_every_field_used_in_where() {
    let ran = list(
        &fixture("valid/refs"),
        &["--code", "WF", "--where", "context=*"],
    );

    assert_eq!(ran.code, 0, "stderr: {}", ran.stderr);
    // A header row names the key, the title and `context`, in that order, above the rows;
    // only WF-1 and WF-3 carry `context`, so each row must show the key, the title and the
    // value of `context`, in that same column order.
    let lines: Vec<&str> = ran.stdout.lines().collect();
    assert_eq!(lines.len(), 3, "{lines:?}");
    assert_eq!(
        lines[0].split_whitespace().collect::<Vec<_>>(),
        ["key", "title", "context"],
        "{lines:?}"
    );
    assert!(lines[1].starts_with("WF-1"), "{lines:?}");
    assert!(lines[1].contains("Ticket one"), "{lines:?}");
    assert!(lines[1].contains("chief::WF-5"), "{lines:?}");
    assert!(lines[2].starts_with("WF-3"), "{lines:?}");
    assert!(lines[2].contains("WF-2"), "{lines:?}");
}

#[test]
fn the_header_row_names_path_for_a_path_identified_collection() {
    let ran = list(&fixture("valid/refs"), &["--collection", "notes"]);

    assert_eq!(ran.code, 0, "stderr: {}", ran.stderr);
    let lines: Vec<&str> = ran.stdout.lines().collect();
    assert_eq!(lines.len(), 2, "{lines:?}");
    assert_eq!(
        lines[0].split_whitespace().collect::<Vec<_>>(),
        ["path", "title"],
        "{lines:?}"
    );
    assert!(lines[1].starts_with("notes/a.md"), "{lines:?}");
}

#[test]
fn the_header_row_names_key_when_the_matched_set_mixes_coded_and_path_identified_documents() {
    // No `--collection`/`--code`: `valid/refs` spans both `tickets` (coded) and `notes`
    // (path-identified). At least one matched document has a key, so the identity column is
    // labeled `key` for the whole table -- the same identity `table_row` already prints per row
    // (`doc.key.unwrap_or(doc.path)`) -- even though `notes/a.md` itself has none and still
    // prints its path in that column, unaffected by the header's label.
    let ran = list(&fixture("valid/refs"), &[]);

    assert_eq!(ran.code, 0, "stderr: {}", ran.stderr);
    let lines: Vec<&str> = ran.stdout.lines().collect();
    assert_eq!(lines.len(), 5, "{lines:?}");
    assert_eq!(
        lines[0].split_whitespace().collect::<Vec<_>>(),
        ["key", "title"],
        "{lines:?}"
    );
    assert!(
        lines.iter().any(|line| line.starts_with("notes/a.md")),
        "{lines:?}"
    );
}

#[test]
fn fields_overrides_the_where_derived_columns() {
    let ran = list(
        &fixture("valid/refs"),
        &[
            "--code",
            "WF",
            "--where",
            "context=*",
            "--fields",
            "collection",
        ],
    );

    assert_eq!(ran.code, 0, "stderr: {}", ran.stderr);
    let mut lines = ran.stdout.lines();
    let header = lines.next().unwrap();
    assert_eq!(
        header.split_whitespace().collect::<Vec<_>>(),
        ["key", "title", "collection"],
        "{header:?}"
    );
    for line in lines {
        assert!(line.contains("tickets"), "{line:?}");
    }
    assert!(!ran.stdout.contains("chief::WF-5"), "{}", ran.stdout);
}

#[test]
fn the_default_table_prints_nothing_for_an_empty_result_not_a_header_alone() {
    let ran = list(&fixture("valid/refs"), &["--where", "title=NoSuchTitle"]);

    assert_eq!(ran.code, 0, "stderr: {}", ran.stderr);
    assert_eq!(ran.stdout, "", "no header when there is nothing under it");
}

#[test]
fn limit_zero_leaves_no_header_with_zero_rows_under_it() {
    let project = three_tickets();

    let ran = list(project.path(), &["--limit", "0"]);

    assert_eq!(ran.code, 0, "stderr: {}", ran.stderr);
    assert_eq!(
        ran.stdout, "",
        "a header above zero listed rows is the same mistake as a header above an empty result"
    );
}

#[test]
fn ids_prints_one_key_or_path_per_line() {
    let ran = list(&fixture("valid/refs"), &["--collection", "notes", "--ids"]);

    assert_eq!(ran.code, 0, "stderr: {}", ran.stderr);
    assert_eq!(ran.stdout, "notes/a.md\n");
}

#[test]
fn ids_and_json_cannot_be_combined() {
    error_of(&list(&fixture("valid/refs"), &["--ids", "--json"]), 1);
}

// ---------------------------------------------------------------------------------------------
// The scope by namespace
// ---------------------------------------------------------------------------------------------

#[test]
fn namespace_narrows_the_scope() {
    let ran = list(
        &fixture("valid/several-namespaces"),
        &["--namespace", "story-*", "--json"],
    );

    assert_eq!(ran.code, 0, "stderr: {}", ran.stderr);
    // story-1: notes/a.md, tickets/WF-1.md (notes/deeper/c.md is too deep for the match, and is
    // not a document of this collection). story-2: notes/a.md, tickets/WF-1.md, tickets/WF-9.md.
    assert_eq!(ran.stdout_json()["total"], json!(5));

    let archive = list(
        &fixture("valid/several-namespaces"),
        &["--namespace", "archive", "--json"],
    );
    assert_eq!(archive.stdout_json()["total"], json!(1));
}

#[test]
fn list_is_no_longer_in_the_list_of_commands_the_binary_lacks() {
    use typdoc::registry;

    assert!(!registry::UNIMPLEMENTED_COMMANDS.contains(&"list"));
    assert!(registry::commands().contains(&"list".to_owned()));
}

// ---------------------------------------------------------------------------------------------
// `ref.*`/`refby.*` through the binary: parses, evaluates, and a dangling ref warns on stderr
// (design, Query, "Reached documents": "dangling refs also warn on stderr").
// ---------------------------------------------------------------------------------------------

#[test]
fn a_ref_star_condition_matches_through_the_binary() {
    // `fixtures/valid/ref-query`: T-2's blocked_by is empty, so ref.all(...).status=resolved is
    // vacuously true for it; T-3's only blocker is T-2, itself resolved.
    let ran = list(
        &fixture("valid/ref-query"),
        &[
            "--collection",
            "tickets",
            "--where",
            "ref.all(blocked_by).status=resolved",
            "--ids",
        ],
    );

    assert_eq!(ran.code, 0, "stderr: {}", ran.stderr);
    let ids: Vec<&str> = ran.stdout.lines().collect();
    assert!(ids.contains(&"T-2"), "{ids:?}");
    assert!(ids.contains(&"T-3"), "{ids:?}");
}

#[test]
fn a_dangling_ref_reached_by_ref_star_warns_on_stderr() {
    // T-1's blocked_by is [T-2, T-99]; T-99 does not exist, so evaluating ref.any(blocked_by)
    // for T-1 walks a dangling ref and must warn about it, even though the query itself succeeds
    // (exit 0) and the dangling ref simply counts as absent for the result.
    let ran = list(
        &fixture("valid/ref-query"),
        &[
            "--collection",
            "tickets",
            "--where",
            "ref.any(blocked_by).status!=resolved",
            "--json",
        ],
    );

    assert_eq!(
        ran.code, 0,
        "stderr should carry a warning, not an error: {}",
        ran.stderr
    );
    assert!(
        ran.stderr.contains("T-99"),
        "expected a dangling-ref warning naming T-99, got {:?}",
        ran.stderr
    );
    assert!(
        ran.stdout_json()["documents"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["path"] == json!("tickets/T-1.md")),
        "T-1 still matches: its dangling blocker counts as absent, which satisfies !=resolved"
    );
}

#[test]
fn a_query_with_no_dangling_ref_prints_nothing_on_stderr() {
    // Narrowed to T-3 alone (key=T-3): its only blocker (T-2) resolves cleanly, so evaluating
    // ref.any(blocked_by) for it never reaches T-1's or T-5's dangling T-99, and stderr stays
    // empty even though the very same field, elsewhere in this project, does have one.
    let ran = list(
        &fixture("valid/ref-query"),
        &[
            "--collection",
            "tickets",
            "--where",
            "key=T-3",
            "--where",
            "ref.any(blocked_by)",
            "--json",
        ],
    );

    assert_eq!(ran.code, 0, "stderr: {}", ran.stderr);
    assert_eq!(ran.stderr, "");
    assert_eq!(ran.stdout_json()["total"], json!(1));
}
