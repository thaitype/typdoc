//! Covers SPC-13.
//!
//! The plain condition, and the parsing of `ref.*`/`refby.*`. Schemas and documents are built by
//! hand, since none of this touches a file; evaluating a `ref.*`/`refby.*` condition needs a
//! project's ref graph, and `tests/ref_query.rs` tests it.

use std::collections::BTreeMap;

use typdoc_core::{
    Condition, Dir, Document, Field, FieldRef, FieldType, Item, Number, Op, PlainCondition, Quant,
    QueryError, RefField, Resolved, Value, evaluate, parse_query as parse,
};

/// `parse` for a plain condition: a `ref.*`/`refby.*` result would mean the text under test is
/// not what it looks like.
fn plain(expr: &str) -> PlainCondition {
    match parse(expr).unwrap() {
        Condition::Plain(condition) => condition,
        Condition::Ref(_) => panic!("expected a plain condition, got a ref condition: {expr}"),
    }
}

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
    Value::Number(Number::read(&n.to_string()).expect("a JSON number"))
}

fn doc(fields: &[(&str, Value)]) -> Document {
    Document {
        path: "doc.md".to_owned(),
        namespace: Some("default".to_owned()),
        key: None,
        code: None,
        collection: "docs".to_owned(),
        schema: "test".to_owned(),
        project: None,
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
    let condition = plain("status=bogus");

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
    let condition = plain("status=op*");

    assert!(evaluate(&condition, &schema, &document).unwrap());
}

#[test]
fn an_ordering_comparison_on_a_type_that_is_not_number_date_or_datetime_is_an_error() {
    let schema = schema(&[("title", field(FieldType::String))]);
    let document = doc(&[("title", Value::Text("Cosmos".to_owned()))]);
    let condition = plain("title<Cosmos");

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
    let condition = plain("title<*");

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
    let condition = plain("bogus=x");

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
    let condition = plain("estimate=abc");

    let err = evaluate(&condition, &schema, &document).unwrap_err();

    assert!(matches!(err, QueryError::CannotCoerce { .. }), "{err}");
}

// -------------------------------------------------------------------------------------------
// Grammar: shape of a successful parse.
// -------------------------------------------------------------------------------------------

#[test]
fn a_plain_condition_parses_into_field_operator_and_value() {
    let condition = plain("status=open");

    assert_eq!(
        condition,
        PlainCondition {
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
        assert_eq!(plain(text).field, want, "{text}");
    }
}

#[test]
fn not_equal_parses_as_its_own_operator() {
    assert_eq!(plain("status!=resolved").op, Op::Ne);
}

#[test]
fn an_escaped_comma_and_an_escaped_star_are_literal_characters() {
    let condition = plain(r"title=Cosmos\, or SQL");

    assert_eq!(
        condition.items,
        vec![Item::Literal("Cosmos, or SQL".to_owned())]
    );
}

#[test]
fn a_literal_escaped_star_is_not_a_glob() {
    assert_eq!(plain(r"k=\*").items, vec![Item::Literal("*".to_owned())]);
}

#[test]
fn an_unescaped_star_splits_the_value_into_glob_segments() {
    let condition = plain("title=Cosmos*");

    assert_eq!(
        condition.items,
        vec![Item::Glob(vec!["Cosmos".to_owned(), String::new()])]
    );
}

#[test]
fn a_bare_star_alone_is_present_not_a_glob() {
    assert_eq!(plain("owner=*").items, vec![Item::Present]);
}

#[test]
fn a_list_of_alternatives_splits_on_unescaped_commas() {
    let condition = plain("status=open,claimed");

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
    let at_least_three = plain("estimate>=3");

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

    let before_the_20th = plain("updated_at<2026-09-20");
    let before_the_19th = plain("updated_at<2026-09-19");

    assert!(evaluate(&before_the_20th, &schema, &document).unwrap());
    assert!(!evaluate(&before_the_19th, &schema, &document).unwrap());
}

#[test]
fn a_document_without_the_field_fails_an_ordering_comparison() {
    let schema = schema(&[("due", field(FieldType::Date))]);
    let no_due = doc(&[]);

    let is_before = plain("due<2026-01-01");

    assert!(!evaluate(&is_before, &schema, &no_due).unwrap());
}

#[test]
fn a_value_that_never_coerced_to_its_type_fails_an_ordering_comparison_the_same_as_absence() {
    let schema = schema(&[("due", field(FieldType::Date))]);
    // A value that does not fit its field's type stays `Text`, as the reader leaves it.
    let misfit_due = doc(&[("due", Value::Text("not-a-date".to_owned()))]);

    let is_before = plain("due<2026-01-01");

    assert!(!evaluate(&is_before, &schema, &misfit_due).unwrap());
}

/// `<`, `<=`, `>` and `>=` are the exception to the absence rule: a document without the field,
/// or whose value never coerced, fails both halves of an ordering pair, so `split` does not
/// apply.
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

    let before = plain("due<2026-01-01");
    let on_or_after = plain("due>=2026-01-01");

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

    let starts_wf = plain("blocked_by=WF-*");
    let starts_zz = plain("blocked_by=ZZ-*");

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

    let by_path = plain("path=tickets/WF-3.md");
    let by_key = plain("key=WF-3");

    assert!(evaluate(&by_path, &empty_schema, &a_document).unwrap());
    assert!(evaluate(&by_key, &empty_schema, &a_document).unwrap());
}

// -------------------------------------------------------------------------------------------
// Absence and negation.
// -------------------------------------------------------------------------------------------

/// Runs `eq` and its complement `ne` over every document, checks that the two never agree, and
/// returns the two halves.
fn split<'a>(
    schema: &Resolved,
    docs: &'a [Document],
    eq: &PlainCondition,
    ne: &PlainCondition,
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

    let is_open = plain("status=open");
    let is_not_open = plain("status!=open");

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

    let has_owner = plain("owner=*");
    let has_no_owner = plain("owner!=*");

    let (matched, unmatched) = split(&schema, &docs, &has_owner, &has_no_owner);

    assert_eq!(matched, ["has-owner.md"]);
    assert_eq!(unmatched, ["empty-owner.md", "no-owner.md"]);
    assert_eq!(matched.len() + unmatched.len(), docs.len());
}

