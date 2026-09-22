//! A frontmatter value that is one scalar keeps the text written in the file: an integer of any
//! length and a value with a tag are read like every other scalar, so the document is listed,
//! `get` shows the text, and `validate` finds nothing to parse. Every project is built in a
//! scratch folder, and every expected value is written out by hand from the design (Documents:
//! no reader decides what `1e3`, `no` or `2026-09-19` is).

#[allow(dead_code, reason = "each test file uses part of the shared helper")]
mod common;

use common::{Ran, Scratch, Spawn};
use serde_json::{Value, json};

const SCHEMA: &str = r#"{ "name": "n", "fields": {
    "s": { "type": "string" },
    "num": { "type": "number" },
    "l": { "type": "list" }
} }"#;

/// A project with one collection of `*.md` and one document `d.md` whose block is `block`.
fn project_with(block: &str) -> Scratch {
    let project = Scratch::project(&[]);
    project.file(
        ".typdoc/collections/notes.json",
        r#"{ "match": "*.md", "schema": "n.json" }"#,
    );
    project.file("n.json", SCHEMA);
    project.file("d.md", &format!("---\n{block}\n---\n\nbody\n"));
    project
}

fn run(project: &Scratch, args: &[&str]) -> Ran {
    Spawn::args(args.iter().copied()).cwd(project.path()).run()
}

/// What `get d.md --json` shows as `fields`; the run must have gone well.
fn fields_of(project: &Scratch) -> Value {
    let ran = run(project, &["get", "d.md", "--json"]);
    assert_eq!(ran.code, 0, "stderr: {}", ran.stderr);
    ran.stdout_json()["document"]["fields"].clone()
}

