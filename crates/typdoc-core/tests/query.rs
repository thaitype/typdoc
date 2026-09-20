//! The scalar half of the query language: the grammar of a plain condition, its escaping and
//! glob, coercion by the field's type, the pseudo-fields, and the rules for absence and
//! negation. Every error below is one the grammar names (`docs/design.md`, Query); the schemas
//! and documents are built by hand, never read from a fixture, since none of this touches a
//! file.

use std::collections::BTreeMap;

use typdoc_core::{
    Condition, Document, Field, FieldRef, FieldType, Item, Op, QueryError, Resolved, Value,
    evaluate, parse_query as parse,
};

fn field(kind: FieldType) -> Field {
    Field {
        kind,
        required: None,
        default: None,
        values: None,
        transitions: None,
        target: None,
        acyclic: None,
        auto: None,
        overrides: None,
        extra: serde_json::Map::new(),
    }
}

fn enum_field(values: &[&str]) -> Field {
    Field {
        values: Some(values.iter().map(|v| (*v).to_owned()).collect()),
        ..field(FieldType::Enum)
    }
}

fn schema(fields: &[(&str, Field)]) -> Resolved {
    let map: BTreeMap<String, Field> = fields
        .iter()
        .map(|(name, f)| ((*name).to_owned(), f.clone()))
        .collect();
    Resolved::new("test".to_owned(), None, map)
}

fn number(n: i64) -> Value {
    Value::Number(serde_json::Number::from(n))
}

/// A document with no code and no key, in a namespace of one, holding `fields`. Individual
/// tests override `path`, `key` and `code` with struct-update syntax where a pseudo-field is
/// what is under test.
fn doc(fields: &[(&str, Value)]) -> Document {
    Document {
        path: "doc.md".to_owned(),
        namespace: "default".to_owned(),
        key: None,
        code: None,
        collection: "docs".to_owned(),
        schema: "test".to_owned(),
        fields: fields
            .iter()
            .map(|(name, v)| ((*name).to_owned(), v.clone()))
            .collect(),
    }
}

// -------------------------------------------------------------------------------------------
// Grammar: one case for each error the grammar names.
// -------------------------------------------------------------------------------------------

#[test]
fn a_backslash_before_an_unrecognized_character_is_an_error() {
    let err = parse(r"status=a\nb").unwrap_err();
    assert!(matches!(err, QueryError::Syntax { .. }), "{err}");
}

