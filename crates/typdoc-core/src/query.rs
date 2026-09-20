//! The scalar half of the query language: a plain condition (`field op value`), its escaping
//! and glob, the pseudo-fields every document carries, coercion by the field's type, and the
//! rules for absence and negation. `ref.*` and `refby.*` conditions are not parsed here: an
//! expression that starts one fails with a plain syntax error, since a field name never
//! contains `.`.
//!
//! `parse` turns one `--where`/`--if`-style expression into a [`Condition`], checking nothing
//! but the expression's own shape. `evaluate` checks that condition against one schema and one
//! document: unknown fields, values that do not fit their field's type, and an ordering
//! comparison on a field that is not `number`, `date` or `datetime`, are reported there, since
//! only there is the field's declared type known.

use std::cmp::Ordering as CmpOrdering;

use chrono::{DateTime, FixedOffset, NaiveDate};

use crate::coerce;
use crate::document::{Document, Value};
use crate::schema::{FieldType, Resolved};

/// The field a condition names: one of the six pseudo-fields every document carries, or a name
/// looked up in the schema of the document being tested.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldRef {
    Path,
    Key,
    Code,
    Collection,
    Schema,
    Namespace,
    Named(String),
}

/// A comparison operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

impl Op {
    fn is_ordering(self) -> bool {
        matches!(self, Op::Lt | Op::Le | Op::Gt | Op::Ge)
    }
}

/// One alternative of a condition's value, once escaping is resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Item {
    /// The bare, unescaped `*` alone: "present", and, on the whole value, not empty.
    Present,
    /// No unescaped `*`: matched by exact text.
    Literal(String),
    /// Text split at each unescaped `*`; matched as a wildcard, each `*` standing for any text
    /// (including none) between the segments around it.
    Glob(Vec<String>),
}

/// A plain condition, parsed but not yet checked against a schema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Condition {
    pub field: FieldRef,
    pub op: Op,
    pub items: Vec<Item>,
}

/// Why an expression could not be parsed or type-checked. Every variant is one of the errors
/// the grammar names (`docs/design.md`, Query).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryError {
    /// The expression does not fit the grammar. `hint` is filled when the likely cause is a
    /// space around the operator.
    Syntax {
        message: String,
        hint: Option<String>,
    },
    /// A field name that is no pseudo-field and no field of the schema this condition was
    /// checked against.
    UnknownField(String),
    /// An ordering comparison (`<`, `<=`, `>`, `>=`) on a field whose type is not `number`,
    /// `date` or `datetime`.
    OrderingNotAllowed { field: String, kind: String },
    /// A value that cannot be read as the field's type.
    CannotCoerce {
        field: String,
        value: String,
        kind: String,
    },
    /// A value that is not one of the field's declared `enum` values.
    NotAnEnumValue { field: String, value: String },
}

impl std::fmt::Display for QueryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QueryError::Syntax { message, hint } => {
                write!(f, "{message}")?;
                if let Some(hint) = hint {
                    write!(f, ": {hint}")?;
                }
                Ok(())
            }
            QueryError::UnknownField(name) => {
                write!(f, "no schema in scope declares the field `{name}`")
            }
            QueryError::OrderingNotAllowed { field, kind } => write!(
                f,
                "`{field}` is a `{kind}`: an ordering comparison applies to number, date and datetime only"
            ),
            QueryError::CannotCoerce { field, value, kind } => {
                write!(f, "`{value}` is not a `{kind}`, the type of `{field}`")
            }
            QueryError::NotAnEnumValue { field, value } => {
                write!(f, "`{value}` is not one of the values `{field}` allows")
            }
        }
    }
}

impl std::error::Error for QueryError {}

fn syntax(message: String) -> QueryError {
    QueryError::Syntax {
        message,
        hint: None,
    }
}

// ---------------------------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------------------------

/// Parses one `--where`/`--if`-style expression: `field op value`. An empty value is always an
/// error, since this is the grammar `--where` and `--if` share (`--set` has its own rule for an
/// empty value, and the read core has no write command to use it in).
pub fn parse(expr: &str) -> Result<Condition, QueryError> {
    let first = parse_strict(expr);
    if let Err(QueryError::Syntax {
        message,
        hint: None,
    }) = &first
        && expr.contains(' ')
    {
        let stripped: String = expr.chars().filter(|c| *c != ' ').collect();
        if stripped != expr && parse_strict(&stripped).is_ok() {
            return Err(QueryError::Syntax {
                message: message.clone(),
                hint: Some(format!("did you mean {stripped}?")),
            });
        }
    }
    first
}