/// The document is listed and `validate` does not report `frontmatter.parse` for it.
fn is_listed_and_parses(project: &Scratch, block: &str) {
    let listed = run(project, &["list", "--ids"]);
    assert_eq!(listed.code, 0, "{block}: {}", listed.stderr);
    assert_eq!(listed.stdout, "d.md\n", "{block}");

    let validated = run(project, &["validate", "--json"]);
    let out = validated.stdout_json();
    let rules: Vec<&str> = out["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["rule"].as_str().unwrap())
        .collect();
    assert!(!rules.contains(&"frontmatter.parse"), "{block}: {rules:?}");
}

/// The document is written as `block` and it is read as `expected`, listed, and not reported by
/// `frontmatter.parse`.
fn reads_as(block: &str, expected: Value) {
    let project = project_with(block);

    assert_eq!(fields_of(&project), expected, "{block}");
    is_listed_and_parses(&project, block);
}

#[test]
fn an_integer_past_u64_in_a_string_field_keeps_its_digits() {
    reads_as(
        "s: 123456789012345678901",
        json!({ "s": "123456789012345678901" }),
    );
    reads_as(
        "s: 18446744073709551616",
        json!({ "s": "18446744073709551616" }),
    );
}

#[test]
fn an_integer_below_i64_min_in_a_string_field_keeps_its_digits() {
    reads_as(
        "s: -9223372036854775809",
        json!({ "s": "-9223372036854775809" }),
    );
}

#[test]
fn an_integer_past_64_bits_in_a_list_keeps_its_digits() {
    reads_as(
        "l: [1, 123456789012345678901]",
        json!({ "l": ["1", "123456789012345678901"] }),
    );
    reads_as(
        "l: [-9223372036854775809, 2]",
        json!({ "l": ["-9223372036854775809", "2"] }),
    );
}

/// A `number` is a JSON number and no wider: a value past `u64` or below `i64::MIN` is a float
/// that holds the value to the precision a float has (the last digit or two are the reader's),
/// and it is not a finding.
#[test]
fn an_integer_past_64_bits_in_a_number_field_is_a_float() {
    for (written, nearest) in [
        ("123456789012345678901", 1.2345678901234568e20),
        ("-9223372036854775809", -9.223372036854776e18),
        ("18446744073709551616", 1.8446744073709552e19),
    ] {
        let project = project_with(&format!("num: {written}"));

        let fields = fields_of(&project);

        assert!(fields["num"].is_f64(), "{written}: {fields}");
        let read = fields["num"].as_f64().unwrap();
        assert!(
            ((read - nearest) / nearest).abs() < 1e-15,
            "{written}: {read}"
        );
        is_listed_and_parses(&project, written);
        let out = run(&project, &["validate", "--json"]);
        assert_eq!(out.code, 0, "{written}: {}", out.stdout);
        assert_eq!(out.stdout_json()["findings"], json!([]), "{written}");
    }
}

#[test]
fn u64_max_in_a_number_field_stays_an_exact_integer() {
    let project = project_with("num: 18446744073709551615");

    let fields = fields_of(&project);

    assert_eq!(fields["num"].as_u64(), Some(u64::MAX));
}

#[test]
fn a_tagged_scalar_in_a_string_field_is_its_value_without_the_tag() {
    reads_as("s: !custom v", json!({ "s": "v" }));
    reads_as("s: !Ref x", json!({ "s": "x" }));
    reads_as("s: !Ref Thing", json!({ "s": "Thing" }));
}

#[test]
fn a_tagged_scalar_in_a_list_is_its_value_without_the_tag() {
    reads_as("l: [!custom v, w]", json!({ "l": ["v", "w"] }));
    reads_as("l: [a, !Ref x]", json!({ "l": ["a", "x"] }));
    reads_as("l: [!a 1, b]", json!({ "l": ["1", "b"] }));
    reads_as("l:\n  - !custom v\n  - !Ref x", json!({ "l": ["v", "x"] }));
}

#[test]
fn a_tagged_value_in_a_number_field_is_read_by_the_field_type() {
    reads_as("num: !custom 5", json!({ "num": 5 }));
}

#[test]
fn a_custom_tag_on_a_list_of_scalars_leaves_a_list_of_scalars() {
    reads_as("l: !custom [a, b]", json!({ "l": ["a", "b"] }));
}

#[test]
fn a_tag_on_a_mapping_is_refused() {
    for block in ["s: !custom { a: 1 }", "s: !custom\n  a: 1"] {
        let project = project_with(block);
        let ran = run(&project, &["get", "d.md", "--json"]);
        assert_eq!(ran.code, 2, "{block}: {}", ran.stdout);
        assert!(
            ran.stderr
                .contains("field `s` is a mapping, which has no text form"),
            "{block}: {}",
            ran.stderr
        );
    }
}

#[test]
fn a_tagged_list_that_holds_a_mapping_is_refused() {
    let project = project_with("l: !custom [a, { b: 1 }]");

    let ran = run(&project, &["get", "d.md", "--json"]);

    assert_eq!(ran.code, 2, "{}", ran.stdout);
    assert!(
        ran.stderr.contains(
            "field `l` is a list that holds something other than text, which has no text form"
        ),
        "{}",
        ran.stderr
    );
}

/// Every other scalar keeps its text, tagged or not.
#[test]
fn every_other_scalar_keeps_its_text() {
    reads_as(
        "s: 18446744073709551615",
        json!({ "s": "18446744073709551615" }),
    );
    reads_as(
        "s: -9223372036854775808",
        json!({ "s": "-9223372036854775808" }),
    );
    reads_as("s: !!str 5", json!({ "s": "5" }));
    reads_as("s: !!int 5", json!({ "s": "5" }));
    reads_as("s: !!binary aGVsbG8=", json!({ "s": "aGVsbG8=" }));
    reads_as("s: !!float 1", json!({ "s": "1" }));
    reads_as("s: !!timestamp 2026-09-19", json!({ "s": "2026-09-19" }));
    reads_as("s: !<tag:yaml.org,2002:str> x", json!({ "s": "x" }));
    reads_as("l: !!seq [a, b]", json!({ "l": ["a", "b"] }));
    for written in ["1.10", "01234", "0x1F", "1e3", "yes", "~", "สวัสดี"] {
        reads_as(&format!("s: {written}"), json!({ "s": written }));
    }
}

// ---------------------------------------------------------------------------------------------
// The binary converts a `number` out of its text before printing it, where the design says the
// digits written in the document are printed. This test pins what it does today, so that making
// it right turns the test red rather than leaving the gap list stale.
// ---------------------------------------------------------------------------------------------

#[test]
fn a_number_outside_the_integer_range_is_printed_converted_not_as_written() {
    assert!(
        typdoc::registry::KNOWN_GAPS
            .iter()
            .any(|gap| gap.starts_with("[number-text]")),
        "this test pins a gap that `registry::KNOWN_GAPS` no longer lists"
    );

    // Two values one apart, both past what an integer holds, and one the design names directly.
    let above = project_with("num: 99999999999999999999");
    let below = project_with("num: 99999999999999999998");
    let written = project_with("num: 1e3");

    // Today: converted, so the digits are gone and the two are indistinguishable.
    assert_eq!(fields_of(&above)["num"], fields_of(&below)["num"]);
    assert_eq!(fields_of(&above)["num"], json!(1e20));
    assert_eq!(fields_of(&written)["num"], json!(1000.0));

    // What the design asks for, for whoever closes the gap: the digits as the file has them.
    // `assert_ne` rather than a comment, so this half also fails once the behaviour changes.
    assert_ne!(fields_of(&above)["num"].to_string(), "99999999999999999999");
    assert_ne!(fields_of(&written)["num"].to_string(), "1e3");

    // A `string` holding the same digits was never affected, which is what locates the loss.
    let as_text = project_with("s: \"99999999999999999999\"");
    assert_eq!(fields_of(&as_text)["s"], json!("99999999999999999999"));
}

// ---------------------------------------------------------------------------------------------
// A field written with no value and one written as an empty string are two different YAML values,
// and the design says each keeps the form it was written in, with the first shown as `null`. The
// binary reads both as the same empty text. This pins that, so closing the gap turns it red.
// ---------------------------------------------------------------------------------------------

#[test]
fn a_field_written_with_no_value_reads_the_same_as_an_empty_string() {
    assert!(
        typdoc::registry::KNOWN_GAPS
            .iter()
            .any(|gap| gap.starts_with("[empty-value]")),
        "this test pins a gap that `registry::KNOWN_GAPS` no longer lists"
    );

    let bare = project_with("s:");
    let quoted = project_with("s: \"\"");

    // Today: indistinguishable, both the empty string.
    assert_eq!(fields_of(&bare)["s"], fields_of(&quoted)["s"]);
    assert_eq!(fields_of(&bare)["s"], json!(""));

    // What the design asks for: the first is `null`, the second is `""`.
    assert_ne!(fields_of(&bare)["s"], Value::Null);

    // Unchanged by the decision, and asserted here so that closing the gap cannot quietly change
    // it: a field written with no value is present rather than missing, and one whose type it
    // does not fit is reported the same way a written value would be.
    let ran = run(&bare, &["validate", "--json"]);
    assert_eq!(ran.code, 0, "stderr: {}", ran.stderr);
    assert_eq!(ran.stdout_json()["findings"], json!([]));

    let number = project_with("num:");
    let ran = run(&number, &["validate", "--json"]);
    assert_eq!(ran.code, 2, "stderr: {}", ran.stderr);
    assert_eq!(
        ran.stdout_json()["findings"][0]["rule"],
        json!("frontmatter.types")
    );
}
