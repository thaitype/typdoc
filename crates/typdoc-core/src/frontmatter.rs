use std::fmt;

use serde::de::{
    self, DeserializeSeed, Deserializer, EnumAccess, IgnoredAny, MapAccess, SeqAccess,
    VariantAccess, Visitor,
};
use serde::ser::SerializeMap;
use serde::{Deserialize, Serialize, Serializer};

use crate::coerce::coerce;
use crate::document::Value;
use crate::lines::next_line;
use crate::schema::Resolved;

const FENCE: &str = "---";

/// A file cut into its frontmatter block and the body after it.
pub struct Split<'a> {
    /// The lines between the opening and the closing `---`, or `None` when the file has no block.
    pub block: Option<&'a str>,
    /// Where the body begins in the file: after the closing `---` line, or 0 with no block.
    pub body: usize,
}

/// Cuts a file where its block ends. A line ends as the design says, so a file with `\r\n` or
/// lone `\r` endings has its block found too.
pub fn split(text: &str) -> Result<Split<'_>, String> {
    let line = |start: usize| {
        let end = next_line(text, start);
        (text[start..end].trim_end_matches(['\r', '\n']), end)
    };
    let (opening, start) = line(0);
    if opening != FENCE {
        return Ok(Split {
            block: None,
            body: 0,
        });
    }
    let mut at = start;
    while at < text.len() {
        let (content, next) = line(at);
        if content == FENCE {
            return Ok(Split {
                block: Some(&text[start..at]),
                body: next,
            });
        }
        at = next;
    }
    Err("the frontmatter block is never closed".to_owned())
}

/// The lines between the opening and the closing `---`, or `None` when the file has no block.
pub fn block(text: &str) -> Result<Option<&str>, String> {
    split(text).map(|split| split.block)
}

/// The fields of a block, in the order written, each read by the type `schema` gives it.
///
/// A value is first read as a scalar, a list of scalars or something else, and then as the text
/// written in the file, so that no reader decides what `1e3`, `no` or `2026-09-19` is. A field
/// the schema does not name, and a value that does not fit its field's type, keep the text as it
/// is written. An integer of any length is a scalar and so is a value with a tag (`!Ref x`): the
/// text is the digits, or the value without its tag. A mapping, or a list that holds anything
/// but scalars, has no text form and is refused, tagged or not.
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

/// Builds the frontmatter block a write produces, through the three operations phase 1's
/// ticket 1 put in front of the writer and decision 20 keeps: set a scalar, append or remove a
/// list item, and add a key. What decision 20 changed is what stands behind them: there is no
/// document being edited in place any more, only this block, built fresh from the fields a read
/// already found and finished into text in one call, so "a write rewrites the whole frontmatter
/// block, not the line it changed" holds however a caller reaches this trait — the three
/// operations change what the block holds, [`FrontmatterWriter::finish`] is the one place that
/// turns what it holds into text, and an implementation is free to change there without a
/// caller changing, which is the property the trait exists to keep.
///
/// A caller starts from the fields a read already found, in the order they were read, so that a
/// key already present never moves; [`YamlSerdeWriter::new`] is that starting point for the one
/// implementation this ticket builds.
pub trait FrontmatterWriter {
    /// Sets `name` to hold `text`, replacing whatever it held before if the field exists, or
    /// adding it at the end if it does not ("a key that is added goes at the end of the
    /// frontmatter block").
    fn set_scalar(&mut self, name: &str, text: String);

    /// Appends `item` to the list at `name`. A field that does not exist yet is added, at the
    /// end, as a list of the one item; what happens to a field that exists but is not a list is
    /// not settled by anything upstream of this ticket, and the choice made here is the one
    /// that cannot corrupt the block: the field becomes a new list holding just `item`, in the
    /// position it already had, rather than the append being refused with no seam here to
    /// report that refusal through.
    fn append_item(&mut self, name: &str, item: String);

    /// Removes the first item at `name` equal to `item`. A field that does not exist, or does
    /// not hold `item`, is left exactly as it was: removing something that was never there is
    /// not a write. A field with every item removed stays a field holding an empty list, since
    /// nothing here decides that removing the last item should remove the field.
    fn remove_item(&mut self, name: &str, item: &str);

    /// Adds `name` with no value, at the end, unless it already exists, in which case it is
    /// left untouched. "No value" is [`Value::Empty`], a field with a name and nothing after it
    /// (`reviewer:`), kept apart from a field written as the empty string (`docs/design.md`, "A
    /// field written with no value at all is not the same as one written as an empty string").
    fn add_key(&mut self, name: &str);

    /// Sets `name` to hold exactly `items`, replacing whatever it held before if the field
    /// exists (a scalar included), or adding it at the end if it does not.
    ///
    /// None of the three operations above expresses this: [`FrontmatterWriter::append_item`]
    /// only ever adds one item to what is already there, and nothing removes every item while
    /// keeping the field. `typdoc set`'s own `k=v1,v2` is a comma-separated array value (design,
    /// `typdoc new`: "Array values are comma-separated"), and it replaces the field's value, the
    /// same as [`FrontmatterWriter::set_scalar`] does for a scalar one — so a list field needs
    /// the same replacing operation a scalar field already has.
    fn set_list(&mut self, name: &str, items: Vec<String>);

