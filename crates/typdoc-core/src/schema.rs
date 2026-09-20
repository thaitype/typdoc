//! The schema format: what a schema file declares, and the schema a collection sees once the
//! schemas it extends are merged in. Whether a schema is right is not decided here.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use serde::{Deserialize, Deserializer};

use crate::config::{Collection, Report};
use crate::error::Error;

/// The type of a field. A name the format does not have is kept, so that a schema that uses it
/// can be read and its values shown as written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldType {
    String,
    Number,
    Bool,
    Date,
    Datetime,
    Enum,
    List,
    Ref,
    RefList,
    Other(String),
}

impl FieldType {
    fn named(name: &str) -> FieldType {
        match name {
            "string" => FieldType::String,
            "number" => FieldType::Number,
            "bool" => FieldType::Bool,
            "date" => FieldType::Date,
            "datetime" => FieldType::Datetime,
            "enum" => FieldType::Enum,
            "list" => FieldType::List,
            "ref" => FieldType::Ref,
            "ref[]" => FieldType::RefList,
            other => FieldType::Other(other.to_owned()),
        }
    }

    /// The name a message can show for this type, the same word the schema is written with.
    pub fn name(&self) -> &str {
        match self {
            FieldType::String => "string",
            FieldType::Number => "number",
            FieldType::Bool => "bool",
            FieldType::Date => "date",
            FieldType::Datetime => "datetime",
            FieldType::Enum => "enum",
            FieldType::List => "list",
            FieldType::Ref => "ref",
            FieldType::RefList => "ref[]",
            FieldType::Other(name) => name,
        }
    }
}

impl<'de> Deserialize<'de> for FieldType {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer).map(|name| FieldType::named(&name))
    }
}

/// What `auto` asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Auto {
    Create,
    Update,
    Moves,
    Other(String),
}

impl<'de> Deserialize<'de> for Auto {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let name = String::deserialize(deserializer)?;
        Ok(match name.as_str() {
            "create" => Auto::Create,
            "update" => Auto::Update,
            "moves" => Auto::Moves,
            _ => Auto::Other(name),
        })
    }
}

/// The schemas a ref field may point at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    /// `"*"`: any file.
    Any,
    /// Schema names, bare or qualified, as written.
    Schemas(Vec<String>),
    /// A text that is not `*`.
    Other(String),
}

impl<'de> Deserialize<'de> for Target {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Written {
            Text(String),
            Names(Vec<String>),
        }
        Ok(match Written::deserialize(deserializer)? {
            Written::Text(text) if text == "*" => Target::Any,
            Written::Text(text) => Target::Other(text),
            Written::Names(names) => Target::Schemas(names),
        })
    }
}

/// One field as its schema declares it.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Field {
    #[serde(rename = "type")]
    pub kind: FieldType,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub default: Option<serde_json::Value>,
    /// The allowed values of an `enum`, in order; the order is also the sort order.
    #[serde(default)]
    pub values: Option<Vec<String>>,
    /// Value to the values that may follow it.
    #[serde(default)]
    pub transitions: Option<BTreeMap<String, Vec<String>>>,
    #[serde(default)]
    pub target: Option<Target>,
    #[serde(default)]
    pub acyclic: bool,
    #[serde(default)]
    pub auto: Option<Auto>,
    #[serde(default, rename = "override")]
    pub overrides: bool,
}

/// One schema file as it is written.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Schema {
    pub name: String,
    #[serde(default)]
    pub code: Option<String>,
    /// A relative path or a URL, as written.
    #[serde(default)]
    pub extends: Option<String>,
    pub fields: BTreeMap<String, Field>,
}

impl Schema {
    pub fn parse(text: &str) -> Result<Schema, serde_json::Error> {
        serde_json::from_str(text)
    }
}

/// The schema of a collection: its own name and code, and its fields with the fields of every
/// schema it extends, a field of a child replacing the field of a parent of the same name.
#[derive(Debug, Clone, PartialEq)]
pub struct Resolved {
    pub name: String,
    pub code: Option<String>,
    fields: BTreeMap<String, Field>,
}

impl Resolved {
    pub fn field(&self, name: &str) -> Option<&Field> {
        self.fields.get(name)
    }

    /// Every field of the schema, by name, in no particular order.
    pub fn fields(&self) -> impl Iterator<Item = (&str, &Field)> {
        self.fields
            .iter()
            .map(|(name, field)| (name.as_str(), field))
    }
}

/// The scheme of a URL such as `ftp://host/x`, and `None` for a path.
fn scheme_of(reference: &str) -> Option<&str> {
    let (scheme, _) = reference.split_once("://")?;
    let mut chars = scheme.chars();
    let valid = chars.next().is_some_and(|c| c.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'));
    valid.then_some(scheme)
}

/// A path with `.` and empty segments removed and each `..` taken against the segment before it.
fn normalize(path: &str) -> String {
    let mut kept: Vec<&str> = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." if kept.last().is_some_and(|last| *last != "..") => {
                kept.pop();
            }
            other => kept.push(other),
        }
    }
    let joined = kept.join("/");
    if path.starts_with('/') {
        format!("/{joined}")
    } else {
        joined
    }
}

