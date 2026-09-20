//! The headings of a file: which are listed, their levels, texts, slugs and line ranges.

use typdoc_core::{Heading, headings};

fn h(level: u8, text: &str, slug: &str, line: usize, end: usize) -> Heading {
    Heading {
        level,
        text: text.to_owned(),
        slug: slug.to_owned(),
        line,
        end,
    }
}

fn of(text: &str) -> Vec<Heading> {
    headings(text).unwrap()
}

fn slugs(text: &str) -> Vec<String> {
    of(text).into_iter().map(|heading| heading.slug).collect()
}

#[test]
fn a_heading_has_a_level_a_text_a_slug_and_the_line_it_is_on() {
    assert_eq!(
        of("# One\n\ntext\n\n### Three Words\n"),
        [
            h(1, "One", "one", 1, 5),
            h(3, "Three Words", "three-words", 5, 5)
        ]
    );
}

#[test]
fn a_section_runs_to_the_line_before_the_next_heading_of_the_same_or_a_shallower_level() {
    let text = "# A\n## B\ntext\n### C\n## D\n# E\n";

    assert_eq!(
        of(text),
        [
            h(1, "A", "a", 1, 5),
            h(2, "B", "b", 2, 4),
            h(3, "C", "c", 4, 4),
            h(2, "D", "d", 5, 5),
            h(1, "E", "e", 6, 6),
        ]
    );
}

#[test]
fn a_heading_with_nothing_under_it_ends_where_it_begins() {
    let found = of("# Top\n\n## Empty\n### Deeper\n## Sibling\n\nbody\n");

    assert_eq!((found[2].line, found[2].end), (4, 4), "### Deeper");
    assert_eq!(
        (found[1].line, found[1].end),
        (3, 4),
        "## Empty holds ### Deeper"
    );
    assert_eq!((found[3].line, found[3].end), (5, 7), "## Sibling");
    assert_eq!(
        (found[0].line, found[0].end),
        (1, 7),
        "# Top holds all of it"
    );
}

#[test]
fn the_last_section_runs_to_the_last_line_of_the_file() {
    let with_ending = of("# A\n\n## B\n\ntext\n");
    let without_ending = of("# A\n\n## B\n\ntext");
    let with_blank_lines = of("# A\n\n## B\n\ntext\n\n\n");

    assert_eq!(with_ending[1].end, 5);
    assert_eq!(without_ending[1].end, 5);
    assert_eq!(with_blank_lines[1].end, 7);
}

#[test]
fn lines_count_from_the_top_of_the_file_and_the_frontmatter_counts() {
    let text = "---\ntitle: T\ntags: [a]\n---\n\n# After\n";

    assert_eq!(of(text), [h(1, "After", "after", 6, 6)]);
}

#[test]
fn an_empty_frontmatter_block_is_cut_like_any_other() {
    assert_eq!(of("---\n---\n# After\n"), [h(1, "After", "after", 3, 3)]);
}

#[test]
fn a_file_with_no_frontmatter_is_all_body() {
    assert_eq!(of("# Only\n"), [h(1, "Only", "only", 1, 1)]);
}

#[test]
fn a_pair_of_dashed_lines_after_the_body_starts_is_not_frontmatter() {
    let text = "---\ntitle: T\n---\n# One\n\n---\n# Inside\n---\n\n# Two\n";

    assert_eq!(
        of(text),
        [
            h(1, "One", "one", 4, 6),
            h(1, "Inside", "inside", 7, 9),
            h(1, "Two", "two", 10, 10),
        ]
    );
}

#[test]
fn a_block_that_is_never_closed_has_no_body_to_read() {
    assert!(headings("---\ntitle: T\n# Not a heading\n").is_err());
}

#[test]
fn headings_inside_fenced_code_are_ignored() {
    let text = "# Real\n\n```\n# Not\n```\n\n~~~md\n## Not either\n~~~\n\n## Real too\n";

    assert_eq!(
        of(text).iter().map(|x| x.text.as_str()).collect::<Vec<_>>(),
        ["Real", "Real too"]
    );
}

