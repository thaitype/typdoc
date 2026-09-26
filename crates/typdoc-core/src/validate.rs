//! The rules `validate` checks on one document's frontmatter, and the merge of rule levels
//! (SPC-1). An always-on rule is never merged: its level is `error`, and
//! `config.rule-always-on` refuses a project that configures one.

use crate::config::{Level, Rules};
use crate::document::Value;
use crate::frontmatter;
use crate::lines::Position;
use crate::schema::{FieldType, Resolved};

/// What a validate report covers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidateScope {
    All,
    Paths,
    Schemas,
}

/// The level a finding is reported at. `Info` comes only from `--audit`, which reports a rule
/// set to `off` rather than skipping it (SPC-2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Info,
    Warn,
    Error,
}

/// One thing `validate` found, in the finding shape of SPC-12. A finding about a file that is
/// not a document carries no `collection` or `key`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub level: Severity,
    pub rule: &'static str,
    pub message: String,
    pub path: String,
    pub namespace: Option<String>,
    pub collection: Option<String>,
    pub key: Option<String>,
    pub field: Option<String>,
    pub position: Option<Position>,
}

pub struct DocName<'a> {
    pub path: &'a str,
    pub namespace: &'a str,
    pub collection: &'a str,
    pub key: Option<&'a str>,
}

/// The order SPC-12 guarantees.
pub fn order(findings: &mut [Finding]) {
    findings.sort_by(|a, b| {
        (&a.path, a.position, a.rule, &a.message).cmp(&(&b.path, b.position, b.rule, &b.message))
    });
}

/// A block that cannot be parsed stops here: no other rule is evaluated for that file (SPC-1). A
/// file with no block and one with an empty block both reach the field checks, with no fields.
///
/// Under `--audit` a file with no block never reaches here (SPC-12); `audit` is passed so that a
/// `frontmatter.unknown` set to `off` is reported at `info`.
pub fn check_document(
    text: &str,
    schema: &Resolved,
    global: &Rules,
    collection: &Rules,
    strict: bool,
    audit: bool,
    name: &DocName,
) -> Vec<Finding> {
    let block = match frontmatter::block(text) {
        Ok(block) => block,
        Err(message) => {
            return vec![finding(
                name,
                Severity::Error,
                "frontmatter.parse",
                None,
                message,
            )];
        }
    };
    let fields = match block {
        None => Vec::new(),
        Some(block) => match frontmatter::fields(block, schema) {
            Ok(fields) => fields,
            Err(message) => {
                return vec![finding(
                    name,
                    Severity::Error,
                    "frontmatter.parse",
                    None,
                    message,
                )];
            }
        },
    };
    let mut findings = Vec::new();
    for (field_name, field) in schema.fields() {
        if field.is_required() && !fields.iter().any(|(written, _)| written == field_name) {
            findings.push(finding(
                name,
                Severity::Error,
                "frontmatter.types",
                Some(field_name),
                format!("the field `{field_name}` is required and is missing"),
            ));
        }
    }
    let unknown_level = effective_level(
        Level::Warn,
        "frontmatter.unknown",
        global,
        collection,
        strict,
        audit,
    );
    for (field_name, value) in &fields {
        match schema.field(field_name) {
            None => {
                if let Some(level) = unknown_level {
                    findings.push(finding(
                        name,
                        level,
                        "frontmatter.unknown",
                        Some(field_name),
                        format!("the field `{field_name}` is not a field of the schema"),
                    ));
                }
            }
            Some(field) if !crate::coerce::fits(&field.kind, value) => {
                findings.push(finding(
                    name,
                    Severity::Error,
                    "frontmatter.types",
                    Some(field_name),
                    format!(
                        "the field `{field_name}` does not fit its type `{}`: {}",
                        field.kind.name(),
                        display_value(value)
                    ),
                ));
            }
            Some(field) if field.kind == FieldType::Enum => {
                // A field written with no value is checked against the schema's `values` the
                // same as one written `''` is: both are the empty text.
                let text = match value {
                    Value::Text(text) => Some(text.as_str()),
                    Value::Empty => Some(""),
                    _ => None,
                };
                if let (Some(text), Some(values)) = (text, &field.values)
                    && !values.iter().any(|allowed| allowed == text)
                {
                    findings.push(finding(
                        name,
                        Severity::Error,
                        "frontmatter.types",
                        Some(field_name),
                        format!(
                            "the field `{field_name}` is not one of the schema's values: `{text}`"
                        ),
                    ));
                }
            }
            Some(_) => {}
        }
    }
    findings
}