    /// Removes `name` entirely, whatever it holds — a scalar, a list or [`Value::Empty`] alike.
    /// A field that does not exist is left exactly as it was: removing something that was never
    /// there is not a write, the same principle [`FrontmatterWriter::remove_item`] already
    /// documents for one item of a list. This is `typdoc set`'s own `k=` (design, `typdoc set`:
    /// "`k=` removes a field"), which [`FrontmatterWriter::remove_item`] cannot express, since
    /// that method removes one item equal to a given value, never a whole field.
    fn remove_field(&mut self, name: &str);

    /// The block's text, between the fences, assembled from every field this holds at the point
    /// it is called: no value is reinterpreted on the way, so a [`Value::Text`], a
    /// [`Value::Number`]'s written digits, a [`Value::Date`] or [`Value::Datetime`]'s written
    /// form and a [`Value::Bool`]'s `true`/`false` are all written out exactly, quoted only
    /// where YAML would otherwise read them as something else. A [`Value::List`] becomes a
    /// block list, one item per line, or `[]` when it holds none. A [`Value::Empty`] is written
    /// with nothing after its name, the form it was read in.
    fn finish(&self) -> Result<String, String>;
}

/// Writes with `yaml_serde`, the crate the reader already trusts, so the writer and the reader
/// cannot disagree about what a document says (decision 20).
pub struct YamlSerdeWriter {
    fields: Vec<(String, Value)>,
}

impl YamlSerdeWriter {
    /// Starts from `fields`, in the order given — ordinarily the fields a read already found,
    /// so that a key already present never moves and a write over an empty document starts
    /// from nothing.
    pub fn new(fields: Vec<(String, Value)>) -> Self {
        YamlSerdeWriter { fields }
    }

    fn position(&self, name: &str) -> Option<usize> {
        self.fields.iter().position(|(field, _)| field == name)
    }
}

impl FrontmatterWriter for YamlSerdeWriter {
    fn set_scalar(&mut self, name: &str, text: String) {
        match self.position(name) {
            Some(at) => self.fields[at].1 = Value::Text(text),
            None => self.fields.push((name.to_owned(), Value::Text(text))),
        }
    }

    fn append_item(&mut self, name: &str, item: String) {
        match self.position(name) {
            Some(at) => match &mut self.fields[at].1 {
                Value::List(items) => items.push(item),
                other => *other = Value::List(vec![item]),
            },
            None => self.fields.push((name.to_owned(), Value::List(vec![item]))),
        }
    }

    fn remove_item(&mut self, name: &str, item: &str) {
        if let Some(at) = self.position(name)
            && let Value::List(items) = &mut self.fields[at].1
            && let Some(found) = items.iter().position(|held| held == item)
        {
            items.remove(found);
        }
    }

    fn add_key(&mut self, name: &str) {
        if self.position(name).is_none() {
            self.fields.push((name.to_owned(), Value::Empty));
        }
    }

    fn set_list(&mut self, name: &str, items: Vec<String>) {
        match self.position(name) {
            Some(at) => self.fields[at].1 = Value::List(items),
            None => self.fields.push((name.to_owned(), Value::List(items))),
        }
    }

    fn remove_field(&mut self, name: &str) {
        if let Some(at) = self.position(name) {
            self.fields.remove(at);
        }
    }

    fn finish(&self) -> Result<String, String> {
        // An empty block is nothing between the fences; `yaml_serde` has no call that means
        // "an empty mapping written as nothing", so this case is handled before reaching it.
        if self.fields.is_empty() {
            return Ok(String::new());
        }
        let written = yaml_serde::to_string(&Block(&self.fields)).map_err(|e| e.to_string())?;
        Ok(bare_the_empty_fields(written, &self.fields))
    }
}

/// `yaml_serde` has no call that writes a scalar with nothing after it: unit and `None` both
/// spell YAML's null as the word `null`, and an empty string is quoted to keep it from reading
/// as that same null on the way back in. So [`Block::serialize`] writes a [`Value::Empty`] field
/// as `name: null`, the one place in a block this writer produces that exact line unquoted — a
/// [`Value::Text`] holding the literal text `"null"` is quoted (`'null'`), because that text
/// would otherwise read back as this word does — and this pass turns each such line into `name:`
/// with nothing after it, matching every field this run's `fields` marks [`Value::Empty`] by
/// name against a whole line of the text `yaml_serde` produced, never a substring, so a value
/// that happens to contain the same text elsewhere is left alone.
fn bare_the_empty_fields(written: String, fields: &[(String, Value)]) -> String {
    let empty: Vec<&str> = fields
        .iter()
        .filter(|(_, value)| matches!(value, Value::Empty))
        .map(|(name, _)| name.as_str())
        .collect();
    if empty.is_empty() {
        return written;
    }
    let mut out = String::with_capacity(written.len());
    for line in written.lines() {
        match line.strip_suffix(": null") {
            Some(name) if empty.contains(&name) => {
                out.push_str(name);
                out.push(':');
            }
            _ => out.push_str(line),
        }
        out.push('\n');
    }
    out
}

