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
/// `Error`; a configurable rule is whatever the merge and `--strict` give it. A rule merged to
/// `off` produces no finding at all in plain `validate`, so `Off` never appears here for a plain
/// run; under `--audit` a rule merged to `off` is reported as `Info` instead of being skipped
/// (design, Audit mode: "Rules set to `off`... Reported as `info`"), which is the only way `Info`
/// is ever produced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Info,
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
///
/// A file with no block at all is never handed to this function under `--audit` (the caller
/// lists it in `no_frontmatter` and evaluates nothing about it instead, design: "`--audit` does
/// not evaluate it"); `audit` is still threaded through to `frontmatter.unknown`'s level so a
/// project that turns it `off` still sees it as `info` here, the same as every other configurable
/// rule.
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

/// `frontmatter.transitions`, always on and checked on write (design, Validation rules table):
/// a field the schema marks `transitions` may not change to a value the map does not allow from
/// where it was. Only a write reaches this — the read core has no "before" to compare against,
/// which is why the design says "checked on write" rather than listing it among what `validate`
/// reports on an existing file.
///
/// **Only a field that actually changed is checked.** The same document reached another way
/// carries the same value, so "changed" is decided by equality of the typed [`Value`] `before`
/// and `after` hold for that field, not of its text — the same value promise `auto: update`'s
/// own stamping reuses (decision 20/ticket 5). A field neither side has (removed by `k=`, or
/// never in the document) is not a transition either: there is no "from" or no "to" to compare.
///
/// **A source value the map does not name has no allowed next value.** `transitions`'s own doc
/// says an *omitted option* allows any change; it does not say what an omitted *source value*
/// inside a present map means. Read literally, a map from a value to its allowed next values
/// simply has nothing for a value it does not list, so a value not named as a source is treated
/// as one this field may not leave. This is a choice where the design is silent, not a settled
/// reading, and it is why the design's own example (`base-ticket.json`, `docs/design.md`) never
/// transitions a document away from `resolved` or `closed`, only into them.
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

/// A value read as `enum` text: [`Value::Text`] as itself, [`Value::Empty`] as the empty text
/// (the same reading `check_document`'s own enum-membership check gives a bare field), anything
/// else `None`.
fn enum_text(value: &Value) -> Option<&str> {
    match value {
        Value::Text(text) => Some(text.as_str()),
        Value::Empty => Some(""),
        _ => None,
    }
}

/// The level `rule` is reported at once the defaults, `validation.global` and the collection's
/// own `validation` are merged, with `strict` raising a remaining `warn` to `error`. `None` is
/// `off`: the rule produces no finding, unless `audit` is set, in which case an `off` rule is
/// still checked and reported at `Info` instead of being skipped (design, Audit mode: "Rules set
/// to `off`... Reported as `info`"). `strict` never touches an `off` rule: raising `warn` to
/// `error` and reporting `off` as `info` are two different questions, and a rule that is `off`
/// has no `warn` for `strict` to raise.
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

/// A finding about a directory entry a `match` or a `namespaces` glob reached and the walk could
/// not read (`files.unreadable`): it was skipped, so it is no document and carries no
/// `collection` or `key`, and carries `namespace` when it was found inside one, the same shape
/// `stray_file_finding` uses. Always on, so its level is always `error`.
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

/// A finding about a leftover temp file a `match` or a `namespaces` glob reaches
/// (`files.leftover`): a rule of its own rather than `files.unreadable`'s, since one rule has
/// one level and this one is `warn` — a leftover is expected after a kill or a power cut and is
/// not itself damage (decision 4). Carries `namespace` when found inside one, the same shape
/// `unreadable_finding` uses.
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

/// A finding about a coded collection with documents in a namespace and no `last` recorded for
/// it there (`state.missing`): nothing about one document is wrong, so the file it is about is
/// the state file, not a document of the collection; `collection` names which one, and there is
/// no `key`, the same reasoning `collections.overlap` already gives a file matched by two.
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

/// A finding about a coded collection whose recorded `last` is present but not a usable whole
/// number (`state.malformed`): the record is there, unlike `state.missing`, so this is `Error`
/// the same way `state.missing` is — `new` and `mv --renumber` refuse to issue a number for the
/// same reason, a guessed one is a key that already belongs to a document.
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

/// A finding about a coded collection whose recorded `last` is a valid number lower than the
/// highest number that exists for it in the namespace (`state.behind`, `warn`): allocation
/// already takes the larger of the two, so the number issued next is still right, but the
/// record itself is telling a reader something untrue.
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

/// A finding about a state entry naming a collection this project no longer has
/// (`state.retired`, `warn`): kept rather than removed, since it is the only record that those
/// numbers were issued, and unlike its predecessor as a config error, this stops nothing —
/// reads included.
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
        // ordinary YAML, where it opens a second document. `yaml_serde` gives no position for
        // this error (checked by running, ticket 18), so neither does the finding.
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

    /// Design, Audit mode: "Rules set to `off`... Reported as `info`". The same `off` setting
    /// that produces nothing above produces one `info` finding once `audit` is set, and `strict`
    /// has nothing to raise it from, since `off` carries no `warn`.
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

    /// A schema of one enum field, `status`, with the design's own example transitions
    /// (`docs/design.md`, Schema format: `base-ticket.json`).
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