/// One field name on its own, read the same way a condition's own field is (`[A-Za-z_]
/// [A-Za-z0-9_-]*`, or a pseudo-field): for `list`'s `--sort field[:asc|:desc]`, whose field
/// part names a sort key and not a condition.
pub fn parse_field(name: &str) -> Result<FieldRef, QueryError> {
    let (parsed, rest) = take_field_name(name)?;
    if !rest.is_empty() {
        return Err(syntax(format!(
            "`{name}` is not a field name: a field name is [A-Za-z_][A-Za-z0-9_-]*"
        )));
    }
    Ok(to_field_ref(parsed))
}

fn parse_strict(expr: &str) -> Result<Condition, QueryError> {
    let (name, rest) = take_field_name(expr)?;
    let (op, value) = take_operator(rest)?;
    let items = parse_value(value, op)?;
    Ok(Condition {
        field: to_field_ref(name),
        op,
        items,
    })
}

/// `[A-Za-z_][A-Za-z0-9_-]*` at the start of `expr`. Field names are ASCII, so scanning bytes
/// always stops on a character boundary.
fn take_field_name(expr: &str) -> Result<(&str, &str), QueryError> {
    let bytes = expr.as_bytes();
    if bytes.is_empty() {
        return Err(syntax("an expression cannot be empty".to_owned()));
    }
    if !(bytes[0].is_ascii_alphabetic() || bytes[0] == b'_') {
        return Err(syntax(format!("expected a field name, found `{expr}`")));
    }
    let mut end = 1;
    while end < bytes.len() {
        let b = bytes[end];
        if b.is_ascii_alphanumeric() || b == b'_' || b == b'-' {
            end += 1;
        } else {
            break;
        }
    }
    Ok((&expr[..end], &expr[end..]))
}

/// The longest of `!=`, `<=`, `>=`, `=`, `<`, `>` at the start of `rest`.
fn take_operator(rest: &str) -> Result<(Op, &str), QueryError> {
    const OPERATORS: [(&str, Op); 6] = [
        ("!=", Op::Ne),
        ("<=", Op::Le),
        (">=", Op::Ge),
        ("=", Op::Eq),
        ("<", Op::Lt),
        (">", Op::Gt),
    ];
    for (token, op) in OPERATORS {
        if let Some(value) = rest.strip_prefix(token) {
            return Ok((op, value));
        }
    }
    Err(syntax(format!(
        "expected an operator (!=, <=, >=, =, <, >), found `{rest}`"
    )))
}

fn to_field_ref(name: &str) -> FieldRef {
    match name {
        "path" => FieldRef::Path,
        "key" => FieldRef::Key,
        "code" => FieldRef::Code,
        "collection" => FieldRef::Collection,
        "schema" => FieldRef::Schema,
        "namespace" => FieldRef::Namespace,
        other => FieldRef::Named(other.to_owned()),
    }
}

/// `value = item { "," item }`, each item read for `\` escapes and unescaped `*`. An empty
/// value, or an empty alternative in a list (`k=a,`), is the same error: there is no text to
/// read as an item.
fn parse_value(value: &str, op: Op) -> Result<Vec<Item>, QueryError> {
    if value.is_empty() {
        return Err(syntax(
            "an empty value is an error in --where and --if".to_owned(),
        ));
    }
    let mut items = Vec::new();
    let mut segments: Vec<String> = vec![String::new()];
    let mut chars = value.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => match chars.next() {
                Some(escaped @ (',' | '*' | '\\')) => {
                    segments
                        .last_mut()
                        .expect("at least one segment")
                        .push(escaped);
                }
                Some(other) => {
                    return Err(syntax(format!(
                        "`\\{other}` is not a recognized escape: only `\\,`, `\\*` and `\\\\` are"
                    )));
                }
                None => {
                    return Err(syntax("a value cannot end with `\\`".to_owned()));
                }
            },
            '*' => segments.push(String::new()),
            ',' => {
                let finished = std::mem::replace(&mut segments, vec![String::new()]);
                items.push(finish_item(finished)?);
            }
            other => segments
                .last_mut()
                .expect("at least one segment")
                .push(other),
        }
    }
    items.push(finish_item(segments)?);
    if op.is_ordering() && items.len() > 1 {
        return Err(syntax(
            "an ordering comparison (<, <=, >, >=) takes one value".to_owned(),
        ));
    }
    Ok(items)
}