/// A `Serialize` wrapper over the fields of a block, in the order given: a `BTreeMap` would
/// sort them, and a key never moves on a write.
struct Block<'a>(&'a [(String, Value)]);

impl Serialize for Block<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for (name, value) in self.0 {
            match value {
                Value::List(items) => map.serialize_entry(name, items)?,
                // Written as the word `null` here and turned bare afterward
                // (`bare_the_empty_fields`): nothing serde's data model offers writes as a
                // scalar with nothing after it.
                Value::Empty => map.serialize_entry(name, &())?,
                Value::Text(text) | Value::Date(text) | Value::Datetime(text) => {
                    map.serialize_entry(name, text)?
                }
                Value::Number(number) => map.serialize_entry(name, number.written())?,
                Value::Bool(value) => {
                    map.serialize_entry(name, if *value { "true" } else { "false" })?
                }
            }
        }
        map.end()
    }
}

/// What a value is, before its text is read.
enum Shape {
    Scalar,
    /// The reader resolved this scalar as YAML's null: a field written with a name and nothing
    /// after it, or a plain `~`, `null`, `Null` or `NULL`. The two are told apart once the text
    /// is read: empty text is the first (design, "Document files"), and any other text is a
    /// value that happens to spell null and is kept as written, the same as every other scalar.
    Null,
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

            fn visit_i128<E: de::Error>(self, _: i128) -> Result<Shape, E> {
                Ok(Shape::Scalar)
            }

            fn visit_u128<E: de::Error>(self, _: u128) -> Result<Shape, E> {
                Ok(Shape::Scalar)
            }

            fn visit_f64<E: de::Error>(self, _: f64) -> Result<Shape, E> {
                Ok(Shape::Scalar)
            }

            fn visit_str<E: de::Error>(self, _: &str) -> Result<Shape, E> {
                Ok(Shape::Scalar)
            }

            fn visit_unit<E: de::Error>(self) -> Result<Shape, E> {
                Ok(Shape::Null)
            }

            fn visit_none<E: de::Error>(self) -> Result<Shape, E> {
                Ok(Shape::Null)
            }

            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Shape, A::Error> {
                let mut scalars = true;
                while let Some(item) = seq.next_element::<Shape>()? {
                    scalars &= matches!(item, Shape::Scalar | Shape::Null);
                }
                Ok(if scalars {
                    Shape::ScalarList
                } else {
                    Shape::Unreadable("a list that holds something other than text")
                })
            }

            /// The reader gives a value with a tag of its own (`!Ref x`, `!custom [a]`) as an
            /// enum: the tag is not part of the text, so the shape is the value's.
            fn visit_enum<A: EnumAccess<'de>>(self, data: A) -> Result<Shape, A::Error> {
                let (IgnoredAny, variant) = data.variant()?;
                variant.newtype_variant()
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
                        Shape::Null => match map.next_value::<Text>()?.0 {
                            text if text.is_empty() => Value::Empty,
                            text => Value::Text(text),
                        },
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

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use serde_json::json;

    use super::*;
    use crate::schema::Field;

    fn schema(fields: &[(&str, &str)]) -> Resolved {
        let map: BTreeMap<String, Field> = fields
            .iter()
            .map(|(name, kind)| {
                let field = serde_json::from_value(json!({ "type": kind })).expect("a field");
                ((*name).to_owned(), field)
            })
            .collect();
        Resolved::new("test".to_owned(), None, map)
    }

    fn text(written: &str) -> Value {
        Value::Text(written.to_owned())
    }

    fn list(items: &[&str]) -> Value {
        Value::List(items.iter().map(|item| (*item).to_owned()).collect())
    }

    fn read(block: &str) -> Vec<(String, Value)> {
        let schema = schema(&[("s", "string"), ("n", "number"), ("l", "list")]);
        fields(block, &schema).unwrap_or_else(|e| panic!("{block:?}: {e}"))
    }

    #[test]
    fn an_integer_past_64_bits_is_a_scalar_that_keeps_its_digits() {
        for written in [
            "123456789012345678901",
            "-9223372036854775809",
            "18446744073709551616",
            "340282366920938463463374607431768211456",
            "-170141183460469231731687303715884105729",
        ] {
            assert_eq!(
                read(&format!("s: {written}")),
                vec![("s".to_owned(), text(written))],
                "{written}"
            );
            assert_eq!(
                read(&format!("l: [1, {written}]")),
                vec![("l".to_owned(), list(&["1", written]))],
                "{written}"
            );
        }
    }

    #[test]
    fn a_tagged_scalar_is_the_value_without_its_tag() {
        assert_eq!(
            read("s: !Ref Thing\nl: [!custom a, b]"),
            vec![
                ("s".to_owned(), text("Thing")),
                ("l".to_owned(), list(&["a", "b"]))
            ]
        );
        assert_eq!(
            read("s: !custom \"a b\""),
            vec![("s".to_owned(), text("a b"))]
        );
    }

