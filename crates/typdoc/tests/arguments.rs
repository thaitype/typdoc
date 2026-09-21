//! The table under "Arguments that name a document" in the design: the string a document is
//! named by, built by hand from the table, round-trips through `get`. Covers a project with
//! one namespace, one with several, and, through `fixtures/valid/imports/main`, an import into
//! a one-namespace project and an import into a several-namespace one.

#[allow(dead_code, reason = "each test file uses part of the shared helper")]
mod common;

use std::path::Path;

use common::{Ran, Spawn, fixture};
use serde_json::json;

fn get(project: &Path, argument: &str) -> Ran {
    Spawn::args(["get", argument, "--json"]).cwd(project).run()
}

/// `argument` names the document at `path`: `get` on it succeeds and reads that file.
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

/// A namespace prefix on a path argument is read the same as one on a key argument (Choosing
/// a namespace, step 1: "a prefix on a key or path argument"); the path itself stays relative
/// to the project folder, exactly as it is with no prefix (a path is not narrowed by scope).
#[test]
fn a_namespace_prefix_on_a_path_validates_the_namespace_and_leaves_the_path_as_written() {
    let project = fixture("valid/several-namespaces");

    // The prefix names a real namespace, and the path (still project-relative, not
    // namespace-relative) names a real document: the prefix does not change what is read.
    round_trips(
        &project,
        "story-2:story-2/tickets/WF-9.md",
        "story-2/tickets/WF-9.md",
    );

    // The prefix names a real namespace, but the path, read from the project folder rather
    // than from that namespace's folder, names nothing: not found.
    let missing = get(&project, "story-2:tickets/WF-9.md");
    assert_eq!(missing.code, 5, "{}", missing.stderr);

    // The prefix names no namespace of the project: bad arguments, same as an unknown prefix
    // on a key.
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

/// An unknown `project::` prefix stays bad arguments (exit 1), the same as an unknown
/// `namespace:` prefix already is; a configured one is read, below.
#[test]
fn an_unknown_double_colon_prefix_is_bad_arguments_whether_it_prefixes_a_key_or_a_path() {
    let project = fixture("valid/several-namespaces");

    for argument in ["chief::WF-9", "chief::story-2/tickets/WF-9.md"] {
        let ran = get(&project, argument);

        assert_eq!(ran.code, 1, "{argument}: {}", ran.stderr);
    }
}

/// The table's "In an imported project" row, through `fixtures/valid/imports/main`, which
/// imports `memory_import` (one namespace, `default`) and `several_import` (several
/// namespaces, reusing `valid/several-namespaces`). The path form takes only the project
/// prefix, whichever project it names, and needs no namespace prefix, since — as the text
/// above the table says of the path form generally — it "needs to know nothing about how many
/// namespaces a project has"; the key form needs a namespace prefix too once the imported
/// project has more than one.
#[test]
fn with_an_import_the_path_form_takes_no_namespace_and_the_key_form_needs_one_past_the_first() {
    let project = fixture("valid/imports/main");

    // One namespace on the far side of the import.
    round_trips(
        &project,
        "memory_import::learnings/LRN-1.md",
        "learnings/LRN-1.md",
    );
    round_trips(&project, "memory_import::LRN-1", "learnings/LRN-1.md");

    // Several namespaces on the far side: the path form still takes no namespace prefix...
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
    // ...while the key form needs one, the same as a ref into a several-namespace import does
    // (Refs, "Across namespaces"): `several_import::WF-9` alone is bad arguments.
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

    // A namespace that is not called `default` on the far side of a one-namespace import
    // (`named_import`, whose only namespace is `only`): the path form still needs no prefix,
    // and it works for an uncoded document too.
    round_trips(&project, "named_import::only/notes/a.md", "only/notes/a.md");
}
