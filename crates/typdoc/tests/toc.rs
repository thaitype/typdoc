//! Covers SPC-1, SPC-2, SPC-5, SPC-12, SPC-14.

#[allow(dead_code, reason = "each test file uses part of the shared helper")]
mod common;

use common::{NOTES, Ran, Scratch, Spawn, fixture};
use serde_json::{Value, json};

fn toc(project: &std::path::Path, args: &[&str]) -> Ran {
    Spawn::args(["toc"].into_iter().chain(args.iter().copied()))
        .cwd(project)
        .run()
}

fn headings_of(ran: &Ran) -> Vec<(u64, String, String, u64, u64)> {
    assert_eq!(ran.code, 0, "stderr: {}", ran.stderr);
    assert_eq!(ran.stderr, "");
    ran.stdout_json()["headings"]
        .as_array()
        .expect("headings is an array")
        .iter()
        .map(|h| {
            (
                h["level"].as_u64().unwrap(),
                h["text"].as_str().unwrap().to_owned(),
                h["slug"].as_str().unwrap().to_owned(),
                h["line"].as_u64().unwrap(),
                h["end"].as_u64().unwrap(),
            )
        })
        .collect()
}

fn row(level: u64, text: &str, slug: &str, line: u64, end: u64) -> (u64, String, String, u64, u64) {
    (level, text.to_owned(), slug.to_owned(), line, end)
}

fn error_of(ran: &Ran, code: i32) -> Value {
    assert_eq!(ran.code, code, "stderr: {}", ran.stderr);
    assert_eq!(ran.stdout, "", "a failure prints nothing on stdout");
    let object = ran.stderr_json();
    assert_eq!(object["code"], json!(code));
    object
}

#[test]
fn toc_prints_the_document_it_was_asked_about_and_its_headings() {
    let ran = toc(&fixture("valid/minimal"), &["note.md", "--json"]);

    assert_eq!(ran.code, 0, "stderr: {}", ran.stderr);
    assert_eq!(ran.stderr, "");
    assert_eq!(
        ran.stdout_json(),
        json!({
            "document": { "path": "note.md", "namespace": "default" },
            "headings": [
                { "level": 1, "text": "A minimal note", "slug": "a-minimal-note", "line": 6, "end": 8 }
            ]
        })
    );
}

#[test]
fn a_document_with_no_heading_has_an_empty_list() {
    let project = Scratch::project(&NOTES);
    project.file("plain.md", "---\ntitle: x\n---\n\njust text\n");

    let ran = toc(project.path(), &["plain.md", "--json"]);

    assert_eq!(ran.code, 0, "stderr: {}", ran.stderr);
    assert_eq!(ran.stdout_json()["headings"], json!([]));
}

#[test]
fn a_document_of_another_namespace_names_its_namespace() {
    let ran = toc(
        &fixture("valid/several-namespaces"),
        &["story-2/notes/a.md", "--json"],
    );

    assert_eq!(ran.code, 0, "stderr: {}", ran.stderr);
    assert_eq!(
        ran.stdout_json()["document"],
        json!({ "path": "story-2/notes/a.md", "namespace": "story-2" })
    );
}

#[test]
fn a_coded_document_carries_its_key_and_a_key_argument_reads_the_same_document() {
    let project = fixture("valid/templates");

    let by_path = toc(&project, &["tickets/WF-1.md", "--json"]);
    let by_key = toc(&project, &["WF-1", "--json"]);

    for ran in [&by_path, &by_key] {
        assert_eq!(ran.code, 0, "stderr: {}", ran.stderr);
        assert_eq!(
            ran.stdout_json()["document"],
            json!({ "path": "tickets/WF-1.md", "namespace": "default", "key": "WF-1" })
        );
    }
}