/// Checked on write only, since a read has no value to compare against (SPC-1). A field is
/// checked only when its typed value changed and both sides have it.
///
/// A value the map does not name as a source may not change at all: a map from a value to its
/// next values has nothing for a value it does not list.
pub fn check_transitions(
    schema: &Resolved,
    before: &[(String, Value)],
    after: &[(String, Value)],
    name: &DocName,
) -> Vec<Finding> {
    let mut findings = Vec::new();
    for (field_name, field) in schema.fields() {
        let Some(map) = &field.transitions else {
            continue;
        };
        let Some(from) = find_field(before, field_name) else {
            continue;
        };
        let Some(to) = find_field(after, field_name) else {
            continue;
        };
        if from == to {
            continue;
        }
        // A value that is not text-shaped (a list where a scalar was expected, say) does not fit
        // an `enum` field's type at all: `frontmatter.types` already reports that on its own, and
        // there is no "value" here for a transition to be about.
        let (Some(from_text), Some(to_text)) = (enum_text(from), enum_text(to)) else {
            continue;
        };
        let allowed = map
            .get(from_text)
            .is_some_and(|nexts| nexts.iter().any(|next| next == to_text));
        if !allowed {
            findings.push(finding(
                name,
                Severity::Error,
                "frontmatter.transitions",
                Some(field_name),
                format!("transition not allowed: {from_text} -> {to_text}"),
            ));
        }
    }
    findings
}

fn find_field<'a>(fields: &'a [(String, Value)], name: &str) -> Option<&'a Value> {
    fields
        .iter()
        .find(|(found, _)| found == name)
        .map(|(_, value)| value)
}

fn enum_text(value: &Value) -> Option<&str> {
    match value {
        Value::Text(text) => Some(text.as_str()),
        Value::Empty => Some(""),
        _ => None,
    }
}

/// `None` is `off`. Under `audit` an `off` rule is reported at `Info` (SPC-2), and `strict`
/// never raises it: an `off` rule has no `warn` to raise.
pub(crate) fn effective_level(
    default: Level,
    rule: &str,
    global: &Rules,
    collection: &Rules,
    strict: bool,
    audit: bool,
) -> Option<Severity> {
    let mut level = default;
    if let Some(set) = global.get(rule).and_then(|setting| setting.level) {
        level = set;
    }
    if let Some(set) = collection.get(rule).and_then(|setting| setting.level) {
        level = set;
    }
    match level {
        Level::Off if audit => Some(Severity::Info),
        Level::Off => None,
        Level::Warn if strict => Some(Severity::Error),
        Level::Warn => Some(Severity::Warn),
        Level::Error => Some(Severity::Error),
    }
}

#[allow(
    clippy::too_many_arguments,
    reason = "a finding has this many parts; grouping them
    loses the name of each at the call site, which is what catches a part passed in the wrong
    place"
)]
fn build_finding(
    level: Severity,
    rule: &'static str,
    path: &str,
    namespace: Option<&str>,
    collection: Option<&str>,
    key: Option<&str>,
    field: Option<&str>,
    position: Option<Position>,
    message: String,
) -> Finding {
    Finding {
        level,
        rule,
        message,
        path: path.to_owned(),
        namespace: namespace.map(str::to_owned),
        collection: collection.map(str::to_owned),
        key: key.map(str::to_owned),
        field: field.map(str::to_owned),
        position,
    }
}

/// A finding with no position: `frontmatter.parse` keeps none, since the YAML reader's position
/// is imprecise for some errors, and nothing maps a frontmatter field or ref back to a line.
pub(crate) fn finding(
    name: &DocName,
    level: Severity,
    rule: &'static str,
    field: Option<&str>,
    message: String,
) -> Finding {
    build_finding(
        level,
        rule,
        name.path,
        Some(name.namespace),
        Some(name.collection),
        name.key,
        field,
        None,
        message,
    )
}

/// `col` is at a link's `[` or `!`, or at a definition's `[` (SPC-1).
pub(crate) fn finding_at(
    name: &DocName,
    level: Severity,
    rule: &'static str,
    field: Option<&str>,
    position: Position,
    message: String,
) -> Finding {
    build_finding(
        level,
        rule,
        name.path,
        Some(name.namespace),
        Some(name.collection),
        name.key,
        field,
        Some(position),
        message,
    )
}