    #[test]
    fn a_number_field_written_past_u64_keeps_its_digits_and_converts_to_a_float() {
        let read = read("n: 123456789012345678901");

        let [(name, Value::Number(number))] = read.as_slice() else {
            panic!("one number: {read:?}");
        };
        assert_eq!(name, "n");
        assert_eq!(number.written(), "123456789012345678901");
        assert!((number.as_f64().unwrap() / 1.2345678901234568e20 - 1.0).abs() < 1e-15);
    }

    #[test]
    fn a_tag_on_a_mapping_is_refused_and_a_tag_on_a_list_of_scalars_is_not() {
        let schema = schema(&[("s", "string"), ("l", "list")]);

        let refused = fields("s: !custom { a: 1 }", &schema).unwrap_err();
        assert_eq!(refused, "field `s` is a mapping, which has no text form");

        let refused = fields("l: !custom [a, { b: 1 }]", &schema).unwrap_err();
        assert_eq!(
            refused,
            "field `l` is a list that holds something other than text, which has no text form"
        );

        assert_eq!(
            fields("l: !custom [a, b]", &schema).unwrap(),
            vec![("l".to_owned(), list(&["a", "b"]))]
        );
    }

    /// The design's promise for an anchor and its alias (`&anchor` with `*alias`  "The value
    /// written out in full at every place that used it") starts here, in what the read path
    /// already gives back for each: no write-side work turns one into the other, because they
    /// are already two independent copies of the same text by the time a write sees them.
    #[test]
    fn an_anchor_and_its_alias_are_each_read_as_the_full_value() {
        let schema = schema(&[("original", "string"), ("mirrored", "string")]);
        let block = "original: &shared shared text\nmirrored: *shared\n";

        assert_eq!(
            fields(block, &schema).unwrap(),
            vec![
                ("original".to_owned(), text("shared text")),
                ("mirrored".to_owned(), text("shared text")),
            ]
        );
    }

    /// Reads `block` back with a schema that names none of its fields, so every value stays the
    /// `Text` or `List` it was written as: a test at the boundary between what a write assembled
    /// and what it says, not a test of coercion, which is the read path's own job and untouched
    /// by anything here.
    fn reread(block: &str) -> Vec<(String, Value)> {
        fields(
            block,
            &Resolved::new("test".to_owned(), None, BTreeMap::new()),
        )
        .unwrap_or_else(|e| panic!("{block:?}: {e}"))
    }

    /// Finishes a [`YamlSerdeWriter`] started from `fields`, panicking with the fields on a
    /// failure, since every case below hands it fields the read path could have produced.
    fn write(fields: &[(String, Value)]) -> String {
        YamlSerdeWriter::new(fields.to_vec())
            .finish()
            .unwrap_or_else(|e| panic!("{fields:?}: {e}"))
    }

    #[test]
    fn a_plain_value_is_written_bare_and_a_key_never_moves() {
        let written = write(&[
            ("title".to_owned(), text("Ship it")),
            ("status".to_owned(), text("open")),
        ]);

        assert_eq!(written, "title: Ship it\nstatus: open\n");
    }

    #[test]
    fn no_fields_is_an_empty_block() {
        assert_eq!(write(&[]), "");
    }

    #[test]
    fn a_flow_list_is_written_as_a_block_list_and_an_empty_list_as_brackets() {
        let written = write(&[
            ("tags".to_owned(), list(&["a", "b"])),
            ("empty".to_owned(), list(&[])),
        ]);

        assert_eq!(written, "tags:\n- a\n- b\nempty: []\n");
    }

    /// Every literal the design names as needing to survive a write exactly (`docs/design.md`,
    /// "Document files"), plus the shapes decision 20 measured: read through `write` and back
    /// through [`reread`], the text of the one field must be untouched.
    #[test]
    fn a_value_the_format_holds_exactly_is_written_back_exactly() {
        let literals = [
            "1e3",
            "1.10",
            "0755",
            "123456789012345678901234567890", // an integer longer than 64 bits
            "-9223372036854775809",
            "no",
            "yes",
            "~",
            "null",
            "true",
            "2026-09-19",
            "2026-09-19T14:30:00+07:00",
            "", // an empty string
            "leading and trailing ",
            " leading space",
            "has\na newline",
            "has\ta tab",
            "*star",
            "&amp",
            "- item",
            "? what",
            "a: b",
            "[a, b]",
            "{x: 1}",
            "already 'quoted'",
            "already \"quoted\"",
        ];

        for literal in literals {
            let written = write(&[("v".to_owned(), text(literal))]);
            let back = reread(&written);

            assert_eq!(
                back,
                vec![("v".to_owned(), text(literal))],
                "{literal:?} did not round-trip through {written:?}"
            );
        }
    }