/// A reference written in the schema file `base`, as a path from the project folder.
fn from_file(base: &str, reference: &str) -> String {
    if reference.starts_with('/') {
        return normalize(reference);
    }
    match base.rsplit_once('/') {
        Some((folder, _)) => normalize(&format!("{folder}/{reference}")),
        None => normalize(reference),
    }
}

/// The schema a collection names, read from the project folder with the schemas it extends.
/// A fault of the collection's config goes to `report` and gives `None`. Remote schemas are
/// not fetched or read yet: a URL other than `http://` and `https://` is a config error, and
/// one of those two stops the run. A schema that extends one already read ends the chain there;
/// what that means is for the check on schemas.
pub(crate) fn load(
    root: &Path,
    collection: &Collection,
    report: &mut Report,
) -> Result<Option<Resolved>, Error> {
    let mut chain: Vec<Schema> = Vec::new();
    let mut seen = BTreeSet::new();
    // What the next schema is called in the config, and the file that says so.
    let mut reference = collection.schema.clone();
    let mut said_in = collection.path.clone();
    let mut extended_by: Option<String> = None;
    loop {
        if let Some(scheme) = scheme_of(&reference) {
            if matches!(scheme, "http" | "https") {
                return Err(Error::Config {
                    file: root.join(&said_in),
                    message: format!(
                        "the schema `{reference}` is a URL, and remote schemas are not read yet"
                    ),
                });
            }
            report.add(
                "config.schema-url",
                &said_in,
                format!(
                    "the schema `{reference}` has the scheme `{scheme}`: only `http://` and `https://` are read"
                ),
            );
            return Ok(None);
        }
        let path = match &extended_by {
            None => normalize(&reference),
            Some(base) => from_file(base, &reference),
        };
        if !seen.insert(path.clone()) {
            break;
        }
        let file = root.join(&path);
        let text = match fs::read_to_string(&file) {
            Ok(text) => text,
            Err(e) if path.is_empty() || e.kind() == std::io::ErrorKind::NotFound => {
                let message = match &extended_by {
                    None => format!("the schema `{reference}` does not exist"),
                    Some(base) => {
                        format!("the schema `{reference}`, which `{base}` extends, does not exist")
                    }
                };
                report.add("config.collection-schema", &collection.path, message);
                return Ok(None);
            }
            Err(source) => return Err(Error::Io { file, source }),
        };
        let schema = Schema::parse(&text).map_err(|e| Error::Config {
            file: file.clone(),
            message: e.to_string(),
        })?;
        let parent = schema.extends.clone();
        chain.push(schema);
        let Some(parent) = parent else { break };
        reference = parent;
        said_in = path.clone();
        extended_by = Some(path);
    }
    Ok(Some(merge(&chain)))
}

/// The chain from the collection's schema up to its most distant parent. `pub(crate)` so a
/// test elsewhere in the crate can build a `Resolved` from a `Schema` it wrote by hand, without
/// going through a project on disk.
pub(crate) fn merge(chain: &[Schema]) -> Resolved {
    let mut fields = BTreeMap::new();
    for schema in chain.iter().rev() {
        fields.extend(schema.fields.clone());
    }
    let own = &chain[0];
    Resolved {
        name: own.name.clone(),
        code: own.code.clone(),
        fields,
    }
}

/// The path of the schema a collection names, for telling whether two collections name one.
pub(crate) fn identity(collection: &Collection) -> String {
    match scheme_of(&collection.schema) {
        Some(_) => collection.schema.clone(),
        None => normalize(&collection.schema),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_path_is_read_without_dots_and_with_each_parent_step_taken() {
        assert_eq!(normalize("a/./b.json"), "a/b.json");
        assert_eq!(normalize("a/../b.json"), "b.json");
        assert_eq!(normalize("../b.json"), "../b.json");
        assert_eq!(normalize("a/../../b.json"), "../b.json");
        assert_eq!(normalize("/a//b.json"), "/a/b.json");
        assert_eq!(normalize(""), "");
    }

    #[test]
    fn a_reference_is_read_from_the_folder_of_the_file_that_holds_it() {
        assert_eq!(from_file("schemas/a.json", "./b.json"), "schemas/b.json");
        assert_eq!(from_file("schemas/a.json", "../b.json"), "b.json");
        assert_eq!(from_file("a.json", "b.json"), "b.json");
        assert_eq!(from_file("schemas/a.json", "/abs/b.json"), "/abs/b.json");
    }

    #[test]
    fn only_a_letter_followed_by_letters_digits_plus_dash_or_dot_then_two_slashes_is_a_scheme() {
        assert_eq!(scheme_of("ftp://x"), Some("ftp"));
        assert_eq!(scheme_of("git+ssh://x"), Some("git+ssh"));
        assert_eq!(scheme_of("schemas/a.json"), None);
        assert_eq!(scheme_of("1ab://x"), None);
        assert_eq!(scheme_of("://x"), None);
        assert_eq!(scheme_of("a b://x"), None);
        assert_eq!(scheme_of("mailto:x"), None);
    }
}
