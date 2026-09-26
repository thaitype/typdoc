//! Covers SPC-1.
//!
//! `typdoc_core::mentions`, pure over one document's text.

use typdoc_core::{Mention, mentions};

fn of(text: &str, inline_code: bool, fenced_code: bool) -> Vec<Mention> {
    mentions(text, inline_code, fenced_code).unwrap()
}

fn m(written: &str, line: usize, col: usize) -> Mention {
    Mention {
        written: written.to_owned(),
        line,
        col,
    }
}

#[test]
fn a_bare_mention_in_prose_is_found_at_its_start() {
    let found = of("see WF-3 for details\n", true, false);

    assert_eq!(found, [m("WF-3", 1, 5)]);
}

#[test]
fn a_sibling_prefixed_mention_is_found_whole() {
    let found = of("see story-2:WF-5 please\n", true, false);

    assert_eq!(found, [m("story-2:WF-5", 1, 5)]);
}

#[test]
fn an_import_prefixed_mention_is_found_whole() {
    let found = of("see memory::LRN-5 please\n", true, false);

    assert_eq!(found, [m("memory::LRN-5", 1, 5)]);
}

#[test]
fn a_trailing_letter_breaks_the_key_shape() {
    let found = of("WF-3a is not a mention\n", true, false);

    assert_eq!(found, []);
}

#[test]
fn a_leading_letter_breaks_the_key_shape() {
    let found = of("xWF-3 is not a mention\n", true, false);

    assert_eq!(found, []);
}

#[test]
fn a_code_shaped_word_that_is_not_a_key_still_has_the_shape() {
    // The project decides which codes are real; this module only reports the shape.
    let found = of("encoded as UTF-8\n", true, false);

    assert_eq!(found, [m("UTF-8", 1, 12)]);
}

#[test]
fn a_mention_inside_a_real_links_text_is_not_reported() {
    let found = of("[WF-3](WF-3.md)\n", true, false);

    assert_eq!(found, [], "body.links checks it, not body.mentions");
}

#[test]
fn inline_code_is_checked_only_when_asked() {
    let text = "see `WF-3` today\n";

    assert_eq!(of(text, false, false), []);
    assert_eq!(of(text, true, false), [m("WF-3", 1, 6)]);
}

#[test]
fn fenced_code_is_checked_only_when_asked() {
    let text = "```\nWF-3\n```\n";

    assert_eq!(of(text, true, false), []);
    assert_eq!(of(text, true, true), [m("WF-3", 2, 1)]);
}

#[test]
fn lines_count_from_the_top_of_the_file_frontmatter_included() {
    let text = "---\ntitle: T\n---\n\nsee WF-3\n";

    assert_eq!(of(text, true, false), [m("WF-3", 5, 5)]);
}
