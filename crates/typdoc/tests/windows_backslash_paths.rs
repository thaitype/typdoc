//! Covers SPC-2 on Windows: a project-relative path argument written with `\` names the same
//! document as the one written with `/`, with or without a `namespace:` or `project::` prefix.
//!
//! Windows only: on Unix `\` is a character a file name may hold.

#![cfg(windows)]

#[allow(dead_code, reason = "each test file uses part of the shared helper")]
mod common;

use common::{Ran, Spawn, fixture};
use serde_json::json;

fn get(argument: &str, project: &str) -> Ran {
    Spawn::args(["get", argument, "--json"])
        .cwd(fixture(project))
        .run()
}

#[test]
fn a_project_path_with_backslashes_names_the_document_the_slashed_one_does() {
    for argument in [r"learnings/LRN-1.md", r"learnings\LRN-1.md"] {
        let ran = get(argument, "valid/imports/memory");

        assert_eq!(ran.code, 0, "{argument}: {}", ran.stderr);
        assert_eq!(
            ran.stdout_json()["document"]["path"],
            json!("learnings/LRN-1.md"),
            "{argument}"
        );
    }
}

#[test]
fn a_namespace_prefixed_path_with_backslashes_names_the_document_the_slashed_one_does() {
    for argument in [r"story-1:tickets/WF-1.md", r"story-1:tickets\WF-1.md"] {
        let ran = get(argument, "valid/several-namespaces");

        assert_eq!(ran.code, 0, "{argument}: {}", ran.stderr);
        let document = ran.stdout_json()["document"].clone();
        assert_eq!(document["path"], json!("tickets/WF-1.md"), "{argument}");
        assert_eq!(document["namespace"], json!("story-1"), "{argument}");
    }
}

#[test]
fn a_project_prefixed_path_with_backslashes_names_the_document_the_slashed_one_does() {
    for argument in [
        r"memory_import::learnings/LRN-1.md",
        r"memory_import::learnings\LRN-1.md",
    ] {
        let ran = get(argument, "valid/imports/main");

        assert_eq!(ran.code, 0, "{argument}: {}", ran.stderr);
        let document = ran.stdout_json()["document"].clone();
        assert_eq!(document["path"], json!("learnings/LRN-1.md"), "{argument}");
        assert_eq!(document["project"], json!("memory_import"), "{argument}");
    }
}