#[test]
fn depth_chooses_which_headings_are_listed_and_never_changes_an_end() {
    let project = fixture("valid/body");

    let all = headings_of(&toc(&project, &["nothing-under.md", "--json"]));
    let two = headings_of(&toc(
        &project,
        &["nothing-under.md", "--depth", "2", "--json"],
    ));
    let one = headings_of(&toc(
        &project,
        &["nothing-under.md", "--depth", "1", "--json"],
    ));

    assert_eq!(
        all,
        [
            row(1, "Top", "top", 1, 7),
            row(2, "Empty", "empty", 3, 4),
            row(3, "Deeper", "deeper", 4, 4),
            row(2, "Sibling", "sibling", 5, 7),
        ]
    );
    assert_eq!(
        two,
        [
            row(1, "Top", "top", 1, 7),
            row(2, "Empty", "empty", 3, 4),
            row(2, "Sibling", "sibling", 5, 7),
        ]
    );
    assert_eq!(one, [row(1, "Top", "top", 1, 7)]);
}

#[test]
fn a_depth_that_is_not_a_level_exits_1() {
    for depth in ["0", "-1", "x", ""] {
        let ran = toc(
            &fixture("valid/body"),
            &["thai.md", "--depth", depth, "--json"],
        );

        error_of(&ran, 1);
    }
}

#[test]
fn headings_come_in_the_order_of_their_lines() {
    let project = Scratch::project(&NOTES);
    project.file(
        "mixed.md",
        "# A\n\n> ## B\n\n- ### C\n\nD\n---\n\n```\n# not a heading\n```\n\n## E\n",
    );

    let found = headings_of(&toc(project.path(), &["mixed.md", "--json"]));

    let lines: Vec<u64> = found.iter().map(|h| h.3).collect();
    assert_eq!(lines, [1, 3, 5, 7, 14]);
    assert!(lines.is_sorted());
    assert_eq!(
        found.iter().map(|h| h.1.as_str()).collect::<Vec<_>>(),
        ["A", "B", "C", "D", "E"]
    );
}

#[test]
fn lines_count_from_the_top_of_the_file_with_the_frontmatter_in() {
    let ran = toc(&fixture("valid/body"), &["thai.md", "--json"]);

    assert_eq!(headings_of(&ran), [row(1, "หัวข้อ", "หัวข้อ", 5, 7)]);
}

#[test]
fn a_file_with_no_final_line_ending_ends_on_its_last_line() {
    let ran = toc(&fixture("valid/body"), &["no-final-newline.md", "--json"]);

    assert_eq!(
        headings_of(&ran),
        [
            row(1, "First", "first", 1, 7),
            row(2, "Second", "second", 5, 7)
        ]
    );
}

#[test]
fn carriage_return_and_line_feed_endings_count_one_line_each() {
    let ran = toc(&fixture("valid/body"), &["crlf.md", "--json"]);

    assert_eq!(
        headings_of(&ran),
        [row(1, "One", "one", 1, 7), row(2, "Two", "two", 5, 7)]
    );
}

#[test]
fn a_lone_carriage_return_ends_a_line() {
    let ran = toc(&fixture("valid/body"), &["lone-cr.md", "--json"]);

    assert_eq!(
        headings_of(&ran),
        [row(1, "One", "one", 1, 7), row(2, "Two", "two", 5, 7)]
    );
}

#[test]
fn a_heading_with_nothing_under_it_ends_on_its_own_line() {
    let ran = toc(&fixture("valid/body"), &["nothing-under.md", "--json"]);

    let found = headings_of(&ran);
    let deeper = found.iter().find(|h| h.1 == "Deeper").unwrap();
    assert_eq!((deeper.3, deeper.4), (4, 4));
}

#[test]
fn a_heading_that_is_not_the_first_in_a_file_with_frontmatter_and_carriage_returns_is_placed() {
    let project = Scratch::project(&NOTES);
    project.file("cr.md", "---\rtitle: x\r---\r\r# One\r\r## Two\r");

    let found = headings_of(&toc(project.path(), &["cr.md", "--json"]));

    assert_eq!(
        found,
        [row(1, "One", "one", 5, 7), row(2, "Two", "two", 7, 7)]
    );
}

#[test]
fn a_document_that_is_not_there_exits_5() {
    let ran = toc(&fixture("valid/body"), &["absent.md", "--json"]);

    error_of(&ran, 5);
}

