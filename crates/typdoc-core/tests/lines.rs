//! Lines and columns as the design counts them: a line ends at a line feed, at a carriage
//! return and a line feed together, or at a carriage return alone; a line ending at the end
//! of the file begins no other line; a column counts Unicode scalar values.

use serde::Deserialize;
use typdoc_core::{LineMap, Position};
use typdoc_testkit::fixtures;

fn at(text: &str, needle: &str) -> Position {
    let offset = text
        .find(needle)
        .unwrap_or_else(|| panic!("{needle:?} is not in {text:?}"));
    LineMap::new(text).position(offset)
}

fn pos(line: usize, col: usize) -> Position {
    Position { line, col }
}

#[test]
fn a_line_ending_at_the_end_of_the_file_begins_no_other_line() {
    assert_eq!(LineMap::new("a\nb").line_count(), 2);
    assert_eq!(LineMap::new("a\nb\n").line_count(), 2);
    assert_eq!(LineMap::new("a\nb\n\n").line_count(), 3);
}

#[test]
fn an_empty_text_has_no_line() {
    assert_eq!(LineMap::new("").line_count(), 0);
}

#[test]
fn a_line_feed_a_carriage_return_and_the_two_together_each_end_one_line() {
    assert_eq!(LineMap::new("a\nb\r\nc\rd").line_count(), 4);
    assert_eq!(at("a\nb\r\nc\rd", "b"), pos(2, 1));
    assert_eq!(at("a\nb\r\nc\rd", "c"), pos(3, 1));
    assert_eq!(at("a\nb\r\nc\rd", "d"), pos(4, 1));
}

#[test]
fn two_carriage_returns_in_a_row_are_two_endings_and_not_one_pair() {
    assert_eq!(LineMap::new("a\r\rb").line_count(), 3);
    assert_eq!(at("a\r\rb", "b"), pos(3, 1));
}

#[test]
fn a_carriage_return_before_a_line_feed_is_one_ending_however_it_is_reached() {
    assert_eq!(LineMap::new("a\r\n\r\nb").line_count(), 3);
    assert_eq!(at("a\r\n\r\nb", "b"), pos(3, 1));
}

#[test]
fn a_tab_a_thai_mark_and_an_emoji_each_count_as_one_column() {
    assert_eq!(at("\tx", "x"), pos(1, 2));
    // ก, ิ and ้ are a consonant, a vowel mark and a tone mark.
    assert_eq!(at("กิ้x", "x"), pos(1, 4));
    // An emoji is one scalar value, where UTF-16 counts two.
    assert_eq!(at("🎉x", "x"), pos(1, 2));
}

#[test]
fn a_position_on_the_line_ending_belongs_to_that_line() {
    let text = "ab\r\ncd";
    let map = LineMap::new(text);

    assert_eq!(map.position(2), pos(1, 3));
    assert_eq!(map.position(4), pos(2, 1));
}

#[derive(Deserialize)]
struct Expected {
    file: String,
    at: String,
    line: usize,
    col: usize,
}

/// Each expectation of `fixtures/valid/body/positions.json` was counted by hand.
#[test]
fn the_positions_written_in_the_fixtures_are_what_the_map_gives() {
    let dir = fixtures::path("valid/body");
    let list = std::fs::read_to_string(dir.join("positions.json")).unwrap();
    let expected: Vec<Expected> = serde_json::from_str(&list).unwrap();
    assert!(!expected.is_empty());

    let mut problems = Vec::new();
    for case in expected {
        let text = std::fs::read_to_string(dir.join(&case.file)).unwrap();
        let got = at(&text, &case.at);
        if got != pos(case.line, case.col) {
            problems.push(format!(
                "{} at {:?}: expected {}:{}, got {}:{}",
                case.file, case.at, case.line, case.col, got.line, got.col
            ));
        }
    }

    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn the_fixtures_keep_the_line_endings_they_were_written_with() {
    let dir = fixtures::path("valid/body");
    let crlf = std::fs::read(dir.join("crlf.md")).unwrap();
    let lone = std::fs::read(dir.join("lone-cr.md")).unwrap();
    let bare = std::fs::read(dir.join("no-final-newline.md")).unwrap();

    assert!(crlf.windows(2).any(|pair| pair == b"\r\n"));
    assert!(!lone.contains(&b'\n') && lone.contains(&b'\r'));
    assert_ne!(bare.last(), Some(&b'\n'));
}
