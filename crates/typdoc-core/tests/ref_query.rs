//! Evaluating `ref.*`/`refby.*` conditions: the seam is `Project::list`, since following an
//! arrow reads more than the one schema and one document `query::evaluate` sees. Every case here
//! reads `fixtures/valid/ref-query`, described in the fixture's own schemas:
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
/// every collection). Panics on a parse or evaluation error, since every test below that expects
/// success names its own expression by hand and already knows it parses.
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

/// The error `list` gives for one `--where` expression, or a panic if it unexpectedly succeeds.
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

// -------------------------------------------------------------------------------------------
// The ticket's own criterion: a pair of complementary queries over a set with a dangling ref.
// -------------------------------------------------------------------------------------------

/// The design's own worked example (`ref.all(blocked_by).status=resolved` and
/// `ref.any(blocked_by).status!=resolved`), over a set that has a dangling ref: every document is
/// in exactly one half, the two halves sum to the whole set, and the dangling-ref documents
/// (`T-1`, mixed with a resolved blocker; `T-5`, only a dangling one) fall into the `any`/`!=`
/// half, never the `all`/`=` half — which is what "a dangling ref counts as absent" has to mean
/// for the pair to split cleanly at all (a reached document lacking `status` would count the same
/// way, but `blocked_by`'s targets all declare `status`, so only the dangling ref is at work
/// here). If a dangling ref were skipped instead of counted, `T-1` and `T-5` would each be
/// evaluated over `[T-2]` alone (`status: resolved`): `ref.all(...)` would then be true for both
/// (moving them into `all_resolved`) and `ref.any(...)status!=resolved` would be false for both
/// (emptying `any_not_resolved`) — both halves below would read differently.
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
    // The dangling-ref documents land in exactly one half.
    for dangling in ["tickets/T-1.md", "tickets/T-5.md"] {
        assert!(any_not_resolved.contains(&dangling.to_owned()));
        assert!(!all_resolved.contains(&dangling.to_owned()));
    }
}

/// The design's own sentence: "a ticket whose only blocker points at nothing does not look
/// unblocked". `T-5`'s only `blocked_by` value is the dangling `T-99`: omitting `.EXPR` counts
/// arrows from the value written, so `ref.any(blocked_by)` must still find one for `T-5` (and for
/// `T-1` and `T-6`, which each have a real arrow too), while `T-2` and `T-4` (`blocked_by: []`)
/// have none.
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

// -------------------------------------------------------------------------------------------
// `refby.*`: arrows pointing at me, never dangling, `$body` included.
// -------------------------------------------------------------------------------------------

#[test]
fn refby_any_finds_the_document_some_other_documents_field_points_at() {
    let project = project();

    // Only T-2 is ever named by another ticket's `blocked_by` (T-1, T-3 and T-6 all point at
    // it); T-99 does not exist, and nothing points at T-1, T-3, T-4, T-5 or T-6 this way.
    let pointed_at = paths(&project, &["tickets"], "refby.any(blocked_by)");
    assert_eq!(pointed_at, ["tickets/T-2.md"]);
}

/// `T-2` is the only ticket with any holder at all: `T-1` and `T-3` (`status: open`), `T-6`
/// (`status: closed`). Every other ticket has zero holders, so `all`/`none` are vacuously true for
/// them regardless of `status` — the interesting case is `T-2`, where a real, non-matching holder
/// (`T-6`) makes `all(...status=open)` and `none(...status=closed)` false, while `any(...
/// status=open)` stays true because of `T-1`/`T-3`. A test that only checked "no holders at all"
/// would never tell "vacuously true" apart from "checked and true".
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

    // Only T-2 has a holder with status=open (T-1 or T-3 is enough for `any`).
    let matched = paths(&project, &["tickets"], "refby.any(blocked_by).status=open");
    assert_eq!(matched, ["tickets/T-2.md"]);

    // T-2 fails `all(...status=open)` because of T-6 (closed); everyone else is vacuously true.
    let matched = paths(&project, &["tickets"], "refby.all(blocked_by).status=open");
    assert_eq!(matched, everyone_but_t2);

    // T-2 fails `none(...status=closed)` because T-6 (a real holder) is closed; everyone else,
    // having no holder at all, is vacuously true.
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

    // T-1's body links to archive/x.md; no other ticket has a body link.
    let has_body_ref = paths(&project, &["tickets"], "ref.any($body)");
    assert_eq!(has_body_ref, ["tickets/T-1.md"]);

    // archive/x.md is the only document any body link points at.
    let linked_to = paths(&project, &["archive"], "refby.any($body)");
    assert_eq!(linked_to, ["archive/x.md"]);
}

// -------------------------------------------------------------------------------------------
// The scope of the condition after `ref.*(f)` follows `f`'s target, not the outer scope.
// -------------------------------------------------------------------------------------------

/// `sources`' `target` is `["archive"]`, never `"*"`. `resolved_note` is declared only by
/// `archive`, not by `ticket` — the `--collection tickets` outer scope this query itself runs
/// under — so a check that (wrongly) used the outer scope would refuse this as an unknown field;
/// one that (wrongly) fell back to "every schema in the project" would not be able to tell this
/// case apart from the next one. Only reading `sources`' own `target` gets both right.
#[test]
fn the_condition_after_ref_star_is_checked_against_the_targets_schemas() {
    let project = project();

    let matched = paths(&project, &["tickets"], "ref.any(sources).resolved_note=*");
    assert_eq!(matched, ["tickets/T-1.md"]);
}

/// `bogus_marker` is declared only by `person`, which `sources`' `target` (`["archive"]`) does
/// not name — even though `person` is a real schema of this project. Erroring here is what
/// proves the scope is `sources`' own target and not "every schema of the project".
#[test]
fn a_field_outside_the_targets_schemas_is_an_error_even_though_some_other_schema_has_it() {
    let project = project();

    let message = error_of(&project, &["tickets"], "ref.any(sources).bogus_marker=*");
    assert!(message.contains("bogus_marker"), "{message}");
}

/// `f` itself must be a ref/ref[] field (or `$body`) of some schema of the project; nothing here
/// declares `nope`, so `ref.any(nope)` is an error before any document is read.
#[test]
fn a_ref_field_no_schema_declares_is_an_error() {
    let project = project();

    let message = error_of(&project, &["tickets"], "ref.any(nope)");
    assert!(message.contains("nope"), "{message}");
}