#[test]
fn a_frontmatter_block_that_is_never_closed_exits_2() {
    let project = Scratch::project(&NOTES);
    project.file("open.md", "---\ntitle: never closed\n\n# Heading\n");

    let ran = toc(project.path(), &["open.md", "--json"]);

    let object = error_of(&ran, 2);
    assert!(
        object["error"].as_str().unwrap().contains("open.md"),
        "{object}"
    );
}

#[test]
fn a_block_that_is_closed_is_cut_without_being_read_as_yaml() {
    let project = Scratch::project(&NOTES);
    project.file("odd.md", "---\n: : not yaml [\n---\n# Still a heading\n");

    let ran = toc(project.path(), &["odd.md", "--json"]);

    assert_eq!(
        headings_of(&ran),
        [row(1, "Still a heading", "still-a-heading", 4, 4)]
    );
}

#[test]
fn an_argument_that_is_not_a_path_exits_1() {
    let ran = toc(&fixture("valid/body"), &["thai", "--json"]);

    error_of(&ran, 1);
}

#[test]
fn a_call_with_no_document_exits_1() {
    let ran = toc(&fixture("valid/body"), &["--json"]);

    error_of(&ran, 1);
}

#[test]
fn without_json_prints_a_header_rowed_table_one_row_per_heading() {
    let ran = toc(&fixture("valid/body"), &["nothing-under.md"]);

    assert_eq!(ran.code, 0, "stderr: {}", ran.stderr);
    assert_eq!(ran.stderr, "");
    assert_eq!(
        ran.stdout,
        "\
line  end  level  heading
1     7    1      Top
3     4    2      Empty
4     4    3      Deeper
5     7    2      Sibling
"
    );
}

#[test]
fn a_document_with_no_headings_prints_nothing_without_json() {
    let project = Scratch::project(&NOTES);
    project.file("plain.md", "---\ntitle: x\n---\n\njust text\n");

    let ran = toc(project.path(), &["plain.md"]);

    assert_eq!(ran.code, 0, "stderr: {}", ran.stderr);
    assert_eq!(ran.stdout, "");
    assert_eq!(ran.stderr, "");
}

/// Both headings are level 2, so `--depth 1` keeps neither.
#[test]
fn depth_filtered_to_nothing_says_so_on_stderr_distinct_from_no_headings_at_all() {
    let project = Scratch::project(&NOTES);
    project.file(
        "two-headings.md",
        "---\ntitle: x\n---\n\n## First\n\ntext\n\n## Second\n\nmore\n",
    );

    let ran = toc(project.path(), &["two-headings.md", "--depth", "1"]);

    assert_eq!(ran.code, 0, "stderr: {}", ran.stderr);
    assert_eq!(ran.stdout, "");
    assert_eq!(
        ran.stderr,
        "no headings at depth \u{2264} 1 (2 headings are deeper)\n"
    );
}

#[test]
fn depth_filtered_to_nothing_uses_the_singular_form_for_one_heading() {
    let project = Scratch::project(&NOTES);
    project.file(
        "one-heading.md",
        "---\ntitle: x\n---\n\n## Only one\n\ntext\n",
    );

    let ran = toc(project.path(), &["one-heading.md", "--depth", "1"]);

    assert_eq!(ran.code, 0, "stderr: {}", ran.stderr);
    assert_eq!(ran.stdout, "");
    assert_eq!(
        ran.stderr,
        "no headings at depth \u{2264} 1 (1 heading is deeper)\n"
    );
}

#[test]
fn an_error_without_json_prints_plain_text_not_the_json_object() {
    let ran = toc(&fixture("valid/body"), &["absent.md"]);

    assert_eq!(ran.code, 5, "stderr: {}", ran.stderr);
    assert_eq!(ran.stdout, "");
    assert!(
        ran.stderr.starts_with("typdoc: "),
        "expected plain text, got {:?}",
        ran.stderr
    );
    assert!(
        serde_json::from_str::<Value>(&ran.stderr).is_err(),
        "stderr should not be the --json error object: {:?}",
        ran.stderr
    );
}
