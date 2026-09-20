use std::fmt;

use serde::Deserialize;
use serde::de::{self, DeserializeSeed, Deserializer, IgnoredAny, MapAccess, SeqAccess, Visitor};

use crate::coerce::coerce;
use crate::document::Value;
use crate::schema::Resolved;

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

/// The fields of a block, in the order written, each read by the type `schema` gives it.
///
/// A value is first read as a scalar, a list of scalars or something else, and then as the text
/// written in the file, so that no reader decides what `1e3`, `no` or `2026-09-19` is. A field
/// the schema does not name, and a value that does not fit its field's type, keep the text as it
/// is written. A mapping, or a list that holds anything but scalars, has no text form and is
/// refused.
pub fn fields(block: &str, schema: &Resolved) -> Result<Vec<(String, Value)>, String> {
    if block.trim().is_empty() {
        return Ok(Vec::new());
    }
    let shapes: Option<Shapes> = yaml_serde::from_str(block).map_err(|e| e.to_string())?;
    let Some(Shapes(shapes)) = shapes else {
        return Ok(Vec::new());
    };
    if let Some((name, what)) = shapes.iter().find_map(|(name, shape)| match shape {
        Shape::Unreadable(what) => Some((name, what)),
        _ => None,
    }) {
        return Err(format!("field `{name}` is {what}, which has no text form"));
    }
    let shapes: Vec<Shape> = shapes.into_iter().map(|(_, shape)| shape).collect();
    let written = WrittenFields(&shapes)
        .deserialize(yaml_serde::Deserializer::from_str(block))
        .map_err(|e| e.to_string())?;
    Ok(written
        .into_iter()
        .map(|(name, text)| {
            let typed = schema
                .field(&name)
                .and_then(|field| coerce(&field.kind, &text));
            (name, typed.unwrap_or(text))
        })
        .collect())
}

/// What a value is, before its text is read.
enum Shape {
    Scalar,
    ScalarList,
    Unreadable(&'static str),
}

/// Each field's name and shape, in the order written.
struct Shapes(Vec<(String, Shape)>);

impl<'de> Deserialize<'de> for Shapes {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ShapesVisitor;
        impl<'de> Visitor<'de> for ShapesVisitor {
            type Value = Shapes;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a mapping of field names to values")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Shapes, A::Error> {
                let mut shapes: Vec<(String, Shape)> = Vec::new();
                while let Some(name) = map.next_key::<String>()? {
                    if shapes.iter().any(|(seen, _)| *seen == name) {
                        return Err(de::Error::custom(format!(
                            "the field `{name}` is written twice"
                        )));
                    }
                    let shape = map
                        .next_value::<Shape>()
                        .map_err(|e| de::Error::custom(format!("field `{name}`: {e}")))?;
                    shapes.push((name, shape));
                }
                Ok(Shapes(shapes))
            }
        }
        deserializer.deserialize_map(ShapesVisitor)
    }
}

impl<'de> Deserialize<'de> for Shape {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ShapeVisitor;
        impl<'de> Visitor<'de> for ShapeVisitor {
            type Value = Shape;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a scalar, a list or a mapping")
            }

            fn visit_bool<E: de::Error>(self, _: bool) -> Result<Shape, E> {
                Ok(Shape::Scalar)
            }

            fn visit_i64<E: de::Error>(self, _: i64) -> Result<Shape, E> {
                Ok(Shape::Scalar)
            }

            fn visit_u64<E: de::Error>(self, _: u64) -> Result<Shape, E> {
                Ok(Shape::Scalar)
            }

            fn visit_f64<E: de::Error>(self, _: f64) -> Result<Shape, E> {
                Ok(Shape::Scalar)
            }

            fn visit_str<E: de::Error>(self, _: &str) -> Result<Shape, E> {
                Ok(Shape::Scalar)
            }

            fn visit_unit<E: de::Error>(self) -> Result<Shape, E> {
                Ok(Shape::Scalar)
            }

            fn visit_none<E: de::Error>(self) -> Result<Shape, E> {
                Ok(Shape::Scalar)
            }

            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Shape, A::Error> {
                let mut scalars = true;
                while let Some(item) = seq.next_element::<Shape>()? {
                    scalars &= matches!(item, Shape::Scalar);
                }
                Ok(if scalars {
                    Shape::ScalarList
                } else {
                    Shape::Unreadable("a list that holds something other than text")
                })
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Shape, A::Error> {
                while map.next_entry::<IgnoredAny, IgnoredAny>()?.is_some() {}
                Ok(Shape::Unreadable("a mapping"))
            }
        }
        deserializer.deserialize_any(ShapeVisitor)
    }
}

/// The text of every field as it is written, read by the shapes found before.
struct WrittenFields<'a>(&'a [Shape]);

impl<'de> DeserializeSeed<'de> for WrittenFields<'_> {
    type Value = Vec<(String, Value)>;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        struct FieldsVisitor<'a>(&'a [Shape]);
        impl<'de> Visitor<'de> for FieldsVisitor<'_> {
            type Value = Vec<(String, Value)>;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a mapping of field names to values")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut fields = Vec::new();
                for shape in self.0 {
                    let name = map
                        .next_key::<String>()?
                        .ok_or_else(|| de::Error::custom("the block changed while it was read"))?;
                    let value = match shape {
                        Shape::ScalarList => Value::List(map.next_value::<Texts>()?.0),
                        _ => Value::Text(map.next_value::<Text>()?.0),
                    };
                    fields.push((name, value));
                }
                Ok(fields)
            }
        }
        deserializer.deserialize_map(FieldsVisitor(self.0))
    }
}

/// A scalar as the text written, whatever it would be read as.
struct Text(String);

impl<'de> Deserialize<'de> for Text {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct TextVisitor;
        impl Visitor<'_> for TextVisitor {
            type Value = Text;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a scalar")
            }

            fn visit_str<E: de::Error>(self, text: &str) -> Result<Text, E> {
                Ok(Text(text.to_owned()))
            }
        }
        deserializer.deserialize_str(TextVisitor)
    }
}

struct Texts(Vec<String>);

impl<'de> Deserialize<'de> for Texts {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct TextsVisitor;
        impl<'de> Visitor<'de> for TextsVisitor {
            type Value = Texts;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a list of scalars")
            }

            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Texts, A::Error> {
                let mut items = Vec::new();
                while let Some(Text(item)) = seq.next_element::<Text>()? {
                    items.push(item);
                }
                Ok(Texts(items))
            }
        }
        deserializer.deserialize_seq(TextsVisitor)
    }
}
