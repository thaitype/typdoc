//! The query language (SPC-13). `parse` checks only an expression's shape. `evaluate` checks one
//! plain condition against one schema and one document, since only there is a field's declared
//! type known. A `ref.*`/`refby.*` condition reads more than one schema and more than one
//! document, so `Project` evaluates it.

use std::cmp::Ordering as CmpOrdering;

use chrono::{DateTime, FixedOffset, NaiveDate};

use crate::coerce;
use crate::document::{Document, Value};
use crate::schema::{FieldType, Resolved};

/// The field a condition names: a pseudo-field, or a field of the schema of the document being
/// tested.
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

    /// The result for something wholly absent, whatever the value: only `!=` holds (SPC-13). A
    /// dangling ref reached by `ref.*`/`refby.*` has no document to evaluate, so `Project` calls
    /// this directly.
    pub fn absent_result(self) -> bool {
        matches!(self, Op::Ne)
    }
}

/// One alternative of a condition's value, once escaping is resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Item {
    /// An unescaped `*` alone: the field is present and not empty.
    Present,
    /// No unescaped `*`: matched by exact text.
    Literal(String),
    /// Text split at each unescaped `*`; matched as a wildcard, each `*` standing for any text
    /// (including none) between the segments around it.
    Glob(Vec<String>),
}

/// A plain condition, parsed but not yet checked against a schema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlainCondition {
    pub field: FieldRef,
    pub op: Op,
    pub items: Vec<Item>,
}

/// One `--where`/`--if` expression: the document's own fields tested directly, or arrows
/// followed to other documents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Condition {
    Plain(PlainCondition),
    Ref(RefCondition),
}

/// `ref` (arrows leaving me) or `refby` (arrows pointing at me).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dir {
    Ref,
    RefBy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quant {
    All,
    Any,
    None,
}

/// `f` in `ref.*(f)`/`refby.*(f)`: a ref or ref[] field name, or the virtual field `$body`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefField {
    Named(String),
    Body,
}

impl RefField {
    /// The name a message or a `RefsReference.field` comparison uses for this field.
    pub fn name(&self) -> &str {
        match self {
            RefField::Body => "$body",
            RefField::Named(name) => name,
        }
    }
}

/// A `ref.*`/`refby.*` condition: which arrows (`dir`, `quant`, `field`) and, when given, the
/// plain condition each document reached must satisfy (`inner`). `inner: None` tests only
/// whether an arrow exists, so `quant` is never `All` then.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefCondition {
    pub dir: Dir,
    pub quant: Quant,
    pub field: RefField,
    pub inner: Option<PlainCondition>,
}

/// Why an expression could not be parsed or type-checked.
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
    /// `<`, `<=`, `>` or `>=` on a field whose type is not `number`, `date` or `datetime`.
    OrderingNotAllowed { field: String, kind: String },
    CannotCoerce {
        field: String,
        value: String,
        kind: String,
    },
    /// A value that is not one of the field's `enum` values.
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

/// Parses one `--where`/`--if` expression. An empty value is always an error here: `--set`, where
/// `k=` removes a field, is read by its own parser.
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

/// One field name on its own, read as a condition's field is: the field part of `list --sort`.
pub fn parse_field(name: &str) -> Result<FieldRef, QueryError> {
    let (parsed, rest) = take_field_name(name)?;
    if !rest.is_empty() {
        return Err(syntax(format!(
            "`{name}` is not a field name: a field name is [A-Za-z_][A-Za-z0-9_-]*"
        )));
    }
    Ok(to_field_ref(parsed))
}

/// `expr = ref-expr | plain`: `ref-expr` is told apart from `plain` by the field name alone —
/// `ref` or `refby` immediately followed by `.` — since both are otherwise ordinary field names
/// a plain condition can use (`ref=x` is a plain condition on a field literally named `ref`).
fn parse_strict(expr: &str) -> Result<Condition, QueryError> {
    let (name, rest) = take_field_name(expr)?;
    if let Some(after_dot) = rest.strip_prefix('.') {
        let dir = match name {
            "ref" => Some(Dir::Ref),
            "refby" => Some(Dir::RefBy),
            _ => None,
        };
        if let Some(dir) = dir {
            return parse_ref_expr(dir, name, after_dot).map(Condition::Ref);
        }
    }
    parse_plain_from(name, rest).map(Condition::Plain)
}

/// The condition after `ref.*(f).`. Not `parse_strict`: the grammar allows only `plain` there, so
/// a nested `ref.`/`refby.` must fail as any other field followed by `.` does.
fn parse_plain(expr: &str) -> Result<PlainCondition, QueryError> {
    let (name, rest) = take_field_name(expr)?;
    parse_plain_from(name, rest)
}

