//! A value read by the type its schema gives it. Nothing here looks at how a value is spelled
//! to decide what it is: the schema says, and a value that does not fit is not changed.

use chrono::{DateTime, NaiveDate};

use crate::document::Value;
use crate::schema::FieldType;

/// `written`, a `Text` or a `List`, as a value of `kind`, or `None` when it does not fit. A
/// value that is already typed fits nothing.
pub fn coerce(kind: &FieldType, written: &Value) -> Option<Value> {
    match (kind, written) {
        (FieldType::Other(_), Value::Text(_) | Value::List(_))
        | (FieldType::String | FieldType::Enum | FieldType::Ref, Value::Text(_))
        | (FieldType::List | FieldType::RefList, Value::List(_)) => Some(written.clone()),
        (FieldType::Number, Value::Text(text)) => text.parse().ok().map(Value::Number),
        (FieldType::Bool, Value::Text(text)) => match text.as_str() {
            "true" => Some(Value::Bool(true)),
            "false" => Some(Value::Bool(false)),
            _ => None,
        },
        (FieldType::Date, Value::Text(text)) => date(text).then(|| Value::Date(text.clone())),
        (FieldType::Datetime, Value::Text(text)) => {
            datetime(text).then(|| Value::Datetime(text.clone()))
        }
        _ => None,
    }
}

/// Whether `value`, already read by `coerce` or kept as written, is what `kind` asks for.
/// `coerce` returns the written value unchanged when nothing fits, so a field that is not one
/// of the typed variants below is a value that did not coerce: `fits` and `coerce` agree by
/// construction, without redoing the parse. `Other`, a type name the format does not have, is
/// read tolerantly and always fits.
pub fn fits(kind: &FieldType, value: &Value) -> bool {
    matches!(
        (kind, value),
        (FieldType::Other(_), Value::Text(_) | Value::List(_))
            | (
                FieldType::String | FieldType::Enum | FieldType::Ref,
                Value::Text(_)
            )
            | (FieldType::List | FieldType::RefList, Value::List(_))
            | (FieldType::Number, Value::Number(_))
            | (FieldType::Bool, Value::Bool(_))
            | (FieldType::Date, Value::Date(_))
            | (FieldType::Datetime, Value::Datetime(_))
    )
}

fn date(text: &str) -> bool {
    let bytes = text.as_bytes();
    let shaped = bytes.len() == 10
        && bytes.iter().enumerate().all(|(at, b)| {
            if at == 4 || at == 7 {
                *b == b'-'
            } else {
                b.is_ascii_digit()
            }
        });
    shaped && NaiveDate::parse_from_str(text, "%Y-%m-%d").is_ok()
}

fn datetime(text: &str) -> bool {
    text.as_bytes().get(10) == Some(&b'T')
        && date(&text[..10])
        && DateTime::parse_from_rfc3339(text).is_ok()
}
