use std::fmt;

use serde::Deserialize;
use serde::de::{self, Deserializer, MapAccess, SeqAccess, Visitor};

use crate::document::Value;

const FENCE: &str = "---";

/// The lines between the opening and the closing `---`, or `None` when the file has no block.
pub fn block(text: &str) -> Result<Option<&str>, String> {
    let mut lines = text.split_inclusive('\n');
    let opening = lines.next().unwrap_or("");
    if opening.trim_end_matches(['\r', '\n']) != FENCE {
        return Ok(None);
    }
    let start = opening.len();
    let mut end = start;
    for line in lines {
        if line.trim_end_matches(['\r', '\n']) == FENCE {
            return Ok(Some(&text[start..end]));
        }
        end += line.len();
    }
    Err("the frontmatter block is never closed".to_owned())
}

/// The fields of a block, in the order written.
pub fn fields(block: &str) -> Result<Vec<(String, Value)>, String> {
    if block.trim().is_empty() {
        return Ok(Vec::new());
    }
    let read: Option<Fields> = yaml_serde::from_str(block).map_err(|e| e.to_string())?;
    Ok(read.map_or_else(Vec::new, |f| f.0))
}

struct Fields(Vec<(String, Value)>);

impl<'de> Deserialize<'de> for Fields {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct FieldsVisitor;
        impl<'de> Visitor<'de> for FieldsVisitor {
            type Value = Fields;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a mapping of field names to values")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Fields, A::Error> {
                let mut fields: Vec<(String, Value)> = Vec::new();
                while let Some(name) = map.next_key::<String>()? {
                    if fields.iter().any(|(seen, _)| *seen == name) {
                        return Err(de::Error::custom(format!(
                            "the field `{name}` is written twice"
                        )));
                    }
                    let value = map
                        .next_value::<Value>()
                        .map_err(|e| de::Error::custom(format!("field `{name}`: {e}")))?;
                    fields.push((name, value));
                }
                Ok(Fields(fields))
            }
        }
        deserializer.deserialize_map(FieldsVisitor)
    }
}

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ValueVisitor;
        impl<'de> Visitor<'de> for ValueVisitor {
            type Value = Value;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("text or a list of text")
            }

            fn visit_str<E: de::Error>(self, text: &str) -> Result<Value, E> {
                Ok(Value::Text(text.to_owned()))
            }

            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Value, A::Error> {
                let mut items = Vec::new();
                while let Some(item) = seq.next_element::<String>()? {
                    items.push(item);
                }
                Ok(Value::List(items))
            }
        }
        deserializer.deserialize_any(ValueVisitor)
    }
}
