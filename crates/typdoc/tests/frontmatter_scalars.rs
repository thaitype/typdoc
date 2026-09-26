//! Covers SPC-4, SPC-12, SPC-13.
//!
//! Every expected value is written out by hand, never copied from the tool's output.

#[allow(dead_code, reason = "each test file uses part of the shared helper")]
mod common;

use common::{Ran, Scratch, Spawn};
use serde_json::{Value, json};

const SCHEMA: &str = r#"{ "name": "n", "fields": {
    "s": { "type": "string" },
    "num": { "type": "number" },
    "l": { "type": "list" }
} }"#;

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

fn fields_of(project: &Scratch) -> Value {
    let ran = run(project, &["get", "d.md", "--json"]);
    assert_eq!(ran.code, 0, "stderr: {}", ran.stderr);
    ran.stdout_json()["document"]["fields"].clone()
}

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

#[test]
fn an_integer_past_64_bits_in_a_number_field_keeps_its_digits_and_converts_to_the_nearest_float() {
    for (written, nearest) in [
        ("123456789012345678901", 1.2345678901234568e20),
        ("-9223372036854775809", -9.223372036854776e18),
        ("18446744073709551616", 1.8446744073709552e19),
    ] {
        let project = project_with(&format!("num: {written}"));

        assert_eq!(printed_num(&project), written);
        let read = fields_of(&project)["num"].as_f64().unwrap();
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

/// The text `get --json` prints for the field `num`, read out of the bytes of the output. A
/// JSON reader of its own would convert the number on the way in, which is the very step under
/// test here, so the output is not parsed before it is looked at.
fn printed_num(project: &Scratch) -> String {
    let ran = run(project, &["get", "d.md", "--json"]);
    assert_eq!(ran.code, 0, "stderr: {}", ran.stderr);
    let key = "\"num\":";
    let at = ran
        .stdout
        .find(key)
        .unwrap_or_else(|| panic!("no field `num` in {}", ran.stdout));
    let rest = &ran.stdout[at + key.len()..];
    let end = rest
        .find([',', '}'])
        .unwrap_or_else(|| panic!("the value of `num` does not end in {}", ran.stdout));
    rest[..end].to_owned()
}

#[test]
fn a_number_is_printed_with_the_digits_the_document_holds() {
    assert!(
        !typdoc::registry::KNOWN_GAPS
            .iter()
            .any(|gap| gap.starts_with("[number-text]")),
        "the gap is closed, so `registry::KNOWN_GAPS` must not list it any longer"
    );

    for written in [
        // Inside the range an integer holds: 2^53 + 1, i64's minimum, u64's maximum.
        "3",
        "9007199254740993",
        "-9223372036854775808",
        "18446744073709551615",
        // Outside it.
        "18446744073709551616",
        "99999999999999999999",
        "99999999999999999998",
        "12345678901234567890123",
        // Inside it in size, but a conversion would change the form: ordinary documents, not
        // extreme ones.
        "1e3",
        "1.10",
        "-2.5e-3",
    ] {
        let project = project_with(&format!("num: {written}"));
        assert_eq!(printed_num(&project), written, "num: {written}");
    }

    // Two numbers that differ in their last digit print differently; converted, both would
    // print `1e+20`.
    assert_ne!(
        printed_num(&project_with("num: 99999999999999999999")),
        printed_num(&project_with("num: 99999999999999999998"))
    );

    let as_text = project_with("s: \"99999999999999999999\"");
    assert_eq!(fields_of(&as_text)["s"], json!("99999999999999999999"));
}

/// Printing keeps the digits; comparing uses the converted value.
#[test]
fn two_numbers_that_differ_past_a_primitive_still_compare_equal() {
    let project = Scratch::project(&[]);
    project.file(
        ".typdoc/collections/notes.json",
        r#"{ "match": "*.md", "schema": "n.json" }"#,
    );
    project.file("n.json", SCHEMA);
    project.file("above.md", "---\nnum: 99999999999999999999\n---\n\nbody\n");
    project.file("below.md", "---\nnum: 99999999999999999998\n---\n\nbody\n");

    // The two print apart, so the difference between the documents is there to be seen.
    let listed = run(&project, &["list", "--json"]);
    assert_eq!(listed.code, 0, "stderr: {}", listed.stderr);
    assert!(
        listed.stdout.contains("99999999999999999999")
            && listed.stdout.contains("99999999999999999998"),
        "{}",
        listed.stdout
    );

    // They still compare together: a `>` on the lower of the two finds neither document.
    let greater = run(
        &project,
        &["list", "--where", "num>99999999999999999998", "--ids"],
    );
    assert_eq!(greater.code, 0, "stderr: {}", greater.stderr);
    assert_eq!(greater.stdout, "");

    // The other side of the same fact: a `>=` on the higher of the two finds both, because
    // the lower one is not below it.
    let at_least = run(
        &project,
        &["list", "--where", "num>=99999999999999999999", "--ids"],
    );
    assert_eq!(at_least.code, 0, "stderr: {}", at_least.stderr);
    let mut found: Vec<&str> = at_least.stdout.lines().collect();
    found.sort_unstable();
    assert_eq!(found, ["above.md", "below.md"]);

    // `=` reads the value a number converts to, not its digits, so both documents match the
    // one value they share and neither is found by its own digits.
    let converted = run(&project, &["list", "--where", "num=1e+20", "--ids"]);
    assert_eq!(converted.code, 0, "stderr: {}", converted.stderr);
    let mut matched: Vec<&str> = converted.stdout.lines().collect();
    matched.sort_unstable();
    assert_eq!(matched, ["above.md", "below.md"]);

    // The same rule in the ordinary case: `1e3` is found by the value it converts to.
    let ratio = project_with("num: 1e3");
    let by_converted = run(&ratio, &["list", "--where", "num=1000.0", "--ids"]);
    assert_eq!(by_converted.code, 0, "stderr: {}", by_converted.stderr);
    assert_eq!(by_converted.stdout, "d.md\n");
}

#[test]
fn a_field_written_with_no_value_is_kept_apart_from_one_written_as_an_empty_string() {
    assert!(
        !typdoc::registry::KNOWN_GAPS
            .iter()
            .any(|gap| gap.starts_with("[empty-value]")),
        "the gap is closed, so `registry::KNOWN_GAPS` must not list it any longer"
    );

    let bare = project_with("s:");
    let quoted = project_with("s: \"\"");

    // Told apart: the first is `null`, the second is `""`.
    assert_ne!(fields_of(&bare)["s"], fields_of(&quoted)["s"]);
    assert_eq!(fields_of(&bare)["s"], Value::Null);
    assert_eq!(fields_of(&quoted)["s"], json!(""));

    // Present rather than missing, and reported like a written value where its type does not
    // fit.
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