    #[test]
    fn a_number_field_keeps_the_digits_it_was_written_with() {
        let number =
            Value::Number(crate::document::Number::read("99999999999999999999").expect("a number"));

        let written = write(&[("n".to_owned(), number)]);
        let back = reread(&written);

        assert_eq!(back, vec![("n".to_owned(), text("99999999999999999999"))]);
    }

    #[test]
    fn a_bool_field_is_written_as_the_word_it_was_read_from() {
        for (value, word) in [(true, "true"), (false, "false")] {
            let written = write(&[("b".to_owned(), Value::Bool(value))]);
            let back = reread(&written);

            assert_eq!(back, vec![("b".to_owned(), text(word))], "{word}");
        }
    }

    #[test]
    fn a_date_and_a_datetime_are_written_with_the_text_they_were_read_with() {
        for value in [
            Value::Date("2026-09-19".to_owned()),
            Value::Datetime("2026-09-19T14:30:00+07:00".to_owned()),
        ] {
            let text_form = match &value {
                Value::Date(t) | Value::Datetime(t) => t.clone(),
                _ => unreachable!(),
            };
            let written = write(&[("d".to_owned(), value)]);
            let back = reread(&written);

            assert_eq!(back, vec![("d".to_owned(), text(&text_form))]);
        }
    }

    #[test]
    fn a_value_written_with_quotes_it_did_not_need_comes_back_unquoted() {
        // The source text of a block that was never read through this writer: this is the one
        // test in the file allowed to hold literal quote marks in the input, because it is
        // pinning what a write drops, not what a value is.
        let block = "title: 'Ship it'\nflag: \"true\"\n";
        let schema = schema(&[("title", "string")]);
        let before = fields(block, &schema).unwrap();

        let written = write(&before);

        assert_eq!(
            written, "title: Ship it\nflag: 'true'\n",
            "'Ship it' needed no quotes to survive and lost them; `true` needs quotes to stay \
             text rather than become YAML's boolean, and kept them"
        );
    }

    #[test]
    fn setting_one_field_leaves_the_others_written_the_same_way() {
        let before = vec![
            ("a".to_owned(), text("1e3")),
            ("b".to_owned(), text("unchanged")),
            ("c".to_owned(), list(&["x", "y"])),
        ];

        let mut after = before.clone();
        after[1] = ("b".to_owned(), text("changed"));
        let written = write(&after);
        let reread_back = reread(&written);

        assert_eq!(
            reread_back,
            vec![
                ("a".to_owned(), text("1e3")),
                ("b".to_owned(), text("changed")),
                ("c".to_owned(), list(&["x", "y"])),
            ]
        );
    }

    #[test]
    fn append_item_adds_to_an_existing_list_or_starts_a_new_one_at_the_end() {
        let mut writer = YamlSerdeWriter::new(vec![("tags".to_owned(), list(&["a"]))]);

        writer.append_item("tags", "b".to_owned());
        writer.append_item("new_list", "only".to_owned());

        assert_eq!(
            reread(&writer.finish().unwrap()),
            vec![
                ("tags".to_owned(), list(&["a", "b"])),
                ("new_list".to_owned(), list(&["only"])),
            ]
        );
    }

    #[test]
    fn append_item_on_a_scalar_field_replaces_it_with_a_new_list_in_place() {
        let mut writer = YamlSerdeWriter::new(vec![
            ("a".to_owned(), text("scalar")),
            ("b".to_owned(), text("untouched")),
        ]);

        writer.append_item("a", "only".to_owned());

        assert_eq!(
            reread(&writer.finish().unwrap()),
            vec![
                ("a".to_owned(), list(&["only"])),
                ("b".to_owned(), text("untouched")),
            ]
        );
    }

    #[test]
    fn remove_item_takes_out_one_match_and_leaves_an_empty_list_rather_than_removing_the_field() {
        let mut writer = YamlSerdeWriter::new(vec![("tags".to_owned(), list(&["a", "b", "a"]))]);

        writer.remove_item("tags", "a");
        assert_eq!(
            reread(&writer.finish().unwrap()),
            vec![("tags".to_owned(), list(&["b", "a"]))],
            "only the first match is removed"
        );

        writer.remove_item("tags", "b");
        writer.remove_item("tags", "a");
        assert_eq!(
            reread(&writer.finish().unwrap()),
            vec![("tags".to_owned(), list(&[]))],
            "every item removed leaves an empty list, not a removed field"
        );
    }

    #[test]
    fn remove_item_on_a_field_that_does_not_hold_it_or_does_not_exist_changes_nothing() {
        let mut writer = YamlSerdeWriter::new(vec![
            ("tags".to_owned(), list(&["a"])),
            ("scalar".to_owned(), text("x")),
        ]);

        writer.remove_item("tags", "not-there");
        writer.remove_item("scalar", "x");
        writer.remove_item("absent", "x");

        assert_eq!(
            reread(&writer.finish().unwrap()),
            vec![
                ("tags".to_owned(), list(&["a"])),
                ("scalar".to_owned(), text("x")),
            ]
        );
    }

