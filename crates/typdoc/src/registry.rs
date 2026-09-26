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

/// Commands the design names and the binary does not have. The list only shrinks, and it is
/// empty when v1 is finished.
pub const UNIMPLEMENTED_COMMANDS: &[&str] = &[
    "pull", // SPC-2
];

/// Exit codes SPC-3 names and no test makes the binary end with. The list only shrinks, and it
/// is empty when v1 is finished.
pub const UNPRODUCED_EXIT_CODES: &[u8] = &[];

/// Places where the binary answers a narrower question than the design describes, and that are
/// neither a missing command nor an exit code.
///
/// Each entry opens with a stable tag in brackets, matched by exactly one test (named in the
/// comment beside it) that pins the narrower behaviour. A test matches the tag, not the prose, so
/// rewording an entry breaks nothing, and a change to the behaviour turns the test red.
pub const KNOWN_GAPS: &[&str] = &[
    // crates/typdoc/tests/imports.rs:
    // `a_reverse_lookup_does_not_see_a_ref_from_an_imported_project`
    "[reverse-scope] a reverse lookup (`refs --reverse`) scans this project's own namespaces \
     only, not also the namespaces of every project this one imports, so \
     a ref written in an imported project's own document, pointing back into this project, is \
     missing from the result rather than being in it",
    // crates/typdoc/tests/imports.rs:
    // `a_body_link_across_an_import_has_its_anchor_left_unchecked`
    "[import-anchor] a body link that crosses an import has its target file's existence \
     checked (`body.links`) but not the `#anchor` after it (`body.anchors`); the same link to \
     a heading that does not exist would be reported inside one project",
];
