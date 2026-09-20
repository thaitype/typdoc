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
    "list", // story 1
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
];