#[test]
fn a_value_ending_in_a_lone_backslash_is_an_error() {
    let err = parse(r"status=open\").unwrap_err();
    assert!(matches!(err, QueryError::Syntax { .. }), "{err}");
}

#[test]
fn a_space_around_the_operator_is_an_error_with_a_did_you_mean_hint() {
    let err = parse("status = open").unwrap_err();
    let QueryError::Syntax { hint, .. } = &err else {
        panic!("expected a syntax error, got {err}");
    };
    assert_eq!(hint.as_deref(), Some("did you mean status=open?"));
}

#[test]
fn an_empty_value_is_an_error_in_where_and_if() {
    let err = parse("status=").unwrap_err();
    assert!(matches!(err, QueryError::Syntax { .. }), "{err}");
}

#[test]
fn an_empty_alternative_in_a_list_is_the_same_error_as_an_empty_value() {
    let err = parse("status=open,").unwrap_err();
    assert!(matches!(err, QueryError::Syntax { .. }), "{err}");
}

#[test]
fn an_ordering_comparison_with_more_than_one_value_is_an_error() {
    let err = parse("estimate<3,5").unwrap_err();
    assert!(matches!(err, QueryError::Syntax { .. }), "{err}");
}

#[test]
fn a_literal_value_outside_the_enum_is_an_error() {
    let schema = schema(&[(
        "status",
        enum_field(&["open", "claimed", "resolved", "closed"]),
    )]);
    let document = doc(&[("status", Value::Text("open".to_owned()))]);
    let condition = parse("status=bogus").unwrap();

    let err = evaluate(&condition, &schema, &document).unwrap_err();

    assert!(matches!(err, QueryError::NotAnEnumValue { .. }), "{err}");
}

#[test]
fn a_glob_against_an_enum_is_never_an_out_of_enum_error() {
    let schema = schema(&[(
        "status",
        enum_field(&["open", "claimed", "resolved", "closed"]),
    )]);
    let document = doc(&[("status", Value::Text("open".to_owned()))]);
    let condition = parse("status=op*").unwrap();

    assert!(evaluate(&condition, &schema, &document).unwrap());
}

#[test]
fn an_ordering_comparison_on_a_type_that_is_not_number_date_or_datetime_is_an_error() {
    let schema = schema(&[("title", field(FieldType::String))]);
    let document = doc(&[("title", Value::Text("Cosmos".to_owned()))]);
    let condition = parse("title<Cosmos").unwrap();

    let err = evaluate(&condition, &schema, &document).unwrap_err();

    assert!(
        matches!(err, QueryError::OrderingNotAllowed { .. }),
        "{err}"
    );
}

#[test]
fn an_ordering_comparison_with_a_glob_reports_the_type_error_first_when_the_type_is_not_orderable()
{
    let schema = schema(&[("title", field(FieldType::String))]);
    let document = doc(&[("title", Value::Text("Cosmos".to_owned()))]);
    let condition = parse("title<*").unwrap();

    let err = evaluate(&condition, &schema, &document).unwrap_err();

    assert!(
        matches!(err, QueryError::OrderingNotAllowed { .. }),
        "a type error should be reported ahead of a value-shape error: {err}"
    );
}

#[test]
fn an_unknown_field_name_is_an_error_not_an_empty_result() {
    let schema = schema(&[("title", field(FieldType::String))]);
    let document = doc(&[("title", Value::Text("Cosmos".to_owned()))]);
    let condition = parse("bogus=x").unwrap();

    let err = evaluate(&condition, &schema, &document).unwrap_err();

    assert!(
        matches!(&err, QueryError::UnknownField(name) if name == "bogus"),
        "{err}"
    );
}

#[test]
fn a_value_that_cannot_be_coerced_to_its_fields_type_is_an_error() {
    let schema = schema(&[("estimate", field(FieldType::Number))]);
    let document = doc(&[("estimate", number(3))]);
    let condition = parse("estimate=abc").unwrap();

    let err = evaluate(&condition, &schema, &document).unwrap_err();

    assert!(matches!(err, QueryError::CannotCoerce { .. }), "{err}");
}

// -------------------------------------------------------------------------------------------
// Grammar: shape of a successful parse.
// -------------------------------------------------------------------------------------------

#[test]
fn a_plain_condition_parses_into_field_operator_and_value() {
    let condition = parse("status=open").unwrap();

    assert_eq!(
        condition,
        Condition {
            field: FieldRef::Named("status".to_owned()),
            op: Op::Eq,
            items: vec![Item::Literal("open".to_owned())],
        }
    );
}

#[test]
fn every_pseudo_field_name_is_recognized() {
    for (text, want) in [
        ("path=a.md", FieldRef::Path),
        ("key=WF-3", FieldRef::Key),
        ("code=WF", FieldRef::Code),
        ("collection=tickets", FieldRef::Collection),
        ("schema=wayfinder", FieldRef::Schema),
        ("namespace=default", FieldRef::Namespace),
    ] {
        assert_eq!(parse(text).unwrap().field, want, "{text}");
    }
}

#[test]
fn not_equal_parses_as_its_own_operator() {
    assert_eq!(parse("status!=resolved").unwrap().op, Op::Ne);
}

#[test]
fn an_escaped_comma_and_an_escaped_star_are_literal_characters() {
    let condition = parse(r"title=Cosmos\, or SQL").unwrap();

    assert_eq!(
        condition.items,
        vec![Item::Literal("Cosmos, or SQL".to_owned())]
    );
}

#[test]
fn a_literal_escaped_star_is_not_a_glob() {
    assert_eq!(
        parse(r"k=\*").unwrap().items,
        vec![Item::Literal("*".to_owned())]
    );
}

#[test]
fn an_unescaped_star_splits_the_value_into_glob_segments() {
    let condition = parse("title=Cosmos*").unwrap();

    assert_eq!(
        condition.items,
        vec![Item::Glob(vec!["Cosmos".to_owned(), String::new()])]
    );
}

#[test]
fn a_bare_star_alone_is_present_not_a_glob() {
    assert_eq!(parse("owner=*").unwrap().items, vec![Item::Present]);
}

#[test]
fn a_list_of_alternatives_splits_on_unescaped_commas() {
    let condition = parse("status=open,claimed").unwrap();

    assert_eq!(
        condition.items,
        vec![
            Item::Literal("open".to_owned()),
            Item::Literal("claimed".to_owned())
        ]
    );
}

// -------------------------------------------------------------------------------------------
// Coercion by type: equality and ordering compare the typed value, not the text.
// -------------------------------------------------------------------------------------------

#[test]
fn a_number_field_is_compared_by_value_not_by_text() {
    let schema = schema(&[("estimate", field(FieldType::Number))]);
    let five = doc(&[("estimate", number(5))]);
    let two = doc(&[("estimate", number(2))]);
    let at_least_three = parse("estimate>=3").unwrap();

    assert!(evaluate(&at_least_three, &schema, &five).unwrap());
    assert!(!evaluate(&at_least_three, &schema, &two).unwrap());
}

#[test]
fn a_date_shaped_value_compares_against_a_datetime_fields_date_part() {
    let schema = schema(&[("updated_at", field(FieldType::Datetime))]);
    let document = doc(&[(
        "updated_at",
        Value::Datetime("2026-09-19T14:30:00Z".to_owned()),
    )]);

    let before_the_20th = parse("updated_at<2026-09-20").unwrap();
    let before_the_19th = parse("updated_at<2026-09-19").unwrap();

    assert!(evaluate(&before_the_20th, &schema, &document).unwrap());
    assert!(!evaluate(&before_the_19th, &schema, &document).unwrap());
}

#[test]
fn a_document_without_the_field_fails_an_ordering_comparison() {
    let schema = schema(&[("due", field(FieldType::Date))]);
    let no_due = doc(&[]);

    let is_before = parse("due<2026-01-01").unwrap();

    assert!(!evaluate(&is_before, &schema, &no_due).unwrap());
}

#[test]
fn a_value_that_never_coerced_to_its_type_fails_an_ordering_comparison_the_same_as_absence() {
    let schema = schema(&[("due", field(FieldType::Date))]);
    // Kept as written, as ticket 5 decided: a value that does not fit its field's type stays
    // text instead of the typed `Value::Date`, exactly what `frontmatter::fields` leaves it as.
    let misfit_due = doc(&[("due", Value::Text("not-a-date".to_owned()))]);

    let is_before = parse("due<2026-01-01").unwrap();

    assert!(!evaluate(&is_before, &schema, &misfit_due).unwrap());
}

/// The exception the design names in the same breath as the absence rule: `<`, `<=`, `>`, `>=`
/// are not paired the way `=`/`!=` are, so a complementary pair of ordering conditions does NOT
/// split a set with none left over. A document without the field, or whose stored value never
/// coerced to the field's type, fails both halves and is left out of both, unlike the `=`/`!=`
/// pairs above (`split`'s `is_eq != is_ne` invariant does not hold here, so this test does not
/// use that helper).
#[test]
fn a_pair_of_ordering_conditions_leaves_a_document_without_the_field_out_of_both_halves() {
    let schema = schema(&[("due", field(FieldType::Date))]);
    let docs = [
        Document {
            path: "early.md".to_owned(),
            ..doc(&[("due", Value::Date("2025-06-01".to_owned()))])
        },
        Document {
            path: "late.md".to_owned(),
            ..doc(&[("due", Value::Date("2026-06-01".to_owned()))])
        },
        Document {
            path: "no-due.md".to_owned(),
            ..doc(&[])
        },
        Document {
            path: "misfit-due.md".to_owned(),
            ..doc(&[("due", Value::Text("not-a-date".to_owned()))])
        },
    ];

    let before = parse("due<2026-01-01").unwrap();
    let on_or_after = parse("due>=2026-01-01").unwrap();

    let mut matched_before = Vec::new();
    let mut matched_on_or_after = Vec::new();
    let mut in_neither = Vec::new();
    for document in &docs {
        let is_before = evaluate(&before, &schema, document).unwrap();
        let is_on_or_after = evaluate(&on_or_after, &schema, document).unwrap();
        match (is_before, is_on_or_after) {
            (true, false) => matched_before.push(document.path.as_str()),
            (false, true) => matched_on_or_after.push(document.path.as_str()),
            (false, false) => in_neither.push(document.path.as_str()),
            (true, true) => panic!(
                "{} matched both halves of an ordering pair, which cannot happen",
                document.path
            ),
        }
    }

    assert_eq!(matched_before, ["early.md"]);
    assert_eq!(matched_on_or_after, ["late.md"]);
    // The exception in practice: unlike an `=`/`!=` pair, the two ordering halves do not cover
    // the whole set on their own — the document with no `due` and the one whose `due` never
    // coerced are both left out of both halves.
    assert_eq!(in_neither, ["no-due.md", "misfit-due.md"]);
    assert_eq!(
        matched_before.len() + matched_on_or_after.len() + in_neither.len(),
        docs.len()
    );
}

#[test]
fn a_glob_on_an_array_field_matches_when_any_element_matches() {
    let schema = schema(&[("blocked_by", field(FieldType::RefList))]);
    let document = doc(&[(
        "blocked_by",
        Value::List(vec!["WF-1".to_owned(), "RFC-2".to_owned()]),
    )]);

    let starts_wf = parse("blocked_by=WF-*").unwrap();
    let starts_zz = parse("blocked_by=ZZ-*").unwrap();

    assert!(evaluate(&starts_wf, &schema, &document).unwrap());
    assert!(!evaluate(&starts_zz, &schema, &document).unwrap());
}

#[test]
fn a_pseudo_field_is_read_from_the_document_itself_not_from_the_schema() {
    let empty_schema = schema(&[]);
    let a_document = Document {
        path: "tickets/WF-3.md".to_owned(),
        key: Some("WF-3".to_owned()),
        ..doc(&[])
    };

    let by_path = parse("path=tickets/WF-3.md").unwrap();
    let by_key = parse("key=WF-3").unwrap();

    assert!(evaluate(&by_path, &empty_schema, &a_document).unwrap());
    assert!(evaluate(&by_key, &empty_schema, &a_document).unwrap());
}

// -------------------------------------------------------------------------------------------
// Absence and negation: `!=` is exactly NOT `=`, and a complementary pair splits a set of
// documents in two with none left over and none counted twice.
// -------------------------------------------------------------------------------------------

/// Runs `eq` and its complement `ne` against every document, checks the two never agree (the
/// defining property of "`!=` is exactly NOT `=`"), and returns which documents fell into each
/// half.
fn split<'a>(
    schema: &Resolved,
    docs: &'a [Document],
    eq: &Condition,
    ne: &Condition,
) -> (Vec<&'a str>, Vec<&'a str>) {
    let mut matched = Vec::new();
    let mut unmatched = Vec::new();
    for document in docs {
        let is_eq = evaluate(eq, schema, document).unwrap();
        let is_ne = evaluate(ne, schema, document).unwrap();
        assert_ne!(
            is_eq, is_ne,
            "`!=` must be exactly NOT `=` for {}",
            document.path
        );
        if is_eq {
            matched.push(document.path.as_str());
        } else {
            unmatched.push(document.path.as_str());
        }
    }
    (matched, unmatched)
}