fn finish_item(segments: Vec<String>) -> Result<Item, QueryError> {
    if segments.len() == 1 {
        let text = segments.into_iter().next().expect("one segment");
        if text.is_empty() {
            return Err(syntax(
                "an empty value is an error in --where and --if".to_owned(),
            ));
        }
        Ok(Item::Literal(text))
    } else if segments.len() == 2 && segments[0].is_empty() && segments[1].is_empty() {
        Ok(Item::Present)
    } else {
        Ok(Item::Glob(segments))
    }
}

// ---------------------------------------------------------------------------------------------
// Evaluation
// ---------------------------------------------------------------------------------------------

/// What a condition's field resolves to against one schema: its type, whether it holds several
/// values, and, for an `enum`, the values it allows.
struct FieldPlan<'s> {
    kind: FieldType,
    is_array: bool,
    enum_values: Option<&'s [String]>,
}

fn resolve_field<'s>(field: &FieldRef, schema: &'s Resolved) -> Option<FieldPlan<'s>> {
    match field {
        FieldRef::Named(name) => {
            let field = schema.field(name)?;
            Some(FieldPlan {
                kind: field.kind.clone(),
                is_array: matches!(field.kind, FieldType::List | FieldType::RefList),
                enum_values: field.values.as_deref(),
            })
        }
        // Every pseudo-field is a plain string: present whenever it applies to the document,
        // never a list, never an enum.
        _ => Some(FieldPlan {
            kind: FieldType::String,
            is_array: false,
            enum_values: None,
        }),
    }
}

fn field_display_name(field: &FieldRef) -> String {
    match field {
        FieldRef::Path => "path".to_owned(),
        FieldRef::Key => "key".to_owned(),
        FieldRef::Code => "code".to_owned(),
        FieldRef::Collection => "collection".to_owned(),
        FieldRef::Schema => "schema".to_owned(),
        FieldRef::Namespace => "namespace".to_owned(),
        FieldRef::Named(name) => name.clone(),
    }
}

/// The value a document holds for `field`, own fields and pseudo-fields alike, as one `Value`.
/// `None` is absence: the field is not in `doc.fields`, or the pseudo-field does not apply to
/// this document (`key`, `code`, on one with no code). `pub(crate)` so `list` (`project.rs`) can
/// read a sort key's value the same way a condition does, without a second lookup.
pub(crate) fn field_value(field: &FieldRef, doc: &Document) -> Option<Value> {
    match field {
        FieldRef::Path => Some(Value::Text(doc.path.clone())),
        FieldRef::Key => doc.key.clone().map(Value::Text),
        FieldRef::Code => doc.code.clone().map(Value::Text),
        FieldRef::Collection => Some(Value::Text(doc.collection.clone())),
        FieldRef::Schema => Some(Value::Text(doc.schema.clone())),
        FieldRef::Namespace => Some(Value::Text(doc.namespace.clone())),
        FieldRef::Named(name) => doc
            .fields
            .iter()
            .find(|(found, _)| found == name)
            .map(|(_, value)| value.clone()),
    }
}

/// Checks one condition against `schema`, then evaluates it against `doc`: absence, `!=` as
/// exactly NOT `=`, and, for `<`, `<=`, `>`, `>=`, a document without the field failing the
/// comparison. Filtering a list of candidate documents calls this once per document with the
/// schema each was read under; a plain condition's scope, when it spans more than one
/// collection, is for that caller to resolve field by field before calling this.
pub fn evaluate(
    condition: &Condition,
    schema: &Resolved,
    doc: &Document,
) -> Result<bool, QueryError> {
    let name = field_display_name(&condition.field);
    let plan = resolve_field(&condition.field, schema)
        .ok_or_else(|| QueryError::UnknownField(name.clone()))?;
    let value = field_value(&condition.field, doc);

    if condition.op.is_ordering() {
        let item = condition
            .items
            .first()
            .expect("parse leaves at least one item");
        return ordering_matches(condition.op, item, &plan.kind, value.as_ref(), &name);
    }

    let matchers: Vec<Matcher> = condition
        .items
        .iter()
        .map(|item| check_item(item, &plan.kind, plan.enum_values, &name))
        .collect::<Result<_, _>>()?;
    let matched = matches_positive(&matchers, plan.is_array, value.as_ref());
    Ok(match condition.op {
        Op::Eq => matched,
        Op::Ne => !matched,
        _ => unreachable!("ordering operators are handled above"),
    })
}