fn parse_plain_from(name: &str, rest: &str) -> Result<PlainCondition, QueryError> {
    let (op, value) = take_operator(rest)?;
    let items = parse_value(value, op)?;
    Ok(PlainCondition {
        field: to_field_ref(name),
        op,
        items,
    })
}

fn parse_ref_expr(dir: Dir, dir_name: &str, rest: &str) -> Result<RefCondition, QueryError> {
    let (quant_word, rest) = take_word(rest);
    let quant = match quant_word {
        "all" => Quant::All,
        "any" => Quant::Any,
        "none" => Quant::None,
        other => {
            return Err(syntax(format!(
                "expected `all`, `any` or `none` after `{dir_name}.`, found `{other}`"
            )));
        }
    };
    let Some(rest) = rest.strip_prefix('(') else {
        return Err(syntax(format!(
            "expected `(` after `{dir_name}.{quant_word}`, found `{rest}`"
        )));
    };
    let (field, rest) = take_ref_field(rest)?;
    let Some(rest) = rest.strip_prefix(')') else {
        return Err(syntax(format!(
            "expected `)` to close `{dir_name}.{quant_word}(...)`, found `{rest}`"
        )));
    };
    if rest.is_empty() {
        if quant == Quant::All {
            return Err(QueryError::Syntax {
                message: format!(
                    "`{dir_name}.all(f)` needs a `.EXPR`: every arrow trivially passes a condition that is not there"
                ),
                hint: Some(format!("use {dir_name}.any(f) or {dir_name}.none(f)")),
            });
        }
        return Ok(RefCondition {
            dir,
            quant,
            field,
            inner: None,
        });
    }
    let Some(after_close) = rest.strip_prefix('.') else {
        return Err(syntax(format!(
            "expected `.EXPR` after `)`, found `{rest}`"
        )));
    };
    let inner = parse_plain(after_close)?;
    Ok(RefCondition {
        dir,
        quant,
        field,
        inner: Some(inner),
    })
}

fn take_ref_field(rest: &str) -> Result<(RefField, &str), QueryError> {
    if let Some(after) = rest.strip_prefix("$body") {
        return Ok((RefField::Body, after));
    }
    let (name, after) = take_field_name(rest)?;
    Ok((RefField::Named(name.to_owned()), after))
}

/// The quantifier word, read without a field name's leading-letter rule, so a bad quantifier is
/// reported with what was found.
fn take_word(s: &str) -> (&str, &str) {
    let bytes = s.as_bytes();
    let mut end = 0;
    while end < bytes.len() {
        let b = bytes[end];
        if b.is_ascii_alphanumeric() || b == b'_' || b == b'-' {
            end += 1;
        } else {
            break;
        }
    }
    (&s[..end], &s[end..])
}

/// Field names are ASCII, so scanning bytes always stops on a character boundary.
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

/// Two-character operators are tried first, so the longest match wins.
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
                    #[expect(
                        clippy::expect_used,
                        reason = "`segments` starts as `vec![String::new()]` and the loop changes it only \
                  with `push` and by `mem::replace` with another `vec![String::new()]`, \
                  so it holds at least one element"
                    )]
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
            #[expect(
                clippy::expect_used,
                reason = "`segments` starts as `vec![String::new()]` and the loop changes it only \
                  with `push` and by `mem::replace` with another `vec![String::new()]`, \
                  so it holds at least one element"
            )]
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
        #[expect(
            clippy::expect_used,
            reason = "the `if` above runs this only when `segments.len()` is 1, so \
                      `segments.into_iter().next()` is `Some`"
        )]
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

/// Shared with `list --sort`, so a sort key reads a field as a condition does.
pub(crate) fn field_value(field: &FieldRef, doc: &Document) -> Option<Value> {
    match field {
        FieldRef::Path => Some(Value::Text(doc.path.clone())),
        FieldRef::Key => doc.key.clone().map(Value::Text),
        FieldRef::Code => doc.code.clone().map(Value::Text),
        FieldRef::Collection => Some(Value::Text(doc.collection.clone())),
        FieldRef::Schema => Some(Value::Text(doc.schema.clone())),
        FieldRef::Namespace => doc.namespace.clone().map(Value::Text),
        FieldRef::Named(name) => doc
            .fields
            .iter()
            .find(|(found, _)| found == name)
            .map(|(_, value)| value.clone()),
    }
}

