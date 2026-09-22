//! The commands the binary has, and the differences between the design and the binary.

use clap::CommandFactory;

use crate::cli::Cli;

/// The commands the CLI has, read from the definition it parses arguments with.
pub fn commands() -> Vec<String> {
    Cli::command()
        .get_subcommands()
        .map(|command| command.get_name().to_owned())
        .collect()
}

/// Commands the design names and the binary does not have yet. Each is a difference between
/// the design and the binary, with the story expected to deliver it as a comment. The list only shrinks, and it is empty
/// when v1 is finished.
pub const UNIMPLEMENTED_COMMANDS: &[&str] = &[
    "new",  // story 2
    "set",  // story 2
    "mv",   // story 2
    "pull", // story 3
];

/// Exit codes the design's table names and no test makes the binary end with. Each is a
/// difference between the design and the binary, with the story expected to deliver it. The list only
/// shrinks, and it is empty when v1 is finished.
pub const UNPRODUCED_EXIT_CODES: &[u8] = &[
    3, // story 2
    4, // story 2
    7, // story 2
];

/// Gaps between the design and the binary that are neither a missing command, rule nor exit
/// code, so none of the lists above holds them: each is a place where the binary answers a
/// narrower question than the design describes, rather than refusing or being silent about it.
/// Each entry opens with a short, stable tag in brackets, matched by exactly one test (named in
/// the comment beside it) that demonstrates the binary's present, narrower behaviour — the tag,
/// not the sentence after it, is what a test matches, so reworking the prose never silently
/// breaks the match; a change to the behaviour itself is meant to turn the test red, which is
/// what keeps an entry here honest without a second, generated list to compare it against.
pub const KNOWN_GAPS: &[&str] = &[
    // crates/typdoc/tests/frontmatter_scalars.rs:
    // `a_field_written_with_no_value_reads_the_same_as_an_empty_string`
    "[empty-value] a field written with no value and one written as an empty string are read as \
     the same thing, so the two cannot be told apart and a write would put back whichever form \
     the writer emits; the design says each keeps the form it was written in and that `--json` \
     shows the first as `null`",
    // crates/typdoc/tests/imports.rs:
    // `a_reverse_lookup_does_not_see_a_ref_from_an_imported_project`
    "[reverse-scope] a reverse lookup (`refs --reverse`) scans this project's own namespaces \
     only; the design says it also scans the namespaces of every project this one imports, so \
     a ref written in an imported project's own document, pointing back into this project, is \
     missing from the result rather than being in it",
    // crates/typdoc/tests/frontmatter_scalars.rs:
    // `a_number_outside_the_integer_range_is_printed_converted_not_as_written`
    "[number-text] a field typed `number` is printed in `--json` from a value converted out of \
     the text, not with the digits written in the document; the design says the digits are \
     printed, so a number outside the range an integer holds comes back as a float that has \
     lost them, and two documents whose numbers differ by one print the same value",
    // crates/typdoc/tests/imports.rs:
    // `a_body_link_across_an_import_has_its_anchor_left_unchecked`
    "[import-anchor] a body link that crosses an import has its target file's existence \
     checked (`body.links`) but not the `#anchor` after it (`body.anchors`); the same link to \
     a heading that does not exist would be reported inside one project",
];