    #[test]
    fn add_key_adds_a_field_with_no_value_at_the_end_and_leaves_an_existing_one_untouched() {
        let mut writer = YamlSerdeWriter::new(vec![("title".to_owned(), text("Ship it"))]);

        writer.add_key("reviewer");
        writer.add_key("title"); // already present: untouched, and not moved

        let written = writer.finish().unwrap();
        assert_eq!(written, "title: Ship it\nreviewer:\n");
        assert_eq!(
            reread(&written),
            vec![
                ("title".to_owned(), text("Ship it")),
                ("reviewer".to_owned(), Value::Empty),
            ]
        );
    }

    #[test]
    fn set_list_replaces_a_list_or_a_scalar_field_and_starts_a_new_one_at_the_end() {
        let mut writer = YamlSerdeWriter::new(vec![
            ("tags".to_owned(), list(&["a", "b"])),
            ("scalar".to_owned(), text("x")),
            ("untouched".to_owned(), text("y")),
        ]);

        writer.set_list("tags", vec!["c".to_owned(), "d".to_owned()]);
        writer.set_list("scalar", vec!["only".to_owned()]);
        writer.set_list("new_list", vec!["first".to_owned()]);

        assert_eq!(
            reread(&writer.finish().unwrap()),
            vec![
                ("tags".to_owned(), list(&["c", "d"])),
                ("scalar".to_owned(), list(&["only"])),
                ("untouched".to_owned(), text("y")),
                ("new_list".to_owned(), list(&["first"])),
            ]
        );
    }

    #[test]
    fn remove_field_takes_out_the_whole_field_and_a_field_that_does_not_exist_changes_nothing() {
        let mut writer = YamlSerdeWriter::new(vec![
            ("tags".to_owned(), list(&["a", "b"])),
            ("scalar".to_owned(), text("x")),
            ("kept".to_owned(), text("y")),
        ]);

        writer.remove_field("tags");
        writer.remove_field("scalar");
        writer.remove_field("absent");

        assert_eq!(
            reread(&writer.finish().unwrap()),
            vec![("kept".to_owned(), text("y"))]
        );
    }

    /// Goal criterion 1, at the two forms decision 21 is about: a field read as
    /// [`Value::Empty`] (`bare:`, YAML's null) and one read as [`Value::Text`] holding nothing
    /// (`quoted: ''`) each write back in the form they were read in, byte for byte, whether or
    /// not either is the field a `set` changes (`docs/design.md`, "A field written with no
    /// value at all is not the same as one written as an empty string").
    #[test]
    fn a_bare_field_and_an_empty_string_field_are_each_written_back_in_the_form_they_held() {
        let before = "bare:\nquoted: ''\nother: kept\n";
        let read = reread(before);
        assert_eq!(
            read,
            vec![
                ("bare".to_owned(), Value::Empty),
                ("quoted".to_owned(), text("")),
                ("other".to_owned(), text("kept")),
            ],
            "the two forms are told apart on the way in"
        );

        // Neither is the field a write changes: both round-trip in the form they held.
        let mut writer = YamlSerdeWriter::new(read.clone());
        writer.set_scalar("other", "changed".to_owned());
        assert_eq!(
            writer.finish().unwrap(),
            "bare:\nquoted: ''\nother: changed\n"
        );

        // Each in turn is the field a write changes, so the other keeps the form it held.
        let mut writer = YamlSerdeWriter::new(read.clone());
        writer.set_scalar("bare", "no longer bare".to_owned());
        assert_eq!(
            writer.finish().unwrap(),
            "bare: no longer bare\nquoted: ''\nother: kept\n"
        );

        let mut writer = YamlSerdeWriter::new(read);
        writer.set_scalar("quoted", "no longer empty".to_owned());
        assert_eq!(
            writer.finish().unwrap(),
            "bare:\nquoted: no longer empty\nother: kept\n"
        );
    }
}

/// The round trip of the contract's first criterion (goal criterion 1), run over the fixtures
/// rather than over cases written by hand: `set` of one field, and every other field read back
/// exactly as it held before. Kept apart from `mod tests` because it reads files from disk,
/// which the cases above never do.
///
/// "Assembling the block it meant to" is checked here, at the boundary of typdoc's own code
/// (decision 20), rather than by a guard that ships: there is no re-read guard in the write
/// path itself, on the reasoning decision 20 gives — a mismatch here would be a defect in
/// `yaml_serde`, the crate the read path already trusts unchecked, not one typdoc's own code
/// could catch by reading its own output back.
#[cfg(test)]
mod corpus {
    use std::collections::BTreeMap;
    use std::path::{Path, PathBuf};

    use super::*;

    /// Every `.md` file under `fixtures/valid/`, found once and sorted, so the walk is
    /// deterministic and a file this test does not account for is one the count at the end
    /// reports rather than one that silently drops out.
    fn documents() -> Vec<PathBuf> {
        let root = typdoc_testkit::fixtures::path("valid");
        let mut found = Vec::new();
        walk(&root, &mut found);
        found.sort();
        found
    }