/// Checks one plain condition against `schema`, then evaluates it against `doc`. A caller
/// whose scope spans several collections resolves the field before calling this. For a
/// `ref.*`/`refby.*` condition, `Project` calls this for the inner condition once per document
/// reached, with that document's own schema.
pub fn evaluate(
    condition: &PlainCondition,
    schema: &Resolved,
    doc: &Document,
) -> Result<bool, QueryError> {
    let name = field_display_name(&condition.field);
    let plan = resolve_field(&condition.field, schema)
        .ok_or_else(|| QueryError::UnknownField(name.clone()))?;
    let value = field_value(&condition.field, doc);

    if condition.op.is_ordering() {
        #[expect(
            clippy::expect_used,
            reason = "in this crate `PlainCondition` is built only in `parse_plain_from`, from the list \
                      `parse_value` returns, and `parse_value` ends with an unconditional \
                      `items.push(finish_item(segments)?)` before it returns `Ok`, so the list \
                      holds at least one item"
        )]
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
        #[expect(
            clippy::unreachable,
            reason = "`Op` has `Eq`, `Ne`, `Lt`, `Le`, `Gt` and `Ge`, and `Op::is_ordering` is true \
                      for the last four; the `if` above returns for those, so only `Eq` and `Ne` \
                      reach this `match`"
        )]
        _ => unreachable!("ordering operators are handled above"),
    })
}

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
        // A glob or a bare `*` is never coerced, whatever the field's type.
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
        // A field written with no value holds no text, here as everywhere else (SPC-4).
        Value::Empty => true,
        Value::List(items) => items.is_empty(),
        Value::Number(_) | Value::Bool(_) | Value::Date(_) | Value::Datetime(_) => false,
    }
}

/// A glob matches a `Number` against its converted value, which may differ from what was written
/// (`1e3`) and may be shared by two numbers too large for a primitive.
fn value_as_text(value: &Value) -> Option<String> {
    match value {
        Value::Text(text) | Value::Date(text) | Value::Datetime(text) => Some(text.clone()),
        Value::Empty => Some(String::new()),
        Value::Number(n) => Some(n.converted()),
        Value::Bool(b) => Some(b.to_string()),
        Value::List(_) => None,
    }
}

/// A document without the field fails, and so does a stored value that does not fit its type. A
/// date compares against a `datetime` field's date part; a datetime against a `date` field fails
/// to coerce.
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
            #[expect(
                clippy::unreachable,
                reason = "`coerce` with `FieldType::Number` and a `Value::Text` takes its \
                          `(FieldType::Number, Value::Text(text))` arm, which returns \
                          `Number::read(text).map(Value::Number)`, so the `Some` it gave is a \
                          `Value::Number`"
            )]
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
            #[expect(
                clippy::expect_used,
                reason = "`have` is the text of a `Value::Date`, which no code in this crate builds but `coerce`, after \
                          `coerce::date` accepted that text; `parse_date` runs `coerce` with \
                          `FieldType::Date` on the same text and then the same \
                          `NaiveDate::parse_from_str` that `coerce::date` ran"
            )]
            let have = parse_date(have).expect("a stored date already fits its type");
            have.partial_cmp(&want)
        }
        FieldType::Datetime => {
            let Value::Datetime(have) = value else {
                return Ok(false);
            };
            if let Some(want) = parse_datetime(text) {
                #[expect(
                    clippy::expect_used,
                    reason = "`have` is the text of a `Value::Datetime`, which no code in this crate builds but `coerce`, \
                              after `coerce::datetime` accepted that text; `parse_datetime` runs \
                              `coerce` with `FieldType::Datetime` on the same text and then the same \
                              `DateTime::parse_from_rfc3339` that `coerce::datetime` ran"
                )]
                let have = parse_datetime(have).expect("a stored datetime already fits its type");
                have.partial_cmp(&want)
            } else if let Some(want) = parse_date(text) {
                #[expect(
                    clippy::expect_used,
                    reason = "`have` is the text of a `Value::Datetime`, which no code in this crate builds but `coerce`, \
                              after `coerce::datetime` checked that its byte 10 is `T` and that \
                              `date(&text[..10])` holds; `parse_date(&have[..10])` runs `coerce` on \
                              those same ten bytes and then the same `NaiveDate::parse_from_str`"
                )]
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
        #[expect(
            clippy::unreachable,
            reason = "the `matches!` at the top of `ordering_matches` returns `OrderingNotAllowed` \
                      for every `kind` except `Number`, `Date` and `Datetime`, the three arms above"
        )]
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
        #[expect(
            clippy::unreachable,
            reason = "`compare` is called only at the end of `ordering_matches`, which `evaluate` \
                      calls only when `condition.op.is_ordering()`, and that is false for `Eq` and \
                      `Ne`"
        )]
        Op::Eq | Op::Ne => unreachable!("equality does not reach ordering_matches"),
    }
}

/// Read as a `date` field's value is; shared with `list --sort`.
pub(crate) fn parse_date(text: &str) -> Option<NaiveDate> {
    match coerce::coerce(&FieldType::Date, &Value::Text(text.to_owned())) {
        Some(Value::Date(text)) => NaiveDate::parse_from_str(&text, "%Y-%m-%d").ok(),
        _ => None,
    }
}

/// Read as a `datetime` field's value is; shared with `list --sort`.
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
