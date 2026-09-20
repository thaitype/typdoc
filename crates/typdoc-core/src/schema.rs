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

/// A `bool` option (`required`, `acyclic`, `override`), read tolerantly: a value that is not
/// `true` or `false` is kept so `schema.valid` can report it as an invalid option instead of
/// refusing the whole schema, and reads as `false` wherever the option's value is used (ticket
/// 5 read `required` strictly, which crashed the read before `schema.valid` could report it;
/// this is the relaxing that ticket left for this ticket to make).
#[derive(Debug, Clone, PartialEq)]
pub enum OptBool {
    Given(bool),
    /// Written, but not `true` or `false`.
    Invalid(serde_json::Value),
}

impl OptBool {
    fn value(&self) -> bool {
        matches!(self, OptBool::Given(true))
    }
}

impl<'de> Deserialize<'de> for OptBool {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = serde_json::Value::deserialize(deserializer)?;
        Ok(match value {
            serde_json::Value::Bool(b) => OptBool::Given(b),
            other => OptBool::Invalid(other),
        })
    }
}

/// One field as its schema declares it.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Field {
    #[serde(rename = "type")]
    pub kind: FieldType,
    #[serde(default)]
    pub required: Option<OptBool>,
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
    pub acyclic: Option<OptBool>,
    #[serde(default)]
    pub auto: Option<Auto>,
    #[serde(default, rename = "override")]
    pub overrides: Option<OptBool>,
    /// Any key besides the ones above, kept so `schema.valid` can report it as an option the
    /// format does not have.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

impl Field {
    pub fn is_required(&self) -> bool {
        self.required.as_ref().is_some_and(OptBool::value)
    }

    pub fn is_acyclic(&self) -> bool {
        self.acyclic.as_ref().is_some_and(OptBool::value)
    }

    pub fn is_override(&self) -> bool {
        self.overrides.as_ref().is_some_and(OptBool::value)
    }
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
    /// A resolved schema built directly from its fields, with no `extends` chain read: for a
    /// caller that has already merged a chain itself, or a test that checks a condition against
    /// a schema it writes by hand.
    pub fn new(name: String, code: Option<String>, fields: BTreeMap<String, Field>) -> Resolved {
        Resolved { name, code, fields }
    }

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
    is_scheme_name(scheme).then_some(scheme)
}