/// A value list's item, checked against a field's declared type: either a typed value to
/// compare exactly (`number`, `bool`, `date`, `datetime`), or one to match as text (every other
/// type, and every glob or bare `*`, which are never coerced).
enum Matcher {
    ExactValue(Value),
    Text(Item),
}

fn check_item(
    item: &Item,
    kind: &FieldType,
    enum_values: Option<&[String]>,
    field: &str,
) -> Result<Matcher, QueryError> {
    let Item::Literal(text) = item else {
        // A glob or a bare `*` is never coerced: it is matched against the value as written,
        // whatever the field's type (see the module doc).
        return Ok(Matcher::Text(item.clone()));
    };
    match kind {
        FieldType::Enum => {
            if let Some(values) = enum_values
                && !values.iter().any(|allowed| allowed == text)
            {
                return Err(QueryError::NotAnEnumValue {
                    field: field.to_owned(),
                    value: text.clone(),
                });
            }
            Ok(Matcher::Text(item.clone()))
        }
        FieldType::Number | FieldType::Bool | FieldType::Date | FieldType::Datetime => {
            match coerce::coerce(kind, &Value::Text(text.clone())) {
                Some(typed) => Ok(Matcher::ExactValue(typed)),
                None => Err(QueryError::CannotCoerce {
                    field: field.to_owned(),
                    value: text.clone(),
                    kind: kind.name().to_owned(),
                }),
            }
        }
        _ => Ok(Matcher::Text(item.clone())),
    }
}

fn matches_positive(matchers: &[Matcher], is_array: bool, value: Option<&Value>) -> bool {
    matchers.iter().any(|matcher| {
        if is_array {
            matches_array(matcher, value)
        } else {
            matches_scalar(matcher, value)
        }
    })
}

fn matches_scalar(matcher: &Matcher, value: Option<&Value>) -> bool {
    match matcher {
        Matcher::ExactValue(want) => value == Some(want),
        Matcher::Text(Item::Present) => value.is_some_and(|v| !value_is_empty(v)),
        Matcher::Text(item) => value
            .and_then(value_as_text)
            .is_some_and(|text| item_matches_text(item, &text)),
    }
}

fn matches_array(matcher: &Matcher, value: Option<&Value>) -> bool {
    match matcher {
        // Arrays are never number, bool, date or datetime typed; not reachable in practice.
        Matcher::ExactValue(_) => false,
        Matcher::Text(Item::Present) => value.is_some_and(|v| !value_is_empty(v)),
        Matcher::Text(item) => {
            let Some(Value::List(elements)) = value else {
                return false;
            };
            elements
                .iter()
                .any(|element| item_matches_text(item, element))
        }
    }
}

fn item_matches_text(item: &Item, text: &str) -> bool {
    match item {
        Item::Present => !text.is_empty(),
        Item::Literal(literal) => literal == text,
        Item::Glob(segments) => glob_matches(segments, text),
    }
}

/// Whether `text` matches a pattern split into `segments` at each unescaped `*`: a prefix, an
/// optional run of inner segments found in order, and a suffix.
fn glob_matches(segments: &[String], text: &str) -> bool {
    let last = segments.len() - 1;
    let Some(mut rest) = text.strip_prefix(segments[0].as_str()) else {
        return false;
    };
    for segment in &segments[1..last] {
        if segment.is_empty() {
            continue;
        }
        let Some(at) = rest.find(segment.as_str()) else {
            return false;
        };
        rest = &rest[at + segment.len()..];
    }
    rest.ends_with(segments[last].as_str())
}

fn value_is_empty(value: &Value) -> bool {
    match value {
        Value::Text(text) => text.is_empty(),
        Value::List(items) => items.is_empty(),
        Value::Number(_) | Value::Bool(_) | Value::Date(_) | Value::Datetime(_) => false,
    }
}

