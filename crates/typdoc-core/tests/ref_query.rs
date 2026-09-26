//! Covers SPC-13.
//!
//! `ref.*`/`refby.*` conditions, through `Project::list`, since following an arrow reads more
//! than the one schema and one document `query::evaluate` sees. Every case reads
//! `fixtures/valid/ref-query`:
//!
//! - `tickets/{key}.md` (schema `ticket`, code `T`): `status` (enum), `blocked_by` (`ref[]`,
//!   `target: "*"`) and `sources` (`ref[]`, `target: ["archive"]`).
//! - `archive/*.md` (schema `archive`, no code): `resolved_note` (string).
//! - `people/*.md` (schema `person`, no code, no document): `bogus_marker` (string), declared by
//!   no other schema, so a query that reaches for it proves which scope a `ref.*`/`refby.*`
//!   condition's inner condition was checked against.
//!
//! `T-1` and `T-5` each carry a dangling `blocked_by` ref (`T-99`, which no document has the key
//! of): `T-1`'s is one of two arrows, `T-5`'s is its only one. `T-2` is the only document any
//! other document's `blocked_by` points at: `T-1` and `T-3` (both `status: open`) and `T-6`
//! (`status: closed`), so `refby.*` over `blocked_by` tells apart "no holder at all" (every
//! ticket but `T-2`) from "a holder, and it matches" or "a holder, and it does not". `T-1`'s
//! body links to `archive/x.md`.

use std::ffi::OsString;
use std::io;
use std::path::PathBuf;

use typdoc_core::{Condition, Env, ListFilter, Project, Scope, Source};
use typdoc_testkit::fixtures::path;

const REF_QUERY: &str = "valid/ref-query";

/// No variable is ever read here: nothing in this file's fixture uses `imports`.
struct NoEnv;

impl Env for NoEnv {
    fn var(&self, _name: &str) -> Option<OsString> {
        None
    }

    fn current_dir(&self) -> io::Result<PathBuf> {
        Ok(PathBuf::from("."))
    }

    fn hostname(&self) -> String {
        "no-host".to_owned()
    }
}

fn project() -> Project {
    Project::load(&path(REF_QUERY), &NoEnv).expect("the fixture loads")
}

fn everything(project: &Project) -> Scope {
    Scope {
        source: Source::Everything,
        namespaces: project
            .config()
            .namespaces
            .iter()
            .map(|n| n.name.clone())
            .collect(),
        imports: Vec::new(),
    }
}

/// `list`'s matching paths for one `--where` expression, scoped to `collections` (empty means
/// every collection).
fn paths(project: &Project, collections: &[&str], expr: &str) -> Vec<String> {
    let scope = everything(project);
    let collections: Vec<String> = collections.iter().map(|s| (*s).to_owned()).collect();
    let condition: Condition = typdoc_core::parse_query(expr).expect("the expression parses");
    let filter = ListFilter {
        collections: &collections,
        codes: &[],
        wheres: &[condition],
        sort: &[],
    };
    let mut matched: Vec<String> = project
        .list(&scope, &filter)
        .unwrap_or_else(|e| panic!("{expr}: {e}"))
        .documents
        .into_iter()
        .map(|d| d.path)
        .collect();
    matched.sort();
    matched
}

fn error_of(project: &Project, collections: &[&str], expr: &str) -> String {
    let scope = everything(project);
    let collections: Vec<String> = collections.iter().map(|s| (*s).to_owned()).collect();
    let condition: Condition = typdoc_core::parse_query(expr).expect("the expression parses");
    let filter = ListFilter {
        collections: &collections,
        codes: &[],
        wheres: &[condition],
        sort: &[],
    };
    match project.list(&scope, &filter) {
        Ok(result) => panic!(
            "{expr}: expected an error, got {} documents",
            result.documents.len()
        ),
        Err(e) => e.to_string(),
    }
}