pub(crate) fn schema_finding(path: &str, field: Option<&str>, message: String) -> Finding {
    build_finding(
        Severity::Error,
        "schema.valid",
        path,
        None,
        None,
        None,
        field,
        None,
        message,
    )
}

pub(crate) fn stray_file_finding(
    level: Severity,
    path: &str,
    namespace: &str,
    message: String,
) -> Finding {
    build_finding(
        level,
        "filename.pattern",
        path,
        Some(namespace),
        None,
        None,
        None,
        None,
        message,
    )
}

pub(crate) fn unreadable_finding(path: &str, namespace: Option<&str>, message: String) -> Finding {
    build_finding(
        Severity::Error,
        "files.unreadable",
        path,
        namespace,
        None,
        None,
        None,
        None,
        message,
    )
}

/// A rule of its own rather than `files.unreadable`, since a rule has one level: a leftover is
/// expected after a kill or a power cut and is not damage, so it is `warn` (SPC-10).
pub(crate) fn leftover_finding(path: &str, namespace: Option<&str>, message: String) -> Finding {
    build_finding(
        Severity::Warn,
        "files.leftover",
        path,
        namespace,
        None,
        None,
        None,
        None,
        message,
    )
}

/// No `collection`: which one is the document's is exactly what is wrong.
pub(crate) fn overlap_finding(path: &str, namespace: &str, message: String) -> Finding {
    build_finding(
        Severity::Error,
        "collections.overlap",
        path,
        Some(namespace),
        None,
        None,
        None,
        None,
        message,
    )
}

pub(crate) fn names_shadowed_finding(level: Severity, path: &str, message: String) -> Finding {
    build_finding(
        level,
        "names.shadowed",
        path,
        None,
        None,
        None,
        None,
        None,
        message,
    )
}

/// The key and the collection are not in doubt, only which document holds the key rightly.
pub(crate) fn duplicate_key_finding(
    path: &str,
    namespace: &str,
    collection: &str,
    key: &str,
    message: String,
) -> Finding {
    build_finding(
        Severity::Error,
        "keys.unique",
        path,
        Some(namespace),
        Some(collection),
        Some(key),
        None,
        None,
        message,
    )
}

/// About the state file, since nothing about one document is wrong.
pub(crate) fn state_missing_finding(
    path: &str,
    namespace: &str,
    collection: &str,
    message: String,
) -> Finding {
    build_finding(
        Severity::Error,
        "state.missing",
        path,
        Some(namespace),
        Some(collection),
        None,
        None,
        None,
        message,
    )
}

/// `Error`: a number guessed from it could be a key that already belongs to a document (SPC-8).
pub(crate) fn state_malformed_finding(
    path: &str,
    namespace: &str,
    collection: &str,
    message: String,
) -> Finding {
    build_finding(
        Severity::Error,
        "state.malformed",
        path,
        Some(namespace),
        Some(collection),
        None,
        None,
        None,
        message,
    )
}

/// `Warn`: allocation takes the larger of the two numbers, so the next one is still right
/// (SPC-8).
pub(crate) fn state_behind_finding(
    path: &str,
    namespace: &str,
    collection: &str,
    message: String,
) -> Finding {
    build_finding(
        Severity::Warn,
        "state.behind",
        path,
        Some(namespace),
        Some(collection),
        None,
        None,
        None,
        message,
    )
}

/// `Warn`, and it stops nothing: the entry is the only record that its numbers were issued
/// (SPC-8).
pub(crate) fn state_retired_finding(
    path: &str,
    namespace: &str,
    collection: &str,
    message: String,
) -> Finding {
    build_finding(
        Severity::Warn,
        "state.retired",
        path,
        Some(namespace),
        Some(collection),
        None,
        None,
        None,
        message,
    )
}

/// `Warn`, and it stops nothing (SPC-1). About the project, so `path` is the `.typdoc` folder.
pub(crate) fn collections_empty_finding(path: &str) -> Finding {
    build_finding(
        Severity::Warn,
        "collections.empty",
        path,
        None,
        None,
        None,
        None,
        None,
        "this project has no collections: nothing is configured to validate".to_owned(),
    )
}

