//! The rules `validate` checks on one document's frontmatter, and the merge of rule levels:
//! typdoc's own default, then `validation.global`, then the collection's own `validation`,
//! with `--strict` raising a remaining `warn` to `error`. The always-on rules, `frontmatter.parse`
//! and `frontmatter.types`, are never merged: their level is always `error` (project rule and
//! ticket decision), and `config.rule-always-on` already refuses a project that tries to
//! configure them.

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

/// The level a finding is reported at. `frontmatter.parse` and `frontmatter.types` are always
/// `Error`; `frontmatter.unknown` is whatever the merge and `--strict` give it, and a rule
/// merged to `off` produces no finding at all, so `Off` never appears here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Warn,
    Error,
}

/// One thing `validate` found. Its name is `path` always, plus `namespace`, `collection` and
/// `key` when the file is a document, as the design's finding shape gives it (a `schema.valid`
/// finding is about a schema file or `config.json`, not a document, so it carries none of the
/// three; `filename.pattern` is about a file in a namespace that no collection matched, so it
/// carries `namespace` and not `collection` or `key`). `field` is present when the finding is
/// about one field, and a position when one is known.
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

/// The name of the document a finding is about, gathered once per document so the checks below
/// do not each have to thread every part of it through.
pub struct DocName<'a> {
    pub path: &'a str,
    pub namespace: &'a str,
    pub collection: &'a str,
    pub key: Option<&'a str>,
}

/// Sorts findings by `path`, then position (a finding with no position first), then `rule` and
/// `message`, the order the design guarantees.
pub fn order(findings: &mut [Finding]) {
    findings.sort_by(|a, b| {
        (&a.path, a.position, a.rule, &a.message).cmp(&(&b.path, b.position, b.rule, &b.message))
    });
}