/// The dangling-ref documents (`T-1`, beside a resolved blocker; `T-5`, with only a dangling one)
/// fall in the `any`/`!=` half. Were a dangling ref skipped, both would be evaluated over `[T-2]`
/// alone (`status: resolved`) and land in the `all`/`=` half. Every target of `blocked_by`
/// declares `status`, so only the dangling ref is at work.
#[test]
fn ref_all_and_ref_any_negated_split_a_set_with_a_dangling_ref_counted_as_absent() {
    let project = project();

    let all_resolved = paths(
        &project,
        &["tickets"],
        "ref.all(blocked_by).status=resolved",
    );
    let any_not_resolved = paths(
        &project,
        &["tickets"],
        "ref.any(blocked_by).status!=resolved",
    );

    assert_eq!(
        all_resolved,
        [
            "tickets/T-2.md",
            "tickets/T-3.md",
            "tickets/T-4.md",
            "tickets/T-6.md"
        ],
        "every blocker of these is resolved (T-2, T-4 vacuously; T-3 and T-6 for real, both via T-2)"
    );
    assert_eq!(
        any_not_resolved,
        ["tickets/T-1.md", "tickets/T-5.md"],
        "each has a blocker that is not resolved: T-1's dangling T-99, T-5's only blocker T-99"
    );
    // None falls through, and none is counted twice.
    let mut whole = [all_resolved.clone(), any_not_resolved.clone()].concat();
    whole.sort();
    assert_eq!(
        whole,
        [
            "tickets/T-1.md",
            "tickets/T-2.md",
            "tickets/T-3.md",
            "tickets/T-4.md",
            "tickets/T-5.md",
            "tickets/T-6.md"
        ]
    );
    for dangling in ["tickets/T-1.md", "tickets/T-5.md"] {
        assert!(any_not_resolved.contains(&dangling.to_owned()));
        assert!(!all_resolved.contains(&dangling.to_owned()));
    }
}

/// `T-5`'s only blocker is the dangling `T-99`; `T-2` and `T-4` have `blocked_by: []`.
#[test]
fn ref_any_with_no_inner_counts_a_dangling_ref_as_an_existing_arrow() {
    let project = project();

    let has_a_blocker = paths(&project, &["tickets"], "ref.any(blocked_by)");
    let has_no_blocker = paths(&project, &["tickets"], "ref.none(blocked_by)");

    assert_eq!(
        has_a_blocker,
        [
            "tickets/T-1.md",
            "tickets/T-3.md",
            "tickets/T-5.md",
            "tickets/T-6.md"
        ]
    );
    assert_eq!(has_no_blocker, ["tickets/T-2.md", "tickets/T-4.md"]);
}

#[test]
fn refby_any_finds_the_document_some_other_documents_field_points_at() {
    let project = project();

    let pointed_at = paths(&project, &["tickets"], "refby.any(blocked_by)");
    assert_eq!(pointed_at, ["tickets/T-2.md"]);
}

/// Every ticket but `T-2` has no holder, so `all` and `none` are vacuously true for it. `T-2`'s
/// holders, `T-1` and `T-3` (open) and `T-6` (closed), tell "vacuously true" apart from "checked
/// and true".
#[test]
fn refby_any_with_an_inner_condition_checks_the_holders_not_me() {
    let project = project();
    let everyone_but_t2 = [
        "tickets/T-1.md",
        "tickets/T-3.md",
        "tickets/T-4.md",
        "tickets/T-5.md",
        "tickets/T-6.md",
    ];

    let matched = paths(&project, &["tickets"], "refby.any(blocked_by).status=open");
    assert_eq!(matched, ["tickets/T-2.md"]);

    let matched = paths(&project, &["tickets"], "refby.all(blocked_by).status=open");
    assert_eq!(matched, everyone_but_t2);

    let matched = paths(
        &project,
        &["tickets"],
        "refby.none(blocked_by).status=closed",
    );
    assert_eq!(matched, everyone_but_t2);
}

#[test]
fn dollar_body_works_in_both_directions() {
    let project = project();

    let has_body_ref = paths(&project, &["tickets"], "ref.any($body)");
    assert_eq!(has_body_ref, ["tickets/T-1.md"]);

    let linked_to = paths(&project, &["archive"], "refby.any($body)");
    assert_eq!(linked_to, ["archive/x.md"]);
}

/// `resolved_note` is declared only by `archive`, the target of `sources`, and not by `ticket`, the
/// outer scope: a check against the outer scope would refuse it, and one against every schema
/// could not tell this case from the next.
#[test]
fn the_condition_after_ref_star_is_checked_against_the_targets_schemas() {
    let project = project();

    let matched = paths(&project, &["tickets"], "ref.any(sources).resolved_note=*");
    assert_eq!(matched, ["tickets/T-1.md"]);
}

/// `bogus_marker` is declared only by `person`, a schema of this project that the target of
/// `sources` does not name.
#[test]
fn a_field_outside_the_targets_schemas_is_an_error_even_though_some_other_schema_has_it() {
    let project = project();

    let message = error_of(&project, &["tickets"], "ref.any(sources).bogus_marker=*");
    assert!(message.contains("bogus_marker"), "{message}");
}

#[test]
fn a_ref_field_no_schema_declares_is_an_error() {
    let project = project();

    let message = error_of(&project, &["tickets"], "ref.any(nope)");
    assert!(message.contains("nope"), "{message}");
}