fn display_value(value: &Value) -> String {
    match value {
        Value::Text(text) => format!("`{text}`"),
        // The same empty text a field written `''` displays as.
        Value::Empty => "``".to_owned(),
        Value::List(items) => format!("[{}]", items.join(", ")),
        Value::Number(number) => number.converted(),
        Value::Bool(flag) => flag.to_string(),
        Value::Date(text) | Value::Datetime(text) => text.clone(),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::config::RuleSetting;

    fn name<'a>() -> DocName<'a> {
        DocName {
            path: "a.md",
            namespace: "default",
            collection: "notes",
            key: None,
        }
    }

    fn schema(fields: &[(&str, &str, bool)]) -> Resolved {
        let text = json!({
            "name": "note",
            "fields": fields.iter().map(|(n, kind, required)| {
                (n.to_string(), json!({ "type": kind, "required": required }))
            }).collect::<serde_json::Map<_, _>>()
        })
        .to_string();
        let schema = crate::schema::Schema::parse(&text).expect("a schema");
        // A chain of one schema is what `load` builds for a collection with no `extends`, so
        // this goes through the same merge a real read does.
        crate::schema::merge(&[crate::schema::ChainLink {
            path: "schema.json".to_owned(),
            schema,
        }])
    }

    fn no_rules() -> Rules {
        Rules::new()
    }

    #[test]
    fn an_unclosed_block_is_frontmatter_parse_and_nothing_else_is_evaluated() {
        let text = "---\ntitle: never closed\n";
        let schema = schema(&[("title", "string", true)]);

        let findings = check_document(
            text,
            &schema,
            &no_rules(),
            &no_rules(),
            false,
            false,
            &name(),
        );

        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(findings[0].rule, "frontmatter.parse");
        assert_eq!(findings[0].level, Severity::Error);
        assert_eq!(findings[0].position, None);
    }

    #[test]
    fn invalid_yaml_in_a_closed_block_is_frontmatter_parse() {
        let text = "---\ntitle: [oops\n---\n";
        let schema = schema(&[("title", "string", false)]);

        let findings = check_document(
            text,
            &schema,
            &no_rules(),
            &no_rules(),
            false,
            false,
            &name(),
        );

        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(findings[0].rule, "frontmatter.parse");
        assert_eq!(
            findings[0].position, None,
            "a position known to be imprecise is never kept"
        );
    }

    #[test]
    fn a_duplicate_key_is_frontmatter_parse_with_no_position() {
        let text = "---\ntitle: a\ntitle: b\n---\n";
        let schema = schema(&[("title", "string", false)]);

        let findings = check_document(
            text,
            &schema,
            &no_rules(),
            &no_rules(),
            false,
            false,
            &name(),
        );

        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(findings[0].rule, "frontmatter.parse");
        assert_eq!(findings[0].position, None);
    }

    #[test]
    fn a_block_that_holds_a_second_yaml_document_is_frontmatter_parse_with_no_position() {
        // The closing fence must be a line that is exactly `---`; a line that starts with
        // `---` but carries more (here, a comment) is not it, and survives into the block as
        // ordinary YAML, where it opens a second document.
        let text = "---\ntitle: x\n--- # hi\nfoo: 1\n---\n\nBody.\n";
        let schema = schema(&[("title", "string", false)]);

        let findings = check_document(
            text,
            &schema,
            &no_rules(),
            &no_rules(),
            false,
            false,
            &name(),
        );

        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(findings[0].rule, "frontmatter.parse");
        assert_eq!(findings[0].position, None);
    }

    #[test]
    fn an_empty_block_and_no_block_both_reach_the_field_checks() {
        let schema = schema(&[("title", "string", true)]);

        let empty = check_document(
            "---\n---\n",
            &schema,
            &no_rules(),
            &no_rules(),
            false,
            false,
            &name(),
        );
        let none = check_document(
            "no frontmatter here\n",
            &schema,
            &no_rules(),
            &no_rules(),
            false,
            false,
            &name(),
        );

        for findings in [&empty, &none] {
            assert_eq!(findings.len(), 1, "{findings:?}");
            assert_eq!(findings[0].rule, "frontmatter.types");
            assert!(findings[0].message.contains("title"), "{findings:?}");
        }
    }

    #[test]
    fn a_missing_required_field_is_frontmatter_types() {
        let schema = schema(&[("title", "string", true)]);
        let text = "---\nother: x\n---\n";

        let findings = check_document(
            text,
            &schema,
            &no_rules(),
            &no_rules(),
            false,
            false,
            &name(),
        );

        let types: Vec<&Finding> = findings
            .iter()
            .filter(|f| f.rule == "frontmatter.types")
            .collect();
        assert_eq!(types.len(), 1, "{findings:?}");
        assert!(types[0].message.contains("title"));
        assert_eq!(types[0].field.as_deref(), Some("title"));
    }

    #[test]
    fn a_value_that_does_not_fit_its_type_is_frontmatter_types() {
        let schema = schema(&[("count", "number", false)]);
        let text = "---\ncount: abc\n---\n";

        let findings = check_document(
            text,
            &schema,
            &no_rules(),
            &no_rules(),
            false,
            false,
            &name(),
        );

        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(findings[0].rule, "frontmatter.types");
        assert_eq!(findings[0].field.as_deref(), Some("count"));
        assert_eq!(findings[0].position, None);
    }

    #[test]
    fn a_field_not_in_the_schema_is_frontmatter_unknown_at_the_default_warn_level() {
        let schema = schema(&[]);
        let text = "---\nextra: x\n---\n";

        let findings = check_document(
            text,
            &schema,
            &no_rules(),
            &no_rules(),
            false,
            false,
            &name(),
        );

        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(findings[0].rule, "frontmatter.unknown");
        assert_eq!(findings[0].level, Severity::Warn);
        assert_eq!(findings[0].field.as_deref(), Some("extra"));
    }

    #[test]
    fn frontmatter_unknown_off_in_the_collection_produces_no_finding() {
        let schema = schema(&[]);
        let text = "---\nextra: x\n---\n";
        let mut collection = Rules::new();
        collection.insert(
            "frontmatter.unknown".to_owned(),
            RuleSetting {
                level: Some(Level::Off),
                options: serde_json::Map::new(),
            },
        );

        let findings = check_document(
            text,
            &schema,
            &no_rules(),
            &collection,
            false,
            false,
            &name(),
        );

        assert_eq!(findings, Vec::new());
    }

    /// `strict` is set as well: an `off` rule has no `warn` for it to raise.
    #[test]
    fn frontmatter_unknown_off_is_info_under_audit() {
        let schema = schema(&[]);
        let text = "---\nextra: x\n---\n";
        let mut collection = Rules::new();
        collection.insert(
            "frontmatter.unknown".to_owned(),
            RuleSetting {
                level: Some(Level::Off),
                options: serde_json::Map::new(),
            },
        );

        let findings = check_document(text, &schema, &no_rules(), &collection, true, true, &name());

        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(findings[0].rule, "frontmatter.unknown");
        assert_eq!(findings[0].level, Severity::Info);
    }

    #[test]
    fn strict_raises_frontmatter_unknown_from_warn_to_error() {
        let schema = schema(&[]);
        let text = "---\nextra: x\n---\n";

        let findings = check_document(
            text,
            &schema,
            &no_rules(),
            &no_rules(),
            true,
            false,
            &name(),
        );

        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(findings[0].level, Severity::Error);
    }

    #[test]
    fn the_collection_overrides_the_project_wide_setting() {
        let schema = schema(&[]);
        let text = "---\nextra: x\n---\n";
        let mut global = Rules::new();
        global.insert(
            "frontmatter.unknown".to_owned(),
            RuleSetting {
                level: Some(Level::Error),
                options: serde_json::Map::new(),
            },
        );
        let mut collection = Rules::new();
        collection.insert(
            "frontmatter.unknown".to_owned(),
            RuleSetting {
                level: Some(Level::Warn),
                options: serde_json::Map::new(),
            },
        );

        let findings = check_document(text, &schema, &global, &collection, false, false, &name());

        assert_eq!(findings[0].level, Severity::Warn);
    }

    #[test]
    fn an_enum_value_not_in_the_schema_is_frontmatter_types() {
        let schema_text = json!({
            "name": "note",
            "fields": { "kind": { "type": "enum", "values": ["a", "b"] } }
        })
        .to_string();
        let parsed = crate::schema::Schema::parse(&schema_text).expect("a schema");
        let schema = crate::schema::merge(&[crate::schema::ChainLink {
            path: "schema.json".to_owned(),
            schema: parsed,
        }]);
        let text = "---\nkind: z\n---\n";

        let findings = check_document(
            text,
            &schema,
            &no_rules(),
            &no_rules(),
            false,
            false,
            &name(),
        );

        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(findings[0].rule, "frontmatter.types");
        assert!(findings[0].message.contains('z'), "{findings:?}");
    }

    #[test]
    fn order_sorts_by_path_then_position_then_rule_then_message() {
        let pos = |line, col| Some(Position { line, col });
        let mut findings = vec![
            f("b.md", None, "frontmatter.types", "m"),
            f("a.md", pos(2, 1), "x.y", "m"),
            f("a.md", None, "x.y", "m"),
            f("a.md", None, "a.a", "m"),
        ];

        order(&mut findings);

        let order: Vec<(&str, &str)> = findings.iter().map(|f| (f.path.as_str(), f.rule)).collect();
        assert_eq!(
            order,
            [
                ("a.md", "a.a"),
                ("a.md", "x.y"),
                ("a.md", "x.y"),
                ("b.md", "frontmatter.types")
            ]
        );
        assert_eq!(findings[2].position, pos(2, 1));
    }

    fn f(path: &str, position: Option<Position>, rule: &'static str, message: &str) -> Finding {
        Finding {
            level: Severity::Error,
            rule,
            message: message.to_owned(),
            path: path.to_owned(),
            namespace: Some("default".to_owned()),
            collection: Some("notes".to_owned()),
            key: None,
            field: None,
            position,
        }
    }

    fn status_schema() -> Resolved {
        let text = json!({
            "name": "ticket",
            "fields": {
                "status": {
                    "type": "enum",
                    "values": ["open", "claimed", "resolved", "closed"],
                    "transitions": {
                        "open": ["claimed", "closed"],
                        "claimed": ["resolved", "open", "closed"]
                    }
                }
            }
        })
        .to_string();
        let schema = crate::schema::Schema::parse(&text).expect("a schema");
        crate::schema::merge(&[crate::schema::ChainLink {
            path: "schema.json".to_owned(),
            schema,
        }])
    }

    fn status(value: &str) -> Vec<(String, Value)> {
        vec![("status".to_owned(), Value::Text(value.to_owned()))]
    }

    #[test]
    fn an_allowed_transition_is_silent() {
        let schema = status_schema();

        assert_eq!(
            check_transitions(&schema, &status("open"), &status("claimed"), &name()),
            Vec::new()
        );
    }

    #[test]
    fn a_transition_not_named_by_the_map_is_refused() {
        let schema = status_schema();

        let findings = check_transitions(&schema, &status("open"), &status("resolved"), &name());

        assert_eq!(
            findings,
            vec![Finding {
                field: Some("status".to_owned()),
                ..f(
                    "a.md",
                    None,
                    "frontmatter.transitions",
                    "transition not allowed: open -> resolved"
                )
            }]
        );
    }

    #[test]
    fn a_source_value_the_map_does_not_name_allows_no_transition() {
        let schema = status_schema();

        let findings = check_transitions(&schema, &status("resolved"), &status("open"), &name());

        assert_eq!(
            findings,
            vec![Finding {
                field: Some("status".to_owned()),
                ..f(
                    "a.md",
                    None,
                    "frontmatter.transitions",
                    "transition not allowed: resolved -> open"
                )
            }]
        );
    }

    #[test]
    fn a_field_left_unchanged_is_not_a_transition_even_off_the_map() {
        let schema = status_schema();

        assert_eq!(
            check_transitions(&schema, &status("resolved"), &status("resolved"), &name()),
            Vec::new()
        );
    }

    #[test]
    fn a_field_missing_on_either_side_has_nothing_to_compare() {
        let schema = status_schema();
        let empty = Vec::new();

        assert_eq!(
            check_transitions(&schema, &empty, &status("open"), &name()),
            Vec::new()
        );
        assert_eq!(
            check_transitions(&schema, &status("open"), &empty, &name()),
            Vec::new()
        );
    }

    #[test]
    fn a_field_the_schema_does_not_mark_transitions_allows_any_change() {
        let schema = schema(&[("title", "string", false)]);
        let before = vec![("title".to_owned(), Value::Text("a".to_owned()))];
        let after = vec![("title".to_owned(), Value::Text("b".to_owned()))];

        assert_eq!(
            check_transitions(&schema, &before, &after, &name()),
            Vec::new()
        );
    }
}