    fn walk(dir: &Path, found: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display())) {
            let entry = entry.unwrap_or_else(|e| panic!("{}: {e}", dir.display()));
            let path = entry.path();
            if path.is_dir() {
                walk(&path, found);
            } else if path.extension().is_some_and(|ext| ext == "md") {
                found.push(path);
            }
        }
    }

    /// A schema that names no field, so every value read through it stays the `Text` or `List`
    /// it was written as. This corpus test is about the writer, not about what a schema coerces
    /// a value into: coercion is a pure function of the text a write already promises not to
    /// change, so a schema-typed value round-trips exactly whenever its text does, and a real
    /// per-fixture schema would prove nothing this one does not.
    fn no_schema() -> Resolved {
        Resolved::new("test".to_owned(), None, BTreeMap::new())
    }

    /// A value distinguishably different from `value`, of the same shape as what
    /// [`mutate`] below produces, so a round trip exercises `set` changing the field it names
    /// and nothing else.
    const MARKER: &str = "frontmatter-write-corpus-marker";

    fn changed_value(value: &Value) -> Value {
        match value {
            Value::List(items) => {
                let mut items = items.clone();
                items.push(MARKER.to_owned());
                Value::List(items)
            }
            _ => Value::Text(MARKER.to_owned()),
        }
    }

    /// Changes `name` on `writer` through the trait's own operations, the way a caller reaches
    /// it, rather than by editing a `Vec` directly: [`FrontmatterWriter::append_item`] for a
    /// list, [`FrontmatterWriter::set_scalar`] for anything else, matching what
    /// [`changed_value`] expects to read back.
    fn mutate(writer: &mut YamlSerdeWriter, name: &str, current: &Value) {
        match current {
            Value::List(_) => writer.append_item(name, MARKER.to_owned()),
            _ => writer.set_scalar(name, MARKER.to_owned()),
        }
    }

    #[test]
    fn every_document_in_the_fixtures_round_trips_a_set_of_one_field() {
        let documents = documents();
        assert!(
            !documents.is_empty(),
            "no fixture document was found to check"
        );

        let (mut checked, mut no_block, mut unparseable, mut no_fields) =
            (0usize, 0usize, 0usize, 0usize);

        for path in &documents {
            let text =
                std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            let raw_block = match block(&text) {
                Ok(Some(raw_block)) => raw_block,
                Ok(None) => {
                    no_block += 1;
                    continue;
                }
                Err(_) => {
                    // `frontmatter.parse` fixtures hold a block on purpose that cannot be read
                    // at all; there is nothing for a write to round-trip.
                    unparseable += 1;
                    continue;
                }
            };
            let before = match fields(raw_block, &no_schema()) {
                Ok(before) => before,
                Err(_) => {
                    unparseable += 1;
                    continue;
                }
            };
            if before.is_empty() {
                no_fields += 1;
                continue;
            }
            checked += 1;

            for at in 0..before.len() {
                let name = before[at].0.clone();
                let expected_after = changed_value(&before[at].1);

                let mut writer = YamlSerdeWriter::new(before.clone());
                mutate(&mut writer, &name, &before[at].1);
                let written = writer.finish().unwrap_or_else(|e| {
                    panic!("{}: writing after setting {name}: {e}", path.display())
                });
                let reread = fields(&written, &no_schema()).unwrap_or_else(|e| {
                    panic!(
                        "{}: re-reading after setting {name}: {e}\n--- written ---\n{written}",
                        path.display()
                    )
                });

                assert_eq!(
                    reread.len(),
                    before.len(),
                    "{}: setting {name} changed the number of fields",
                    path.display()
                );
                for (index, (reread_name, reread_value)) in reread.iter().enumerate() {
                    let (expected_name, expected_value) = if index == at {
                        (&name, &expected_after)
                    } else {
                        (&before[index].0, &before[index].1)
                    };
                    assert_eq!(
                        reread_name,
                        expected_name,
                        "{}: field {index} moved after setting {name}",
                        path.display()
                    );
                    assert_eq!(
                        reread_value,
                        expected_value,
                        "{}: field {reread_name} changed value after setting {name}, which did \
                         not ask for it",
                        path.display()
                    );
                }
            }
        }

        assert_eq!(
            checked + no_block + unparseable + no_fields,
            documents.len(),
            "a fixture document was found and then accounted for nowhere"
        );
        assert!(checked > 0, "no fixture document had a field to round-trip");
        println!(
            "frontmatter round trip over {}: {checked} checked, {no_block} with no block, \
             {unparseable} unparseable, {no_fields} with no fields",
            documents.len()
        );
    }

    /// The raw blocks of every fixture document that has one, so a shape is looked for in the
    /// text as written, not in what `fields` made of it — a comment and a blank line have
    /// already been thrown away by the time `fields` sees anything.
    fn raw_blocks() -> Vec<String> {
        documents()
            .iter()
            .filter_map(|path| {
                let text = std::fs::read_to_string(path)
                    .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
                block(&text).ok().flatten().map(str::to_owned)
            })
            .collect()
    }

    fn has_comment(b: &str) -> bool {
        b.lines().any(|l| l.trim_start().starts_with('#'))
    }

    fn has_blank_line_between_fields(b: &str) -> bool {
        let lines: Vec<&str> = b.lines().collect();
        lines
            .windows(3)
            .any(|w| !w[0].trim().is_empty() && w[1].trim().is_empty() && !w[2].trim().is_empty())
    }

    fn has_flow_list(b: &str) -> bool {
        b.lines().any(|l| l.contains(": ["))
    }

    fn has_a_quoted_value(b: &str) -> bool {
        b.lines().any(|l| l.contains(": '") || l.contains(": \""))
    }

    fn has_extra_spacing_after_a_colon(b: &str) -> bool {
        b.lines().any(|l| match l.split_once(':') {
            Some((_, after)) => {
                let trimmed = after.trim_start();
                !trimmed.is_empty() && after.len() - trimmed.len() >= 2
            }
            None => false,
        })
    }

    fn has_an_anchor_and_an_alias(b: &str) -> bool {
        b.lines().any(|l| l.contains('&')) && b.lines().any(|l| l.contains('*'))
    }

    fn has_a_tag(b: &str) -> bool {
        b.lines().any(|l| l.contains(" !"))
    }

    /// Which detector tells whether some document holds the shape a row of the design's table
    /// of losses names, matched on a fragment of the row stable across a rewording of the
    /// table's prose. `Err` names a row nothing here recognizes, so a row the design adds is a
    /// row this test cannot silently pass on the strength of a different one.
    fn detector_for(row: &str) -> Result<fn(&str) -> bool, String> {
        if row == "Comments, anywhere in the block" {
            Ok(has_comment)
        } else if row == "Blank lines between fields" {
            Ok(has_blank_line_between_fields)
        } else if row.contains("tags: [a, b]") {
            Ok(has_flow_list)
        } else if row.contains("'Ship it'") {
            Ok(has_a_quoted_value)
        } else if row.contains("id:   WF-3") {
            Ok(has_extra_spacing_after_a_colon)
        } else if row.contains("&anchor") {
            Ok(has_an_anchor_and_an_alias)
        } else if row.contains("!Ref") {
            Ok(has_a_tag)
        } else {
            Err(format!(
                "the design's table of losses has a row this check does not recognize: \
                 {row:?}; teach this test to tell whether a fixture document holds it before \
                 trusting that one does"
            ))
        }
    }

    #[test]
    fn a_row_the_detector_does_not_recognize_is_named_rather_than_skipped() {
        let error = detector_for("a shape nobody taught this test about").unwrap_err();

        assert!(
            error.contains("a shape nobody taught this test about"),
            "{error}"
        );
    }

    #[test]
    fn each_detector_tells_the_shape_it_names_from_a_block_with_none_of_the_others() {
        // A block with none of the seven shapes: every detector below must say so, so that a
        // detector that always answers `true` is caught by this line before it is trusted to
        // report a gap in the corpus.
        let plain = "title: Ship it\nstatus: open\n";
        for (detector, name) in [
            (has_comment as fn(&str) -> bool, "comment"),
            (has_blank_line_between_fields, "blank line"),
            (has_flow_list, "flow list"),
            (has_a_quoted_value, "quoted value"),
            (has_extra_spacing_after_a_colon, "extra spacing"),
            (has_an_anchor_and_an_alias, "anchor and alias"),
            (has_a_tag, "tag"),
        ] {
            assert!(
                !detector(plain),
                "{name} was found in a block that holds none"
            );
        }

        assert!(has_comment("a: 1\n# a comment\nb: 2\n"));
        assert!(has_blank_line_between_fields("a: 1\n\nb: 2\n"));
        assert!(has_flow_list("a: [1, 2]\n"));
        assert!(has_a_quoted_value("a: 'x'\n"));
        assert!(has_a_quoted_value("a: \"x\"\n"));
        assert!(has_extra_spacing_after_a_colon("a:   1\n"));
        assert!(has_an_anchor_and_an_alias("a: &x 1\nb: *x\n"));
        assert!(has_a_tag("a: !Ref x\n"));
    }

    /// Goal criterion 1's other half: the table of losses is read from the design, not copied
    /// here a second time, so a row the design adds or changes is a row this test reads too.
    #[test]
    fn every_shape_the_design_names_as_lost_is_held_by_some_document_in_the_corpus() {
        let losses =
            typdoc_testkit::design::frontmatter_losses(&typdoc_testkit::fixtures::design_text())
                .unwrap_or_else(|e| panic!("reading the table of losses from the design: {e}"));
        let blocks = raw_blocks();
        assert!(
            !blocks.is_empty(),
            "no fixture document has a frontmatter block to check"
        );

        for row in &losses {
            let holds = detector_for(row).unwrap_or_else(|e| panic!("{e}"));

            assert!(
                blocks.iter().any(|b| holds(b)),
                "no document in the fixtures holds the shape {row:?}, which the design's table \
                 of losses names as something a write does not keep"
            );
        }
    }
}