/// The findings of one document: `frontmatter.parse` first, and, only when the block parsed,
/// `frontmatter.types` and `frontmatter.unknown`. A block that cannot be parsed stops here, as
/// the design asks: no other rule is evaluated for that file. A file with no block, and one
/// whose block is empty, both reach the field checks with no fields, since neither fails to
/// parse: the block that is absent is never handed to the reader, and the block that is empty
/// parses to nothing.
pub fn check_document(
    text: &str,
    schema: &Resolved,
    global: &Rules,
    collection: &Rules,
    strict: bool,
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
                if let (Value::Text(text), Some(values)) = (value, &field.values)
                    && !values.contains(text)
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

/// The level `rule` is reported at once the defaults, `validation.global` and the collection's
/// own `validation` are merged, with `strict` raising a remaining `warn` to `error`. `None` is
/// `off`: the rule produces no finding.
pub(crate) fn effective_level(
    default: Level,
    rule: &str,
    global: &Rules,
    collection: &Rules,
    strict: bool,
) -> Option<Severity> {
    let mut level = default;
    if let Some(set) = global.get(rule).and_then(|setting| setting.level) {
        level = set;
    }
    if let Some(set) = collection.get(rule).and_then(|setting| setting.level) {
        level = set;
    }
    match level {
        Level::Off => None,
        Level::Warn if strict => Some(Severity::Error),
        Level::Warn => Some(Severity::Warn),
        Level::Error => Some(Severity::Error),
    }
}

/// Every finding this crate's rules construct is built here, so the nine-field shape of
/// `Finding` is written out once; each rule below calls this with the parts it has and `None`
/// for the parts a file that is not a document (a schema file, `config.json`, a stray file)
/// does not carry.
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

/// A finding about `name`'s document, at `field` when it is about one field. No position-tracking
/// is built yet for any rule that uses this (frontmatter.parse: contract item 7, checked by
/// running; the others: nothing maps a field or a ref back to a line yet), so this is the one
/// place that constructs one. `pub(crate)` so `project.rs`'s ref rules (`refs.resolve`,
/// `refs.target`, `refs.codedByPath`, `refs.moved`, `refs.acyclic`) build their findings the same
/// way `frontmatter.types` and `frontmatter.unknown` do, rather than a second shape for refs.
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

/// A finding about `name`'s document at a known position: the body-side rules (`body.links`,
/// `body.anchors`) are the first this crate builds whose position is known, from `links::scan`'s
/// own line and column (`col` at the `[` or `!`, or at a definition's `[`, per the design's
/// Output paragraph).
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

/// A finding about a schema file or `config.json`: not a document, so it carries no
/// `namespace`, `collection` or `key`. Used by `schema.valid`.
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

/// A finding about a file in a namespace that no collection matched: not a document, so it
/// carries `namespace` but no `collection` or `key`. Used by `filename.pattern`.
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

/// A finding about a document matched by more than one collection (`collections.overlap`):
/// which collection is "the" collection of this document is exactly what is wrong, so it is
/// left out rather than guessed; the message names every collection that matched.
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

/// A finding about a name that is both a sibling namespace and an import alias
/// (`names.shadowed`): a fact about the config, not about one document, so it carries no
/// `namespace`, `collection` or `key`, the same shape `schema_finding` uses.
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

/// A finding about a document whose key is also used by another document in the same namespace
/// (`keys.unique`): the key and the collection are not in doubt, only which document is the
/// right holder of it, so both are carried and the message names the other document.
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

fn display_value(value: &Value) -> String {
    match value {
        Value::Text(text) => format!("`{text}`"),
        Value::List(items) => format!("[{}]", items.join(", ")),
        Value::Number(number) => number.to_string(),
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

        let findings = check_document(text, &schema, &no_rules(), &no_rules(), false, &name());

        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(findings[0].rule, "frontmatter.parse");
        assert_eq!(findings[0].level, Severity::Error);
        assert_eq!(findings[0].position, None);
    }

    #[test]
    fn invalid_yaml_in_a_closed_block_is_frontmatter_parse() {
        let text = "---\ntitle: [oops\n---\n";
        let schema = schema(&[("title", "string", false)]);

        let findings = check_document(text, &schema, &no_rules(), &no_rules(), false, &name());

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

        let findings = check_document(text, &schema, &no_rules(), &no_rules(), false, &name());

        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(findings[0].rule, "frontmatter.parse");
        assert_eq!(findings[0].position, None);
    }

    #[test]
    fn a_block_that_holds_a_second_yaml_document_is_frontmatter_parse_with_no_position() {
        // The closing fence must be a line that is exactly `---`; a line that starts with
        // `---` but carries more (here, a comment) is not it, and survives into the block as
        // ordinary YAML, where it opens a second document. `yaml_serde` gives no position for
        // this error (checked by running, ticket 18), so neither does the finding.
        let text = "---\ntitle: x\n--- # hi\nfoo: 1\n---\n\nBody.\n";
        let schema = schema(&[("title", "string", false)]);

        let findings = check_document(text, &schema, &no_rules(), &no_rules(), false, &name());

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
            &name(),
        );
        let none = check_document(
            "no frontmatter here\n",
            &schema,
            &no_rules(),
            &no_rules(),
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

        let findings = check_document(text, &schema, &no_rules(), &no_rules(), false, &name());

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

        let findings = check_document(text, &schema, &no_rules(), &no_rules(), false, &name());

        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(findings[0].rule, "frontmatter.types");
        assert_eq!(findings[0].field.as_deref(), Some("count"));
        assert_eq!(findings[0].position, None);
    }

    #[test]
    fn a_field_not_in_the_schema_is_frontmatter_unknown_at_the_default_warn_level() {
        let schema = schema(&[]);
        let text = "---\nextra: x\n---\n";

        let findings = check_document(text, &schema, &no_rules(), &no_rules(), false, &name());

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

        let findings = check_document(text, &schema, &no_rules(), &collection, false, &name());

        assert_eq!(findings, Vec::new());
    }

    #[test]
    fn strict_raises_frontmatter_unknown_from_warn_to_error() {
        let schema = schema(&[]);
        let text = "---\nextra: x\n---\n";

        let findings = check_document(text, &schema, &no_rules(), &no_rules(), true, &name());

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

        let findings = check_document(text, &schema, &global, &collection, false, &name());

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

        let findings = check_document(text, &schema, &no_rules(), &no_rules(), false, &name());

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
}