/// `value` rendered the way it was written, for a glob or a bare `*` to match against: `Date`
/// and `Datetime` already hold their original text, `Number` and `Bool` render canonically
/// (which, for a number written in scientific notation, may not equal what was written).
fn value_as_text(value: &Value) -> Option<String> {
    match value {
        Value::Text(text) | Value::Date(text) | Value::Datetime(text) => Some(text.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        Value::List(_) => None,
    }
}

/// `<`, `<=`, `>`, `>=`: a document without the field fails, and a stored value that does not
/// fit its declared type (kept as written) is treated the same as absent, since it is not a
/// `number`, `date` or `datetime` to compare against. A `date`-shaped value compares against a
/// `datetime` field's date part, per the design's Comparisons paragraph; the reverse (a
/// datetime-shaped value against a `date` field) is not offered, and fails to coerce.
fn ordering_matches(
    op: Op,
    item: &Item,
    kind: &FieldType,
    value: Option<&Value>,
    field: &str,
) -> Result<bool, QueryError> {
    // Checked before the item's shape: a field whose type is not orderable at all is the more
    // useful error to report, whatever the value looks like.
    if !matches!(
        kind,
        FieldType::Number | FieldType::Date | FieldType::Datetime
    ) {
        return Err(QueryError::OrderingNotAllowed {
            field: field.to_owned(),
            kind: kind.name().to_owned(),
        });
    }
    let Item::Literal(text) = item else {
        return Err(QueryError::CannotCoerce {
            field: field.to_owned(),
            value: display_item(item),
            kind: kind.name().to_owned(),
        });
    };
    let Some(value) = value else {
        return Ok(false);
    };
    let ordering = match kind {
        FieldType::Number => {
            let Value::Number(have) = value else {
                return Ok(false);
            };
            let Some(want) = coerce::coerce(kind, &Value::Text(text.clone())) else {
                return Err(QueryError::CannotCoerce {
                    field: field.to_owned(),
                    value: text.clone(),
                    kind: "number".to_owned(),
                });
            };
            let Value::Number(want) = want else {
                unreachable!("coerce of a number field gives a number")
            };
            let (Some(have), Some(want)) = (have.as_f64(), want.as_f64()) else {
                return Ok(false);
            };
            have.partial_cmp(&want)
        }
        FieldType::Date => {
            let Value::Date(have) = value else {
                return Ok(false);
            };
            let Some(want) = parse_date(text) else {
                return Err(QueryError::CannotCoerce {
                    field: field.to_owned(),
                    value: text.clone(),
                    kind: "date".to_owned(),
                });
            };
            let have = parse_date(have).expect("a stored date already fits its type");
            have.partial_cmp(&want)
        }
        FieldType::Datetime => {
            let Value::Datetime(have) = value else {
                return Ok(false);
            };
            if let Some(want) = parse_datetime(text) {
                let have = parse_datetime(have).expect("a stored datetime already fits its type");
                have.partial_cmp(&want)
            } else if let Some(want) = parse_date(text) {
                let have = parse_date(&have[..10]).expect("a stored datetime starts with a date");
                have.partial_cmp(&want)
            } else {
                return Err(QueryError::CannotCoerce {
                    field: field.to_owned(),
                    value: text.clone(),
                    kind: "datetime".to_owned(),
                });
            }
        }
        _ => unreachable!("checked above"),
    };
    Ok(compare(op, ordering))
}

fn compare(op: Op, ordering: Option<CmpOrdering>) -> bool {
    let Some(ordering) = ordering else {
        return false;
    };
    match op {
        Op::Lt => ordering.is_lt(),
        Op::Le => ordering.is_le(),
        Op::Gt => ordering.is_gt(),
        Op::Ge => ordering.is_ge(),
        Op::Eq | Op::Ne => unreachable!("equality does not reach ordering_matches"),
    }
}

/// A calendar date read the same way a `date` field's value is, for `list`'s `--sort` (a date
/// compares by value, the Sorting table under `list`). `pub(crate)` for the same reason as
/// `field_value`.
pub(crate) fn parse_date(text: &str) -> Option<NaiveDate> {
    match coerce::coerce(&FieldType::Date, &Value::Text(text.to_owned())) {
        Some(Value::Date(text)) => NaiveDate::parse_from_str(&text, "%Y-%m-%d").ok(),
        _ => None,
    }
}

/// A datetime read the same way a `datetime` field's value is, for `list`'s `--sort`.
/// `pub(crate)` for the same reason as `field_value`.
pub(crate) fn parse_datetime(text: &str) -> Option<DateTime<FixedOffset>> {
    match coerce::coerce(&FieldType::Datetime, &Value::Text(text.to_owned())) {
        Some(Value::Datetime(text)) => DateTime::parse_from_rfc3339(&text).ok(),
        _ => None,
    }
}

fn display_item(item: &Item) -> String {
    match item {
        Item::Present => "*".to_owned(),
        Item::Literal(text) => text.clone(),
        Item::Glob(segments) => segments.join("*"),
    }
}