/// Whether `text` has the shape of a URL scheme (a letter, then letters, digits, `+`, `-` or
/// `.`), whether or not it is followed by `://`. Used both to tell a schema reference's URL
/// scheme from a path and, under `schema.valid`, to catch an import name that would collide
/// with one.
pub(crate) fn is_scheme_name(text: &str) -> bool {
    let mut chars = text.chars();
    chars.next().is_some_and(|c| c.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
}

/// A path with `.` and empty segments removed and each `..` taken against the segment before it.
/// Shared with `refs`, which joins a relative ref against its base the same way a schema
/// reference is joined against the folder that names it.
pub(crate) fn normalize(path: &str) -> String {
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

/// One schema of a chain of `extends`, with the file it was read from (relative to the project
/// folder), so a message can name it.
#[derive(Debug, Clone)]
pub(crate) struct ChainLink {
    pub path: String,
    pub schema: Schema,
}

/// What `load` found: the chain from the collection's own schema to its most distant parent,
/// and, when `extends` forms a cycle, the file whose `extends` closes it and the path it names
/// (already earlier in `chain`), for `schema.valid` to report.
pub(crate) struct Loaded {
    pub chain: Vec<ChainLink>,
    pub cycle: Option<(String, String)>,
}

/// The schema a collection names, read from the project folder with the schemas it extends.
/// A fault of the collection's config goes to `report` and gives `None`. Remote schemas are
/// not fetched or read yet: a URL other than `http://` and `https://` is a config error, and
/// one of those two stops the run. A schema that extends one already read ends the chain there,
/// with the cycle recorded for `schema.valid` to reject; ticket 5 left it ending silently.
pub(crate) fn load(
    root: &Path,
    collection: &Collection,
    report: &mut Report,
) -> Result<Option<Loaded>, Error> {
    let mut chain: Vec<ChainLink> = Vec::new();
    let mut seen = BTreeSet::new();
    // What the next schema is called in the config, and the file that says so.
    let mut reference = collection.schema.clone();
    let mut said_in = collection.path.clone();
    let mut extended_by: Option<String> = None;
    let mut cycle = None;
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
            cycle = Some((said_in.clone(), path));
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
        chain.push(ChainLink {
            path: path.clone(),
            schema,
        });
        let Some(parent) = parent else { break };
        reference = parent;
        said_in = path.clone();
        extended_by = Some(path);
    }
    Ok(Some(Loaded { chain, cycle }))
}

/// The chain from the collection's schema up to its most distant parent. `pub(crate)` so a
/// test elsewhere in the crate can build a `Resolved` from a `Schema` it wrote by hand, without
/// going through a project on disk.
pub(crate) fn merge(chain: &[ChainLink]) -> Resolved {
    let mut fields = BTreeMap::new();
    for link in chain.iter().rev() {
        fields.extend(link.schema.fields.clone());
    }
    let own = &chain[0].schema;
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

/// A fault `schema.valid` reports: the schema file it is about, the field it names when it is
/// about one, and the message.
pub(crate) struct Problem {
    pub path: String,
    pub field: Option<String>,
    pub message: String,
}

fn problem(path: &str, field: Option<&str>, message: String) -> Problem {
    Problem {
        path: path.to_owned(),
        field: field.map(str::to_owned),
        message,
    }
}

/// The message for a boolean option (`required`, `override`, `acyclic`) written as something
/// other than `true` or `false`.
fn not_a_bool_message(option: &str, field: &str, written: &serde_json::Value) -> String {
    format!("the option `{option}` of `{field}` is {written}: it is `true` or `false`")
}

/// Names reserved for a document's own pseudo-fields: a schema field may not use one, and
/// neither may a name starting with `$`.
const RESERVED_FIELD_NAMES: &[&str] = &["path", "key", "code", "collection", "schema", "namespace"];

/// Whether `name` fits the field naming rule, `[A-Za-z_][A-Za-z0-9_-]*`.
fn plain_field_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// What `schema.valid` finds in one field: a name that breaks the naming rule or uses a
/// reserved name, and an option that is not valid for the field (a boolean written as something
/// else, an unknown type, `auto` or `target` name, an option that does not apply to the field's
/// type, or a key the format has no option of).
fn check_field(path: &str, name: &str, field: &Field, problems: &mut Vec<Problem>) {
    if let Some(message) = field_name_problem(name) {
        problems.push(problem(path, Some(name), message));
    }
    if let FieldType::Other(other) = &field.kind {
        problems.push(problem(
            path,
            Some(name),
            format!("the type `{other}` is not a field type"),
        ));
    }
    for (option, value) in [
        ("required", &field.required),
        ("override", &field.overrides),
        ("acyclic", &field.acyclic),
    ] {
        if let Some(OptBool::Invalid(written)) = value {
            problems.push(problem(
                path,
                Some(name),
                not_a_bool_message(option, name, written),
            ));
        }
    }
    let is_enum = field.kind == FieldType::Enum;
    let is_ref = matches!(field.kind, FieldType::Ref | FieldType::RefList);
    if field.values.is_some() && !is_enum {
        problems.push(problem(
            path,
            Some(name),
            format!(
                "`values` does not apply to the type `{}`",
                field.kind.name()
            ),
        ));
    }
    if field.transitions.is_some() && !is_enum {
        problems.push(problem(
            path,
            Some(name),
            format!(
                "`transitions` does not apply to the type `{}`",
                field.kind.name()
            ),
        ));
    }
    if let Some(target) = &field.target {
        if is_ref {
            if let Target::Other(text) = target {
                problems.push(problem(
                    path,
                    Some(name),
                    format!("`target` is `{text}`: it is `*` or a list of schema names"),
                ));
            }
        } else {
            problems.push(problem(
                path,
                Some(name),
                format!(
                    "`target` does not apply to the type `{}`",
                    field.kind.name()
                ),
            ));
        }
    }
    // The option's own value was already checked above, alongside `required` and `override`;
    // only a value of the right shape (`true` or `false`) needs a type-applicability check.
    if matches!(field.acyclic, Some(OptBool::Given(_))) && !is_ref {
        problems.push(problem(
            path,
            Some(name),
            format!(
                "`acyclic` does not apply to the type `{}`",
                field.kind.name()
            ),
        ));
    }
    if let Some(auto) = &field.auto {
        let applies = matches!(
            field.kind,
            FieldType::Date | FieldType::Datetime | FieldType::List
        );
        match auto {
            Auto::Other(other) => problems.push(problem(
                path,
                Some(name),
                format!("the option `auto` of `{name}` is `{other}`: it is `create`, `update` or `moves`"),
            )),
            Auto::Moves if field.kind != FieldType::List => problems.push(problem(
                path,
                Some(name),
                "`auto: moves` applies to the type `list` only".to_owned(),
            )),
            _ if !applies => problems.push(problem(
                path,
                Some(name),
                format!("`auto` does not apply to the type `{}`", field.kind.name()),
            )),
            _ => {}
        }
    }
    for key in field.extra.keys() {
        problems.push(problem(
            path,
            Some(name),
            format!("`{key}` is not an option of a field"),
        ));
    }
}

fn field_name_problem(name: &str) -> Option<String> {
    if let Some(rest) = name.strip_prefix('$') {
        return Some(format!(
            "the field name `{name}` starts with `$`: `${rest}` is reserved for a virtual field"
        ));
    }
    if RESERVED_FIELD_NAMES.contains(&name) {
        return Some(format!(
            "the field name `{name}` is reserved: it is already a name of the document itself"
        ));
    }
    if !plain_field_name(name) {
        return Some(format!(
            "the field name `{name}` does not fit the naming rule [A-Za-z_][A-Za-z0-9_-]*"
        ));
    }
    None
}

/// Keeps every schema file `schema.valid` has already checked, project-wide, so a schema shared
/// through `extends` by more than one collection is checked once.
#[derive(Default)]
pub(crate) struct Checked {
    fields: BTreeSet<String>,
    cycles: BTreeSet<String>,
}

/// The `schema.valid` findings of one collection's chain: an extends cycle, an undeclared
/// override and, for every schema file not already checked through another collection, its
/// field names and options. `state` is threaded across every collection of the project so a
/// shared parent is checked once and a cycle is reported once.
pub(crate) fn check(loaded: &Loaded, state: &mut Checked) -> Vec<Problem> {
    let mut problems = Vec::new();
    if let Some((said_in, target)) = &loaded.cycle
        && state.cycles.insert(said_in.clone())
    {
        problems.push(problem(
            said_in,
            None,
            format!("`extends` reaches `{target}` again: the chain cycles"),
        ));
    }
    let mut inherited: BTreeSet<String> = BTreeSet::new();
    for link in loaded.chain.iter().rev() {
        if state.fields.insert(link.path.clone()) {
            for (name, field) in &link.schema.fields {
                check_field(&link.path, name, field, &mut problems);
                if inherited.contains(name) && !field.is_override() {
                    problems.push(problem(
                        &link.path,
                        Some(name),
                        format!(
                            "the field `{name}` redefines a field of a schema it extends, without `\"override\": true`"
                        ),
                    ));
                }
            }
        }
        inherited.extend(link.schema.fields.keys().cloned());
    }
    problems
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