// -------------------------------------------------------------------------------------------
// `ref.*`/`refby.*`: grammar only.
// -------------------------------------------------------------------------------------------

fn ref_condition(expr: &str) -> typdoc_core::RefCondition {
    match parse(expr).unwrap() {
        Condition::Ref(condition) => condition,
        Condition::Plain(_) => panic!("expected a ref condition, got a plain condition: {expr}"),
    }
}

#[test]
fn ref_and_refby_are_ordinary_field_names_when_not_followed_by_a_dot() {
    // `ref`/`refby` are only ever read as a direction when immediately followed by `.`; a plain
    // condition may still use either as a literal field name.
    assert_eq!(plain("ref=x").field, FieldRef::Named("ref".to_owned()));
    assert_eq!(plain("refby=y").field, FieldRef::Named("refby".to_owned()));
}

#[test]
fn ref_all_parses_direction_quantifier_field_and_inner_condition() {
    let condition = ref_condition("ref.all(blocked_by).status=resolved");

    assert_eq!(condition.dir, Dir::Ref);
    assert_eq!(condition.quant, Quant::All);
    assert_eq!(condition.field, RefField::Named("blocked_by".to_owned()));
    let inner = condition.inner.expect("ref.all always carries a .EXPR");
    assert_eq!(inner.field, FieldRef::Named("status".to_owned()));
    assert_eq!(inner.op, Op::Eq);
    assert_eq!(inner.items, vec![Item::Literal("resolved".to_owned())]);
}

#[test]
fn refby_any_parses_with_the_refby_direction() {
    let condition = ref_condition("refby.any(sources).kind=research");

    assert_eq!(condition.dir, Dir::RefBy);
    assert_eq!(condition.quant, Quant::Any);
    assert_eq!(condition.field, RefField::Named("sources".to_owned()));
    assert!(condition.inner.is_some());
}

#[test]
fn omitting_the_inner_expr_tests_only_whether_an_arrow_exists() {
    let any = ref_condition("ref.any(blocked_by)");
    assert_eq!(any.quant, Quant::Any);
    assert!(any.inner.is_none());

    let none = ref_condition("refby.none(sources)");
    assert_eq!(none.quant, Quant::None);
    assert!(none.inner.is_none());
}

#[test]
fn dollar_body_is_a_valid_f_in_either_direction() {
    assert_eq!(ref_condition("ref.any($body)").field, RefField::Body);
    assert_eq!(ref_condition("refby.any($body)").field, RefField::Body);
}

#[test]
fn ref_all_without_an_expression_is_an_error_with_a_hint() {
    let err = parse("ref.all(blocked_by)").unwrap_err();
    let QueryError::Syntax { hint, .. } = &err else {
        panic!("expected a syntax error, got {err}");
    };
    assert_eq!(
        hint.as_deref(),
        Some("use ref.any(f) or ref.none(f)"),
        "{err}"
    );
}

#[test]
fn refby_all_without_an_expression_is_an_error_with_a_hint_too() {
    let err = parse("refby.all(sources)").unwrap_err();
    let QueryError::Syntax { hint, .. } = &err else {
        panic!("expected a syntax error, got {err}");
    };
    assert_eq!(
        hint.as_deref(),
        Some("use refby.any(f) or refby.none(f)"),
        "{err}"
    );
}

#[test]
fn another_ref_star_nested_inside_one_is_an_error() {
    let err = parse("ref.any(blocked_by).ref.any(sources)").unwrap_err();
    assert!(matches!(err, QueryError::Syntax { .. }), "{err}");
}

#[test]
fn another_refby_star_nested_inside_one_is_an_error() {
    let err = parse("ref.any(blocked_by).refby.any(sources)").unwrap_err();
    assert!(matches!(err, QueryError::Syntax { .. }), "{err}");
}

#[test]
fn anything_after_the_closing_paren_that_is_not_dot_expr_is_an_error() {
    let err = parse("ref.any(blocked_by)status=open").unwrap_err();
    assert!(matches!(err, QueryError::Syntax { .. }), "{err}");
}

#[test]
fn an_unknown_quantifier_is_an_error() {
    let err = parse("ref.every(blocked_by).status=open").unwrap_err();
    assert!(matches!(err, QueryError::Syntax { .. }), "{err}");
}

#[test]
fn a_missing_closing_paren_is_an_error() {
    let err = parse("ref.any(blocked_by.status=open").unwrap_err();
    assert!(matches!(err, QueryError::Syntax { .. }), "{err}");
}
