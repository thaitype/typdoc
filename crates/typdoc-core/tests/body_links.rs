//! Body links, reference definitions, duplicate labels and text that looks like a link but is
//! not: `typdoc_core::scan`, pure over one document's text (ticket 10 of the decisions is the
//! spec; see `docs/archived-design/design-decision-phase-1/_tickets/10-link-forms-checked.md`).

use typdoc_core::{BodyLink, DuplicateDefinition, Suspect, scan};

fn of(text: &str) -> typdoc_core::BodyLinks {
    scan(text).unwrap()
}

/// An inline or image occurrence (`is_reference: false`); reference-style occurrences are built
/// by hand where a test needs one, since the shape they come in varies with the test.
fn link(
    written: &str,
    target: Option<&str>,
    anchor: Option<&str>,
    line: usize,
    col: usize,
) -> BodyLink {
    BodyLink {
        written: written.to_owned(),
        target: target.map(str::to_owned),
        anchor: anchor.map(str::to_owned),
        line,
        col,
        is_reference: false,
    }
}

#[test]
fn an_inline_link_is_checked_at_its_opening_bracket() {
    let found = of("[t](a.md)\n");

    assert_eq!(found.links, [link("a.md", Some("a.md"), None, 1, 1)]);
}

#[test]
fn an_inline_link_with_a_heading_anchor_splits_path_and_fragment() {
    let found = of("[t](a.md#section)\n");

    assert_eq!(
        found.links,
        [link("a.md#section", Some("a.md"), Some("section"), 1, 1)]
    );
}

#[test]
fn an_image_is_checked_like_a_link_at_its_bang() {
    let found = of("![alt](pic.png)\n");

    assert_eq!(found.links, [link("pic.png", Some("pic.png"), None, 1, 1)]);
}

#[test]
fn a_self_anchor_has_no_target_and_names_the_document_itself() {
    let found = of("[t](#local)\n");

    assert_eq!(found.links, [link("#local", None, Some("local"), 1, 1)]);
}

#[test]
fn a_destination_is_percent_decoded_but_written_stays_as_authored() {
    let found = of("[t](my%20file.md)\n");

    assert_eq!(
        found.links,
        [link("my%20file.md", Some("my file.md"), None, 1, 1)]
    );
}

#[test]
fn an_angle_bracket_destination_with_a_space_is_a_real_link_not_a_suspect() {
    let found = of("[t](<my file.md>)\n");

    assert_eq!(
        found.links,
        [link("my file.md", Some("my file.md"), None, 1, 1)]
    );
    assert_eq!(found.suspects, []);
}

#[test]
fn reference_style_shortcut_collapsed_and_full_are_all_checked_and_counted() {
    let found = of(
        "[full][ref] and [collapsed][] and [shortcut]\n\n[ref]: a.md\n[collapsed]: a.md\n[shortcut]: a.md\n",
    );

    assert_eq!(found.links.len(), 3, "{:?}", found.links);
    for found_link in &found.links {
        assert_eq!(found_link.target.as_deref(), Some("a.md"));
        assert!(
            found_link.is_reference,
            "a reference-style occurrence, checked once at its definition: {found_link:?}"
        );
    }
    assert_eq!(found.definitions.len(), 3, "{:?}", found.definitions);
    for def in &found.definitions {
        assert_eq!(def.uses, 1, "{def:?}");
    }
}

#[test]
fn an_inline_link_is_not_marked_reference_style() {
    let found = of("[t](a.md)\n");

    assert!(!found.links[0].is_reference);
}

#[test]
fn an_autolink_is_skipped_entirely() {
    let found = of("<https://example.com/a.md>\n");

    assert_eq!(found.links, []);
}

#[test]
fn an_undefined_reference_is_not_reported_plain_text_to_commonmark() {
    let found = of("a[0][1] and [t][ref]\n");

    assert_eq!(found.links, [], "{:?}", found.links);
    assert_eq!(found.suspects, [], "{:?}", found.suspects);
}

#[test]
fn a_definition_is_reported_once_with_its_use_count() {
    let found = of("[t][ref] [t][ref] [t][ref]\n\n[ref]: notes/x.md\n");

    assert_eq!(found.definitions.len(), 1, "{:?}", found.definitions);
    let def = &found.definitions[0];
    assert_eq!(def.target.as_deref(), Some("notes/x.md"));
    assert_eq!(def.uses, 3);
    assert_eq!((def.line, def.col), (3, 1));
}

#[test]
fn an_unused_definition_is_checked_with_zero_uses() {
    let found = of("[ref]: notes/x.md\n");

    assert_eq!(found.definitions.len(), 1, "{:?}", found.definitions);
    assert_eq!(found.definitions[0].uses, 0);
    assert_eq!(found.links, []);
}

