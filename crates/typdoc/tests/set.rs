//! `typdoc set`, through the built binary. Every case that writes runs on a copy (`common::
//! spawn_fixture` or `typdoc_testkit::staging::stage` directly), never on a fixture in the
//! repository's own tree.

#[allow(dead_code, reason = "each test file uses part of the shared helper")]
mod common;

use std::path::Path;

use common::{NOTES, Ran, Scratch, Spawn, fixture};
use serde_json::{Value, json};

fn run(project: &Path, args: &[&str]) -> Ran {
    Spawn::args(args.iter().copied()).cwd(project).run()
}

fn ok_json(ran: &Ran) -> Value {
    assert_eq!(ran.code, 0, "stderr: {}", ran.stderr);
    assert_eq!(ran.stderr, "");
    ran.stdout_json()
}

/// A schema of one enum field with the design's own example transitions
/// (`docs/design.md`, Schema format), for the `--if` and refusal tests below.
const STATUS_SCHEMA: &str = r#"{
  "name": "ticket",
  "fields": {
    "title": { "type": "string", "required": true },
    "status": {
      "type": "enum",
      "values": ["open", "claimed", "resolved", "closed"],
      "transitions": { "open": ["claimed", "closed"], "claimed": ["resolved", "open", "closed"] }
    }
  }
}"#;

const STATUS_COLLECTION: &str = r#"{ "match": "*.md", "schema": "ticket.json" }"#;

/// Goal criterion 1 ("a write changes only the fields it is given"), through `set` itself: the
/// transitions fixture is the first fixture whose declared command is a write
/// (`.chief/story-2/_contract/testing-decisions.md`, item 4). It runs on a copy, asserts exit 2
/// and the finding, and — the part a mere exit-code check would miss — that the document's
/// bytes on disk are exactly what they were before the run.
#[test]
fn the_transitions_fixture_exits_2_names_the_finding_and_leaves_the_document_unchanged() {
    let dir = fixture("broken/frontmatter.transitions");
    let spec = typdoc_testkit::spec::FixtureSpec::load(&dir, "frontmatter.transitions")
        .expect("the fixture's own spec");
    assert!(
        spec.is_write(),
        "a `set` fixture must be classified as a write"
    );
    let staged = typdoc_testkit::staging::stage(&dir, &spec).expect("staged outside the repo");
    let before = std::fs::read(dir.join("ticket.md")).unwrap();

    let ran = Spawn::args(&spec.command).cwd(staged.dir()).run();

    assert_eq!(ran.code, 2, "stdout: {} stderr: {}", ran.stdout, ran.stderr);
    assert_eq!(ran.stdout, "");
    let error = ran.stderr_json();
    assert_eq!(error["code"], json!(2));
    let details = error["details"].as_array().expect("a details array");
    assert_eq!(details.len(), 1, "{details:?}");
    assert_eq!(details[0]["rule"], json!("frontmatter.transitions"));
    assert_eq!(
        details[0]["message"],
        json!("transition not allowed: open -> resolved")
    );

    let after = std::fs::read(staged.dir().join("ticket.md")).unwrap();
    assert_eq!(
        before, after,
        "a refused write must leave the document's bytes untouched"
    );
    let repo_copy = std::fs::read(dir.join("ticket.md")).unwrap();
    assert_eq!(
        before, repo_copy,
        "the repository's own fixture must be untouched too"
    );
}

/// A false `--if` leaves the file byte-identical and exits 3 with the failed condition in
/// `details`.
#[test]
fn a_false_if_writes_nothing_and_exits_3_naming_the_condition() {
    let project = Scratch::project(&[
        (".typdoc/collections/tickets.json", STATUS_COLLECTION),
        ("ticket.json", STATUS_SCHEMA),
        ("a.md", "---\ntitle: A ticket\nstatus: open\n---\n\nBody.\n"),
    ]);
    let before = std::fs::read(project.path().join("a.md")).unwrap();

    let ran = run(
        project.path(),
        &[
            "set",
            "a.md",
            "status=claimed",
            "--if",
            "status=closed",
            "--json",
        ],
    );

    assert_eq!(ran.code, 3, "stdout: {} stderr: {}", ran.stdout, ran.stderr);
    assert_eq!(ran.stdout, "");
    let error = ran.stderr_json();
    assert_eq!(error["code"], json!(3));
    let details = error["details"].as_array().expect("a details array");
    assert_eq!(details.len(), 1, "{details:?}");
    assert_eq!(details[0]["message"], json!("`status=closed` is false"));

    let after = std::fs::read(project.path().join("a.md")).unwrap();
    assert_eq!(before, after);
}