#[test]
fn headings_inside_block_quotes_and_list_items_are_listed() {
    let text = "> ## Quoted\n\n- ## In a list item\n\n## Plain\n";

    let found = of(text);

    assert_eq!(
        found
            .iter()
            .map(|x| (x.text.as_str(), x.line))
            .collect::<Vec<_>>(),
        [("Quoted", 1), ("In a list item", 3), ("Plain", 5)]
    );
}

#[test]
fn a_setext_heading_is_on_the_line_of_its_text() {
    let found = of("Title\n=====\n\ntext\n\nSub\n---\n");

    assert_eq!(
        found,
        [h(1, "Title", "title", 1, 7), h(2, "Sub", "sub", 6, 7)]
    );
}

#[test]
fn the_text_of_a_heading_is_its_text_and_code_spans() {
    let found = of("## `code` and [link](x.md) *em* ![alt](i.png) <b>b</b> a<br>b\n");

    assert_eq!(found[0].text, "code and link em  b ab");
}

#[test]
fn dashes_and_underscores_stay_and_other_punctuation_goes() {
    assert_eq!(slugs("# A_B-c: d.e/f!\n"), ["a_b-c-def"]);
}

#[test]
fn a_repeated_slug_gets_a_number_that_skips_the_slugs_already_taken() {
    assert_eq!(
        slugs("# Dup\n# Dup\n# Dup 1\n# Dup\n"),
        ["dup", "dup-1", "dup-1-1", "dup-2"]
    );
}

#[test]
fn a_heading_whose_slug_is_empty_is_numbered_like_any_other() {
    assert_eq!(slugs("# !!!\n# 😀\n# ???\n# !!!\n"), ["", "-1", "-2", "-3"]);
}

#[test]
fn an_empty_heading_has_an_empty_slug() {
    let found = of("#\n\n## \n");

    assert_eq!((found[0].text.as_str(), found[0].slug.as_str()), ("", ""));
    assert_eq!(found[1].slug, "-1");
}

#[test]
fn thai_is_kept_as_written_with_its_vowel_and_tone_marks() {
    assert_eq!(slugs("# ทดสอบ ภาษาไทย ก็ได้\n"), ["ทดสอบ-ภาษาไทย-ก็ได้"]);
}

#[test]
fn each_space_becomes_a_hyphen_and_the_ends_are_not_trimmed() {
    assert_eq!(
        slugs("# a  b\n# Trailing hyphen -\n# - leading\n"),
        ["a--b", "trailing-hyphen--", "--leading"]
    );
}

#[test]
fn a_document_of_only_the_frontmatter_has_no_heading() {
    assert_eq!(of("---\ntitle: T\n---\n"), []);
}

#[test]
fn every_kind_of_line_ending_gives_the_same_lines() {
    let lf = of("# One\n\ntext\n\n## Two\n\nmore\n");
    let crlf = of("# One\r\n\r\ntext\r\n\r\n## Two\r\n\r\nmore\r\n");
    let cr = of("# One\r\rtext\r\r## Two\r\rmore\r");

    assert_eq!(lf, [h(1, "One", "one", 1, 7), h(2, "Two", "two", 5, 7)]);
    assert_eq!(crlf, lf);
    assert_eq!(cr, lf);
}

#[test]
fn a_frontmatter_block_is_found_whatever_its_line_endings_are() {
    let lf = of("---\ntitle: T\n---\n# H\n");
    let crlf = of("---\r\ntitle: T\r\n---\r\n# H\r\n");
    let cr = of("---\rtitle: T\r---\r# H\r");

    assert_eq!(lf, [h(1, "H", "h", 4, 4)]);
    assert_eq!(crlf, lf);
    assert_eq!(cr, lf);
}