#[test]
fn equal_and_not_equal_split_a_set_with_a_document_missing_the_field_left_in_the_not_equal_half() {
    let schema = schema(&[(
        "status",
        enum_field(&["open", "claimed", "resolved", "closed"]),
    )]);
    let docs = [
        Document {
            path: "open.md".to_owned(),
            ..doc(&[("status", Value::Text("open".to_owned()))])
        },
        Document {
            path: "claimed.md".to_owned(),
            ..doc(&[("status", Value::Text("claimed".to_owned()))])
        },
        Document {
            path: "no-status.md".to_owned(),
            ..doc(&[])
        },
    ];

    let is_open = parse("status=open").unwrap();
    let is_not_open = parse("status!=open").unwrap();

    let (matched, unmatched) = split(&schema, &docs, &is_open, &is_not_open);

    assert_eq!(matched, ["open.md"]);
    assert_eq!(unmatched, ["claimed.md", "no-status.md"]);
    assert_eq!(matched.len() + unmatched.len(), docs.len());
}

#[test]
fn present_and_absent_split_a_set_with_an_empty_value_counted_as_absent() {
    let schema = schema(&[("owner", field(FieldType::String))]);
    let docs = [
        Document {
            path: "has-owner.md".to_owned(),
            ..doc(&[("owner", Value::Text("alice".to_owned()))])
        },
        Document {
            path: "empty-owner.md".to_owned(),
            ..doc(&[("owner", Value::Text(String::new()))])
        },
        Document {
            path: "no-owner.md".to_owned(),
            ..doc(&[])
        },
    ];

    let has_owner = parse("owner=*").unwrap();
    let has_no_owner = parse("owner!=*").unwrap();

    let (matched, unmatched) = split(&schema, &docs, &has_owner, &has_no_owner);

    assert_eq!(matched, ["has-owner.md"]);
    assert_eq!(unmatched, ["empty-owner.md", "no-owner.md"]);
    assert_eq!(matched.len() + unmatched.len(), docs.len());
}