/// A true `--if` lets the write through, under the same lock (design, `typdoc set`).
#[test]
fn a_true_if_lets_the_write_through() {
    let project = Scratch::project(&[
        (".typdoc/collections/tickets.json", STATUS_COLLECTION),
        ("ticket.json", STATUS_SCHEMA),
        ("a.md", "---\ntitle: A ticket\nstatus: open\n---\n\nBody.\n"),
    ]);

    let ran = run(
        project.path(),
        &[
            "set",
            "a.md",
            "status=claimed",
            "--if",
            "status=open",
            "--json",
        ],
    );

    let out = ok_json(&ran);
    assert_eq!(out["document"]["fields"]["status"], json!("claimed"));
}

/// Contract item 8's default: a `set` on a document outside every namespace is an ordinary
/// write, and its `--json` leaves `namespace` out, the way `get` already leaves it out for a
/// document with none.
#[test]
fn a_document_outside_every_namespace_is_an_ordinary_write_with_no_namespace_in_json() {
    let project = Scratch::empty();
    project.file(
        ".typdoc/config.json",
        r#"{ "version": 1, "namespaces": ["story-1", "story-2"] }"#,
    );
    project.file(
        ".typdoc/collections/notes.json",
        r#"{ "match": "*.md", "schema": "schemas/n.json" }"#,
    );
    project.file("schemas/n.json", r#"{ "name": "n", "fields": {} }"#);
    project.file("README.md", "---\ntitle: Root\n---\n\nBody.\n");
    project.file("story-1/a.md", "---\ntitle: A\n---\n\nBody.\n");
    project.file("story-2/b.md", "---\ntitle: B\n---\n\nBody.\n");

    let ran = run(
        project.path(),
        &["set", "README.md", "title=Changed", "--json"],
    );

    let out = ok_json(&ran);
    assert_eq!(out["document"]["path"], json!("README.md"));
    assert_eq!(out["document"]["fields"]["title"], json!("Changed"));
    assert!(out["document"].get("namespace").is_none(), "{out}");

    let after = std::fs::read_to_string(project.path().join("README.md")).unwrap();
    assert_eq!(after, "---\ntitle: Changed\n---\n\nBody.\n");
}

/// The value promise (decision 20/ticket 5) and the printing fix (ticket 7) together, proved
/// through `set` itself: a number no primitive holds, on a field `set` never touches, is written
/// back unchanged and printed with the digits the document holds, not a converted value. Checked
/// on the bytes of standard output, because reading `--json` back into parsed JSON cannot tell
/// `99999999999999999999` from what a primitive would round it to (ticket 7's own report).
#[test]
fn a_number_no_primitive_holds_on_an_untouched_field_is_written_back_and_printed_unchanged() {
    let project = Scratch::project(&[
        (
            ".typdoc/collections/notes.json",
            r#"{ "match": "*.md", "schema": "note.json" }"#,
        ),
        (
            "note.json",
            r#"{ "name": "note", "fields": { "title": { "type": "string" }, "count": { "type": "number" } } }"#,
        ),
        (
            "a.md",
            "---\ntitle: Before\ncount: 99999999999999999999\n---\n\nBody.\n",
        ),
    ]);

    let ran = run(project.path(), &["set", "a.md", "title=After", "--json"]);

    assert_eq!(ran.code, 0, "stderr: {}", ran.stderr);
    assert!(
        ran.stdout.contains(r#""count":99999999999999999999"#),
        "the digits must appear unconverted on stdout: {}",
        ran.stdout
    );

    // The value promise is about what a read of the file gives back, not about whether the
    // writer quotes the scalar: `yaml_serde` quotes a value past `u128` that would otherwise
    // read as a number were it left bare (`ser.rs`'s own scalar-style inference), and
    // `frontmatter::fields` reads a scalar's text the same either way, quoted or not. So the
    // digits surviving the round trip is what "unchanged" means here, checked two ways: the
    // digits are still on disk untouched, and reading the file back through `get` prints them
    // unconverted, exactly as `set`'s own `--json` just did above.
    let after = std::fs::read_to_string(project.path().join("a.md")).unwrap();
    assert!(
        after.contains("99999999999999999999"),
        "the digits must survive a write that never touched this field: {after:?}"
    );
    let reread = run(project.path(), &["get", "a.md", "--json"]);
    assert_eq!(reread.code, 0, "stderr: {}", reread.stderr);
    assert!(
        reread.stdout.contains(r#""count":99999999999999999999"#),
        "a later read must still print the same unconverted digits: {}",
        reread.stdout
    );
}

/// The hand-written golden for `set`'s text-mode shape (contract, text-output shapes: the same
/// labeled block `get` prints, for the document as it stands after the write). `valid/minimal`'s
/// `note.md` writes `title` before `tags`, and `set` here only touches `title`, so the labeled
/// block shows the changed title and the untouched `tags` in that same file order.
#[test]
fn set_without_json_prints_the_labeled_block_after_the_write() {
    let project = Scratch::project(&NOTES);
    project.file(
        "note.md",
        "---\ntitle: A minimal note\ntags: [alpha, beta]\n---\n\nBody.\n",
    );

    let ran = run(project.path(), &["set", "note.md", "title=A changed note"]);

    assert_eq!(ran.code, 0, "stderr: {}", ran.stderr);
    assert_eq!(ran.stderr, "");
    assert_eq!(
        ran.stdout,
        "path: note.md\n\
         collection: notes\n\
         schema: note\n\
         namespace: default\n\
         title: A changed note\n\
         tags: alpha,beta\n"
    );
}

/// The already-fixed error path (contract decision 4, ticket 16's own "a validation failure on
/// set" case): a validation failure prints plain text on stderr without `--json`, never the
/// `--json` error object.
#[test]
fn set_without_json_prints_a_plain_text_error_on_a_validation_failure() {
    let project = Scratch::project(&[
        (".typdoc/collections/tickets.json", STATUS_COLLECTION),
        ("ticket.json", STATUS_SCHEMA),
        ("a.md", "---\ntitle: A ticket\nstatus: open\n---\n\nBody.\n"),
    ]);
    let before = std::fs::read(project.path().join("a.md")).unwrap();

    let ran = run(project.path(), &["set", "a.md", "status=resolved"]);

    assert_eq!(ran.code, 2, "stdout: {} stderr: {}", ran.stdout, ran.stderr);
    assert_eq!(ran.stdout, "");
    assert!(
        ran.stderr.starts_with("typdoc: ") && !ran.stderr.starts_with("typdoc: {"),
        "{}",
        ran.stderr
    );

    let after = std::fs::read(project.path().join("a.md")).unwrap();
    assert_eq!(
        before, after,
        "a refused write must leave the file untouched"
    );
}

/// `k=` removes a field entirely.
#[test]
fn k_with_nothing_after_the_equals_removes_the_field() {
    let project = Scratch::project(&NOTES);
    project.file("a.md", "---\ntitle: Keep\nowner: someone\n---\n\nBody.\n");

    let ran = run(project.path(), &["set", "a.md", "owner=", "--json"]);

    let out = ok_json(&ran);
    assert!(out["document"]["fields"].get("owner").is_none(), "{out}");

    let after = std::fs::read_to_string(project.path().join("a.md")).unwrap();
    assert!(!after.contains("owner"), "{after:?}");
}

/// A list field is replaced by a comma-separated value, not appended to.
#[test]
fn a_comma_separated_value_replaces_a_list_field_rather_than_appending_to_it() {
    let project = Scratch::project(&[
        (
            ".typdoc/collections/notes.json",
            r#"{ "match": "*.md", "schema": "note.json" }"#,
        ),
        (
            "note.json",
            r#"{ "name": "note", "fields": { "tags": { "type": "list" } } }"#,
        ),
        ("a.md", "---\ntags: [alpha, beta]\n---\n\nBody.\n"),
    ]);

    let ran = run(
        project.path(),
        &["set", "a.md", "tags=gamma,delta", "--json"],
    );

    let out = ok_json(&ran);
    assert_eq!(out["document"]["fields"]["tags"], json!(["gamma", "delta"]));
}

/// Writing an `auto` field directly is a validation error, and nothing is written.
#[test]
fn writing_an_auto_field_directly_is_refused_and_writes_nothing() {
    let project = Scratch::project(&[
        (
            ".typdoc/collections/notes.json",
            r#"{ "match": "*.md", "schema": "note.json" }"#,
        ),
        (
            "note.json",
            r#"{ "name": "note", "fields": { "title": { "type": "string" }, "updated_at": { "type": "datetime", "auto": "update" } } }"#,
        ),
        (
            "a.md",
            "---\ntitle: Before\nupdated_at: 2026-01-01T00:00:00+00:00\n---\n\nBody.\n",
        ),
    ]);
    let before = std::fs::read(project.path().join("a.md")).unwrap();

    let ran = run(
        project.path(),
        &[
            "set",
            "a.md",
            "updated_at=2020-01-01T00:00:00+00:00",
            "--json",
        ],
    );

    assert_eq!(ran.code, 2, "stdout: {} stderr: {}", ran.stdout, ran.stderr);
    let error = ran.stderr_json();
    let details = error["details"].as_array().expect("a details array");
    assert_eq!(details[0]["rule"], json!("frontmatter.types"));

    let after = std::fs::read(project.path().join("a.md")).unwrap();
    assert_eq!(before, after);
}

/// `auto: update` is stamped only when at least one value actually changes: setting a field to
/// the value it already holds is not a change (the value promise ticket 5/decision 20 already
/// gives; `auto: update`'s own condition reuses it), and the stamp is untouched; setting it to a
/// different value is, and the stamp moves.
#[test]
fn auto_update_is_stamped_only_when_a_value_actually_changes() {
    let schema = r#"{ "name": "note", "fields": {
        "title": { "type": "string" },
        "updated_at": { "type": "datetime", "auto": "update" }
    } }"#;
    let doc = "---\ntitle: Same\nupdated_at: 2026-01-01T00:00:00+00:00\n---\n\nBody.\n";

    let unchanged = Scratch::project(&[
        (
            ".typdoc/collections/notes.json",
            r#"{ "match": "*.md", "schema": "note.json" }"#,
        ),
        ("note.json", schema),
        ("a.md", doc),
    ]);
    let ran = run(unchanged.path(), &["set", "a.md", "title=Same", "--json"]);
    let out = ok_json(&ran);
    assert_eq!(
        out["document"]["fields"]["updated_at"],
        json!("2026-01-01T00:00:00+00:00"),
        "setting a field to the value it already holds is not a change"
    );

    let changed = Scratch::project(&[
        (
            ".typdoc/collections/notes.json",
            r#"{ "match": "*.md", "schema": "note.json" }"#,
        ),
        ("note.json", schema),
        ("a.md", doc),
    ]);
    let ran = run(
        changed.path(),
        &["set", "a.md", "title=Different", "--json"],
    );
    let out = ok_json(&ran);
    assert_ne!(
        out["document"]["fields"]["updated_at"],
        json!("2026-01-01T00:00:00+00:00"),
        "a real change must stamp `auto: update`"
    );
}

// ---------------------------------------------------------------------------------------------
// Ticket 31 (M-19): `--set`/`field=value` escaping (design §Query, as applied to `set`).
// ---------------------------------------------------------------------------------------------

/// `\*` is a literal `*`. Repro: today the backslash is kept in the stored value instead.
#[test]
fn a_backslash_star_in_a_scalar_value_is_stored_as_a_literal_star() {
    let project = Scratch::project(&NOTES);
    project.file("a.md", "---\ntitle: Before\n---\n\nBody.\n");

    let ran = run(project.path(), &["set", "a.md", r"title=a\*b", "--json"]);

    let out = ok_json(&ran);
    assert_eq!(out["document"]["fields"]["title"], json!("a*b"));
    let after = std::fs::read_to_string(project.path().join("a.md")).unwrap();
    assert!(after.contains("a*b"), "{after:?}");
    assert!(!after.contains(r"a\*b"), "{after:?}");
}

/// A bare, unescaped `*` has no wildcard meaning in `--set` (unlike `--where`/`--if`) and is an
/// outright error. Repro: today it is silently accepted and stored literally.
#[test]
fn a_bare_unescaped_star_in_a_set_value_is_refused_and_writes_nothing() {
    let project = Scratch::project(&NOTES);
    project.file("a.md", "---\ntitle: Before\n---\n\nBody.\n");
    let before = std::fs::read(project.path().join("a.md")).unwrap();

    let ran = run(project.path(), &["set", "a.md", "title=x*y", "--json"]);

    assert_eq!(ran.code, 1, "stdout: {} stderr: {}", ran.stdout, ran.stderr);
    let after = std::fs::read(project.path().join("a.md")).unwrap();
    assert_eq!(before, after, "a refused set must write nothing");
}

/// `\,` is a literal `,`; a scalar field has nothing to split into, so the escape just yields the
/// character, with no list produced.
#[test]
fn a_backslash_comma_in_a_scalar_value_is_stored_as_a_literal_comma() {
    let project = Scratch::project(&NOTES);
    project.file("a.md", "---\ntitle: Before\n---\n\nBody.\n");

    let ran = run(project.path(), &["set", "a.md", r"title=a\,b", "--json"]);

    let out = ok_json(&ran);
    assert_eq!(out["document"]["fields"]["title"], json!("a,b"));
}

/// A list field still splits on an unescaped `,`, but a `\,` inside an item is a literal comma,
/// not a split point.
#[test]
fn a_list_field_splits_on_unescaped_commas_but_not_on_an_escaped_one() {
    let project = Scratch::project(&[
        (
            ".typdoc/collections/notes.json",
            r#"{ "match": "*.md", "schema": "note.json" }"#,
        ),
        (
            "note.json",
            r#"{ "name": "note", "fields": { "tags": { "type": "list" } } }"#,
        ),
        ("a.md", "---\ntags: [alpha]\n---\n\nBody.\n"),
    ]);

    let ran = run(project.path(), &["set", "a.md", r"tags=a\,b,c", "--json"]);

    let out = ok_json(&ran);
    assert_eq!(out["document"]["fields"]["tags"], json!(["a,b", "c"]));
}

/// `\\` is a literal `\`.
#[test]
fn a_double_backslash_in_a_value_is_stored_as_a_single_backslash() {
    let project = Scratch::project(&NOTES);
    project.file("a.md", "---\ntitle: Before\n---\n\nBody.\n");

    let ran = run(project.path(), &["set", "a.md", r"title=a\\b", "--json"]);

    let out = ok_json(&ran);
    assert_eq!(out["document"]["fields"]["title"], json!(r"a\b"));
}

/// `\` followed by anything other than `,`, `*` or `\` is an unrecognized escape and refused.
#[test]
fn an_unrecognized_escape_in_a_set_value_is_refused_and_writes_nothing() {
    let project = Scratch::project(&NOTES);
    project.file("a.md", "---\ntitle: Before\n---\n\nBody.\n");
    let before = std::fs::read(project.path().join("a.md")).unwrap();

    let ran = run(project.path(), &["set", "a.md", r"title=a\qb", "--json"]);

    assert_eq!(ran.code, 1, "stdout: {} stderr: {}", ran.stdout, ran.stderr);
    let error = ran.stderr_json();
    assert!(
        error["error"].as_str().unwrap_or_default().contains(r"\q"),
        "{error}"
    );
    let after = std::fs::read(project.path().join("a.md")).unwrap();
    assert_eq!(before, after, "a refused set must write nothing");
}

/// A value ending in a lone, unmatched `\` is refused rather than silently dropped or kept.
#[test]
fn a_value_ending_in_a_lone_backslash_is_refused_and_writes_nothing() {
    let project = Scratch::project(&NOTES);
    project.file("a.md", "---\ntitle: Before\n---\n\nBody.\n");
    let before = std::fs::read(project.path().join("a.md")).unwrap();

    let ran = run(project.path(), &["set", "a.md", r"title=a\", "--json"]);

    assert_eq!(ran.code, 1, "stdout: {} stderr: {}", ran.stdout, ran.stderr);
    let after = std::fs::read(project.path().join("a.md")).unwrap();
    assert_eq!(before, after, "a refused set must write nothing");
}

/// A `set --if` naming a `ref.*`/`refby.*` condition is refused plainly (bad arguments) rather
/// than evaluated wrongly: an open limit, stated rather than hidden.
#[test]
fn an_if_with_a_ref_condition_is_refused_plainly() {
    let project = Scratch::project(&NOTES);
    project.file("a.md", "---\ntitle: A\n---\n\nBody.\n");

    let ran = run(
        project.path(),
        &[
            "set",
            "a.md",
            "title=B",
            "--if",
            "ref.any(x).path=y",
            "--json",
        ],
    );

    assert_eq!(ran.code, 1, "stdout: {} stderr: {}", ran.stdout, ran.stderr);
}

// --- ticket 30 (M-18): `set` refuses a write-time cycle on an `acyclic` field ---

const WF_ACYCLIC_SCHEMA: &str = r#"{
  "name": "ticket",
  "code": "WF",
  "fields": {
    "title": { "type": "string" },
    "blocked_by": { "type": "ref[]", "target": "*", "acyclic": true }
  }
}"#;

const WF_ACYCLIC_COLLECTION: [(&str, &str); 2] = [
    (
        ".typdoc/collections/tickets.json",
        r#"{ "match": "tickets/{key}.md", "schema": "wf.json" }"#,
    ),
    ("wf.json", WF_ACYCLIC_SCHEMA),
];

/// M-18 repro (ticket 30): `WF-1` already has `blocked_by: [WF-5]`. Setting `WF-5`'s own
/// `blocked_by` to `WF-1` would close the cycle `WF-1 -> WF-5 -> WF-1` on the `acyclic` field
/// `blocked_by`; this must be refused at write time (exit 2, nothing written, `refs.acyclic` in
/// `details`), the same command, not caught only by a later `validate` run.
#[test]
fn set_refuses_a_write_that_closes_an_immediate_cycle_on_an_acyclic_field() {
    let project = Scratch::project(&WF_ACYCLIC_COLLECTION);
    project.file(
        "tickets/WF-1.md",
        "---\ntitle: One\nblocked_by: [WF-5]\n---\n",
    );
    project.file("tickets/WF-5.md", "---\ntitle: Five\n---\n");
    let before = std::fs::read(project.path().join("tickets/WF-5.md")).unwrap();

    let ran = run(
        project.path(),
        &["set", "WF-5", "blocked_by=WF-1", "--json"],
    );

    assert_eq!(ran.code, 2, "stdout: {} stderr: {}", ran.stdout, ran.stderr);
    assert_eq!(ran.stdout, "");
    let error = ran.stderr_json();
    let details = error["details"].as_array().expect("a details array");
    assert!(
        details.iter().any(|f| f["rule"] == json!("refs.acyclic")),
        "{details:?}"
    );

    let after = std::fs::read(project.path().join("tickets/WF-5.md")).unwrap();
    assert_eq!(
        before, after,
        "a refused write must leave the document's bytes untouched"
    );
}

/// A longer chain (3 documents) closed by the write, not just the immediate two-document repro
/// above: the indirect case ticket 30 also requires, exercising `refs::cyclic_nodes`'s walk
/// through more than one intermediate document.
#[test]
fn set_refuses_a_write_that_closes_a_longer_chain_into_a_cycle() {
    let project = Scratch::project(&WF_ACYCLIC_COLLECTION);
    project.file(
        "tickets/WF-1.md",
        "---\ntitle: One\nblocked_by: [WF-2]\n---\n",
    );
    project.file(
        "tickets/WF-2.md",
        "---\ntitle: Two\nblocked_by: [WF-3]\n---\n",
    );
    project.file("tickets/WF-3.md", "---\ntitle: Three\n---\n");
    let before = std::fs::read(project.path().join("tickets/WF-3.md")).unwrap();

    let ran = run(
        project.path(),
        &["set", "WF-3", "blocked_by=WF-1", "--json"],
    );

    assert_eq!(ran.code, 2, "stdout: {} stderr: {}", ran.stdout, ran.stderr);
    let details = ran.stderr_json()["details"].as_array().unwrap().clone();
    assert!(
        details.iter().any(|f| f["rule"] == json!("refs.acyclic")),
        "{details:?}"
    );
    let after = std::fs::read(project.path().join("tickets/WF-3.md")).unwrap();
    assert_eq!(before, after);
}

/// A write untouched by an unrelated, pre-existing cycle elsewhere succeeds normally: the "no
/// false refusal" half of ticket 30's fix — the pre-write scan necessarily still finds that
/// unrelated cycle too, and it must not leak into this write's own refusal.
#[test]
fn set_is_not_refused_by_an_unrelated_pre_existing_cycle_elsewhere() {
    let project = Scratch::project(&WF_ACYCLIC_COLLECTION);
    project.file(
        "tickets/WF-10.md",
        "---\ntitle: Ten\nblocked_by: [WF-11]\n---\n",
    );
    project.file(
        "tickets/WF-11.md",
        "---\ntitle: Eleven\nblocked_by: [WF-10]\n---\n",
    );
    project.file("tickets/WF-1.md", "---\ntitle: One\n---\n");

    let ran = run(project.path(), &["set", "WF-1", "title=Updated", "--json"]);

    let out = ok_json(&ran);
    assert_eq!(out["document"]["fields"]["title"], json!("Updated"));
}