#[test]
fn a_label_defined_twice_reports_the_later_one_and_keeps_the_first_active() {
    let found = of("[t][ref]\n\n[ref]: a.md\n[ref]: b.md\n");

    assert_eq!(found.definitions.len(), 1, "{:?}", found.definitions);
    assert_eq!(found.definitions[0].target.as_deref(), Some("a.md"));
    assert_eq!(found.definitions[0].uses, 1);
    assert_eq!(
        found.duplicate_definitions,
        [DuplicateDefinition {
            line: 4,
            col: 1,
            first_line: 3
        }]
    );
    assert_eq!(found.links[0].target.as_deref(), Some("a.md"));
}

#[test]
fn labels_are_compared_case_folded_and_whitespace_collapsed() {
    let found = of("[t][A  B]\n\n[a b]: x.md\n[A B]: y.md\n");

    assert_eq!(found.definitions.len(), 1, "{:?}", found.definitions);
    assert_eq!(found.definitions[0].target.as_deref(), Some("x.md"));
    assert_eq!(found.definitions[0].uses, 1);
    assert_eq!(found.duplicate_definitions.len(), 1);
}

#[test]
fn a_definition_inside_a_fenced_code_block_is_not_a_definition() {
    let found = of("```\n[ref]: a.md\n```\n\n[t][ref]\n");

    assert_eq!(found.definitions, [], "{:?}", found.definitions);
    assert_eq!(found.links, [], "an undefined reference is plain text");
}

#[test]
fn an_unescaped_space_in_a_destination_is_a_suspect_at_the_opening_bracket() {
    let found = of("[t](my file.md)\n");

    assert_eq!(found.links, []);
    assert_eq!(
        found.suspects,
        [Suspect {
            inner: "my file.md".to_owned(),
            line: 1,
            col: 1,
            definition: false,
        }]
    );
}

#[test]
fn an_unescaped_space_in_an_image_destination_is_a_suspect_at_the_bang() {
    let found = of("![t](my pic.png)\n");

    assert_eq!(
        found.suspects,
        [Suspect {
            inner: "my pic.png".to_owned(),
            line: 1,
            col: 1,
            definition: false,
        }]
    );
}

#[test]
fn a_trailing_title_is_removed_before_the_extension_is_checked() {
    let found = of("[t](my file.md \"title\")\n");

    assert_eq!(found.suspects[0].inner, "my file.md");
}

#[test]
fn version_1_2_is_not_reported_a_digit_only_extension_does_not_count() {
    let found = of("[t](version 1.2)\n");

    assert_eq!(found.suspects, [], "{:?}", found.suspects);
}

#[test]
fn ask_mr_smith_is_reported_a_known_false_positive() {
    let found = of("[t](ask Mr.Smith)\n");

    assert_eq!(found.suspects.len(), 1, "{:?}", found.suspects);
    assert_eq!(found.suspects[0].inner, "ask Mr.Smith");
}

#[test]
fn a_line_that_looks_like_a_definition_but_has_a_space_is_a_suspect() {
    let found = of("[label]: my file.md\n");

    assert_eq!(
        found.suspects,
        [Suspect {
            inner: "my file.md".to_owned(),
            line: 1,
            col: 1,
            definition: true,
        }]
    );
}

#[test]
fn text_inside_a_fenced_code_block_that_looks_like_a_link_is_not_a_suspect() {
    let found = of("```\n[t](my file.md)\n```\n");

    assert_eq!(found.suspects, []);
    assert_eq!(found.links, []);
}

#[test]
fn text_inside_inline_code_that_looks_like_a_link_is_not_a_suspect() {
    let found = of("`[t](my file.md)`\n");

    assert_eq!(found.suspects, []);
}

#[test]
fn a_real_link_is_never_also_reported_as_a_suspect() {
    let found = of("[t](a.md)\n");

    assert_eq!(found.suspects, []);
}

#[test]
fn a_plain_shortcut_shaped_run_of_text_is_not_a_suspect_no_parens_follow() {
    // The common false-positive shape ticket 10 names: `a[0][1]` has brackets but no
    // parenthesis right after either `]`, so it never becomes a bracket-paren candidate.
    let found = of("a[0][1] and [t][ref]\n");

    assert_eq!(found.suspects, []);
}

#[test]
fn lines_count_from_the_top_of_the_file_frontmatter_included() {
    let text = "---\ntitle: T\n---\n\n[t](missing.md)\n";

    let found = of(text);

    assert_eq!(found.links[0].line, 5);
}
