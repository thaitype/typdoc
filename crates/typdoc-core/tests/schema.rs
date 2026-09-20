//! The schema format as declared data: what a schema file says, and nothing decided from it.
//! The schemas are the design's own examples.

use std::collections::BTreeMap;

use serde_json::json;
use typdoc_core::{Auto, FieldType, Schema, Target};

const BASE_TICKET: &str = r#"{
  "name": "base-ticket",
  "fields": {
    "title":  { "type": "string", "required": true },
    "status": {
      "type": "enum",
      "values": ["open", "claimed", "resolved", "closed"],
      "default": "open",
      "transitions": { "open": ["claimed", "closed"], "claimed": ["resolved", "open", "closed"] }
    },
    "blocked_by": { "type": "ref[]", "target": "*", "acyclic": true, "default": [] },
    "created_at": { "type": "datetime", "auto": "create" },
    "updated_at": { "type": "datetime", "auto": "update" }
  }
}"#;

const WAYFINDER: &str = r#"{
  "name": "wayfinder",
  "code": "WF",
  "extends": "./base-ticket.json",
  "fields": {
    "kind":    { "type": "enum", "values": ["research", "prototype", "grilling", "task", "feature"], "required": true },
    "owner":   { "type": "string" },
    "context": { "type": "ref[]", "target": ["memory::precedent", "memory::learning"] }
  }
}"#;

fn parse(text: &str) -> Schema {
    Schema::parse(text).expect("a schema")
}

#[test]
fn a_schema_has_a_name_a_code_a_parent_and_fields() {
    let schema = parse(WAYFINDER);

    assert_eq!(schema.name, "wayfinder");
    assert_eq!(schema.code.as_deref(), Some("WF"));
    assert_eq!(schema.extends.as_deref(), Some("./base-ticket.json"));
    assert_eq!(
        schema.fields.keys().map(String::as_str).collect::<Vec<_>>(),
        ["context", "kind", "owner"]
    );
}

#[test]
fn a_schema_with_no_code_and_no_parent_says_neither() {
    let schema = parse(BASE_TICKET);

    assert_eq!(schema.code, None);
    assert_eq!(schema.extends, None);
}

#[test]
fn every_option_of_a_field_is_read_as_declared() {
    let schema = parse(BASE_TICKET);

    let title = &schema.fields["title"];
    assert_eq!(title.kind, FieldType::String);
    assert!(title.is_required());

    let status = &schema.fields["status"];
    assert_eq!(status.kind, FieldType::Enum);
    assert!(!status.is_required());
    assert_eq!(status.default, Some(json!("open")));
    assert_eq!(
        status.transitions,
        Some(BTreeMap::from([
            (
                "open".to_owned(),
                vec!["claimed".to_owned(), "closed".to_owned()]
            ),
            (
                "claimed".to_owned(),
                vec![
                    "resolved".to_owned(),
                    "open".to_owned(),
                    "closed".to_owned()
                ]
            ),
        ]))
    );

    let blocked_by = &schema.fields["blocked_by"];
    assert_eq!(blocked_by.kind, FieldType::RefList);
    assert_eq!(blocked_by.target, Some(Target::Any));
    assert!(blocked_by.is_acyclic());
    assert_eq!(blocked_by.default, Some(json!([])));

    assert_eq!(schema.fields["created_at"].auto, Some(Auto::Create));
    assert_eq!(schema.fields["updated_at"].auto, Some(Auto::Update));
}

#[test]
fn the_values_of_an_enum_keep_the_order_they_are_written_in() {
    let schema = parse(BASE_TICKET);

    assert_eq!(
        schema.fields["status"].values.as_deref(),
        Some(&["open", "claimed", "resolved", "closed"].map(String::from)[..])
    );
    let wayfinder = parse(WAYFINDER);
    assert_eq!(
        wayfinder.fields["kind"].values.as_deref(),
        Some(&["research", "prototype", "grilling", "task", "feature"].map(String::from)[..])
    );
}

#[test]
fn a_target_is_any_or_a_list_of_names_bare_or_qualified() {
    let schema = parse(WAYFINDER);

    assert_eq!(
        schema.fields["context"].target,
        Some(Target::Schemas(vec![
            "memory::precedent".to_owned(),
            "memory::learning".to_owned()
        ]))
    );
    assert_eq!(schema.fields["owner"].target, None);
}

#[test]
fn every_field_type_of_the_design_is_read() {
    let schema = parse(
        r#"{ "name": "all", "fields": {
            "a": { "type": "string" },   "b": { "type": "number" },
            "c": { "type": "bool" },     "d": { "type": "date" },
            "e": { "type": "datetime" }, "f": { "type": "enum" },
            "g": { "type": "list" },     "h": { "type": "ref" },
            "i": { "type": "ref[]" }
        } }"#,
    );

    let kinds: Vec<&FieldType> = schema.fields.values().map(|field| &field.kind).collect();
    assert_eq!(
        kinds,
        [
            &FieldType::String,
            &FieldType::Number,
            &FieldType::Bool,
            &FieldType::Date,
            &FieldType::Datetime,
            &FieldType::Enum,
            &FieldType::List,
            &FieldType::Ref,
            &FieldType::RefList,
        ]
    );
}

#[test]
fn override_and_moves_are_read() {
    let schema = parse(
        r#"{ "name": "n", "fields": {
            "was": { "type": "list", "auto": "moves" },
            "kind": { "type": "enum", "values": ["a"], "override": true }
        } }"#,
    );

    assert_eq!(schema.fields["was"].auto, Some(Auto::Moves));
    assert!(schema.fields["kind"].is_override());
    assert!(!schema.fields["was"].is_override());
}

#[test]
fn a_name_the_format_does_not_have_is_kept_and_not_refused() {
    let schema = parse(
        r#"{ "name": "n", "fields": {
            "a": { "type": "integer" },
            "b": { "type": "ref", "target": "learning", "auto": "sometimes" }
        } }"#,
    );

    assert_eq!(schema.fields["a"].kind, FieldType::Other("integer".into()));
    assert_eq!(
        schema.fields["b"].target,
        Some(Target::Other("learning".into()))
    );
    assert_eq!(
        schema.fields["b"].auto,
        Some(Auto::Other("sometimes".into()))
    );
}

#[test]
fn a_schema_that_has_not_the_shape_of_the_format_cannot_be_read() {
    for text in [
        "",
        "[]",
        r#"{ "fields": {} }"#,
        r#"{ "name": "n" }"#,
        r#"{ "name": "n", "fields": [] }"#,
        r#"{ "name": "n", "fields": { "a": {} } }"#,
        r#"{ "name": "n", "fields": { "a": { "type": "enum", "values": "a" } } }"#,
        r#"{ "name": "n", "fields": { "a": { "type": 3 } } }"#,
    ] {
        assert!(Schema::parse(text).is_err(), "{text:?}");
    }
}

/// Ticket 5 read a wrongly typed option strictly, which crashed the whole read before
/// `schema.valid` (ticket 9) could report it, unlike an unknown type, `auto` or `target` name,
/// already read tolerantly for the same reason. Decided here: `required` (and `acyclic` and
/// `override`, the format's other boolean options) are relaxed the same way, so a schema like
/// this one is read, with the field's `required` false, and `schema.valid` is what reports it.
#[test]
fn a_boolean_option_written_as_something_else_is_read_tolerantly_and_not_refused() {
    let schema =
        parse(r#"{ "name": "n", "fields": { "a": { "type": "string", "required": "yes" } } }"#);

    assert!(!schema.fields["a"].is_required());
}
