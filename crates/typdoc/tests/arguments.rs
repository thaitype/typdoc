//! Covers SPC-2, SPC-7.
//!
//! Every argument is written by hand from SPC-2's table, never taken from typdoc's own output.

#[allow(dead_code, reason = "each test file uses part of the shared helper")]
mod common;

use std::path::Path;

use common::{Ran, Spawn, fixture};
use serde_json::json;

fn get(project: &Path, argument: &str) -> Ran {
    Spawn::args(["get", argument, "--json"]).cwd(project).run()
}

fn round_trips(project: &Path, argument: &str, path: &str) {
    let ran = get(project, argument);

    assert_eq!(ran.code, 0, "{argument}: {}", ran.stderr);
    assert_eq!(
        ran.stdout_json()["document"]["path"],
        json!(path),
        "{argument}"
    );
}

#[test]
fn in_one_namespace_the_path_form_and_the_bare_key_form_both_read_the_document() {
    let project = fixture("valid/templates");

    for (path, key) in [("tickets/WF-1.md", "WF-1"), ("tickets/WF-12.md", "WF-12")] {
        round_trips(&project, path, path);
        round_trips(&project, key, path);
    }

    // An uncoded document has a path and no key at all.
    round_trips(&project, "notes/a.md", "notes/a.md");
}

#[test]
fn in_several_namespaces_the_path_form_takes_no_prefix_and_the_key_form_needs_one() {
    let project = fixture("valid/several-namespaces");

    round_trips(
        &project,
        "story-1/tickets/WF-1.md",
        "story-1/tickets/WF-1.md",
    );
    round_trips(&project, "story-1:WF-1", "story-1/tickets/WF-1.md");
    round_trips(
        &project,
        "story-2/tickets/WF-9.md",
        "story-2/tickets/WF-9.md",
    );
    round_trips(&project, "story-2:WF-9", "story-2/tickets/WF-9.md");
}

#[test]
fn a_namespace_prefix_on_a_path_validates_the_namespace_and_leaves_the_path_as_written() {
    let project = fixture("valid/several-namespaces");

    round_trips(
        &project,
        "story-2:story-2/tickets/WF-9.md",
        "story-2/tickets/WF-9.md",
    );

    // The prefix names a real namespace, but the path, read from the project folder rather
    // than from that namespace's folder, names nothing: not found.
    let missing = get(&project, "story-2:tickets/WF-9.md");
    assert_eq!(missing.code, 5, "{}", missing.stderr);

    let unknown = get(&project, "nosuch:story-2/tickets/WF-9.md");
    assert_eq!(unknown.code, 1, "{}", unknown.stderr);
    assert!(
        unknown.stderr_json()["error"]
            .as_str()
            .unwrap()
            .contains("nosuch"),
        "{}",
        unknown.stderr
    );
}

#[test]
fn an_unknown_double_colon_prefix_is_bad_arguments_whether_it_prefixes_a_key_or_a_path() {
    let project = fixture("valid/several-namespaces");

    for argument in ["chief::WF-9", "chief::story-2/tickets/WF-9.md"] {
        let ran = get(&project, argument);

        assert_eq!(ran.code, 1, "{argument}: {}", ran.stderr);
    }
}

#[test]
fn with_an_import_the_path_form_takes_no_namespace_and_the_key_form_needs_one_past_the_first() {
    let project = fixture("valid/imports/main");

    // `memory_import` has one namespace, `default`.
    round_trips(
        &project,
        "memory_import::learnings/LRN-1.md",
        "learnings/LRN-1.md",
    );
    round_trips(&project, "memory_import::LRN-1", "learnings/LRN-1.md");

    // `several_import` has several.
    round_trips(
        &project,
        "several_import::story-1/tickets/WF-1.md",
        "story-1/tickets/WF-1.md",
    );
    round_trips(
        &project,
        "several_import::story-2/tickets/WF-9.md",
        "story-2/tickets/WF-9.md",
    );
    round_trips(
        &project,
        "several_import::story-1:WF-1",
        "story-1/tickets/WF-1.md",
    );
    round_trips(
        &project,
        "several_import::story-2:WF-9",
        "story-2/tickets/WF-9.md",
    );

    // `named_import` has one namespace, and it is not called `default`: it is `only`.
    round_trips(&project, "named_import::only/notes/a.md", "only/notes/a.md");
}
