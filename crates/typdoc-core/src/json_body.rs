//! The central helper for reading a typdoc document's body as JSON (ticket 11).
//!
//! A catalog document (`docs/design/catalog/*.md`) carries its body's shape in its own
//! `content_type` frontmatter field rather than in its path or its schema's name, so a caller
//! that wants typed data out of one reads that field itself, here, instead of guessing from
//! where the file lives. [`read_json_body`] is the one place that decision is made: it reads the
//! frontmatter block the same way the rest of typdoc-core does ([`crate::frontmatter`]), looks
//! at `content_type`, and only for `content_type: json` parses the body with `serde_json` into
//! whatever type the caller asks for. Anything else -- the field missing, holding a value this
//! helper does not recognize, or a body that is not valid JSON despite declaring it should be --
//! is its own distinct, readable [`JsonBodyError`], never a silent fallback to treating the
//! document as plain prose.

use std::collections::BTreeMap;

use serde::de::DeserializeOwned;

use crate::document::Value;
use crate::frontmatter;
use crate::schema::Resolved;

/// The one frontmatter field [`read_json_body`] dispatches on. Not a path, not a schema name --
/// this field, on the document itself.
const CONTENT_TYPE: &str = "content_type";

/// The only `content_type` this helper recognizes today (contract decision 2: "Mild's own name,
/// not `body-type`"; the enum it drives has one value, `json`).
const JSON: &str = "json";

/// Why [`read_json_body`] could not read a document's body into the type it was asked for. Each
/// case is its own variant on purpose -- a caller (or a test) that wants to tell "no
/// `content_type` at all" apart from "a `content_type` this helper doesn't know" apart from "said
/// `json` but wasn't" never has to parse a message to do it.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum JsonBodyError {
    /// The frontmatter block itself could not be read at all (for example, never closed). Not
    /// one of ticket 11's three named cases, but a document this broken has no `content_type` to
    /// dispatch on either, so it is reported for what it is rather than folded into
    /// [`JsonBodyError::MissingContentType`].
    #[error("the frontmatter block could not be read: {0}")]
    Frontmatter(String),

    /// No `content_type` field is present in the frontmatter block at all.
    #[error("the document has no `content_type` field")]
    MissingContentType,

    /// `content_type` is present but is not a value this helper recognizes -- today, only `json`
    /// is. `found` describes what was there instead, for a message a person can act on.
    #[error(
        "the document's `content_type` is {found}, which this helper does not recognize (only \
         `json` is)"
    )]
    UnrecognizedContentType { found: String },

    /// `content_type: json` was declared, but the text under it does not deserialize into the
    /// type asked for -- whether because it is not valid JSON at all, or because it is valid
    /// JSON of the wrong shape.
    #[error(
        "the document declares `content_type: json` but its body does not parse as the \
         requested type: {0}"
    )]
    InvalidJson(String),
}

/// Describes a non-`json` `content_type` value for [`JsonBodyError::UnrecognizedContentType`],
/// in a form that reads naturally after "is": `` `yaml` ``, `an empty field`, `a list`, and so
/// on. A frontmatter value that survived `frontmatter::fields` is always one of these shapes
/// ([`crate::document::Value`]).
fn describe(value: &Value) -> String {
    match value {
        Value::Text(text) => format!("`{text}`"),
        Value::Empty => "an empty field".to_owned(),
        Value::List(_) => "a list".to_owned(),
        Value::Number(_) => "a number".to_owned(),
        Value::Bool(_) => "a bool".to_owned(),
        Value::Date(_) => "a date".to_owned(),
        Value::Datetime(_) => "a datetime".to_owned(),
    }
}

/// Reads `file` (a whole document, frontmatter block and body, exactly as it sits on disk) into
/// `T`, dispatching entirely on the document's own `content_type` frontmatter field -- nothing
/// here reads or asks for the document's path, so nothing about the result can depend on where
/// the file lives (ticket 6's requirement, restated by ticket 11).
///
/// The frontmatter block is read with [`crate::frontmatter::split`] and [`crate::frontmatter::fields`],
/// the same functions the rest of typdoc-core reads a document's fields with, against a schema
/// that names no fields -- this helper only ever looks at `content_type` itself, so every other
/// field is left exactly as written and never coerced into something it might not be.
pub fn read_json_body<T: DeserializeOwned>(file: &str) -> Result<T, JsonBodyError> {
    let split = frontmatter::split(file).map_err(JsonBodyError::Frontmatter)?;
    let fields = match split.block {
        Some(block) => {
            // The name and code below are never read by `frontmatter::fields` (only
            // `schema.field(name)` is, and an empty field map always answers `None` for it),
            // so every field -- `content_type` included -- comes back exactly as written rather
            // than coerced into a type this helper never declared. Left empty/`None` rather than
            // naming a real schema on purpose: nothing here should look like it is standing in
            // for `.typdoc/schemas/catalog.json`.
            let schema = Resolved::new(String::new(), None, BTreeMap::new());
            frontmatter::fields(block, &schema).map_err(JsonBodyError::Frontmatter)?
        }
        None => Vec::new(),
    };

    let content_type = fields
        .iter()
        .find(|(name, _)| name == CONTENT_TYPE)
        .map(|(_, value)| value);

    match content_type {
        None => Err(JsonBodyError::MissingContentType),
        Some(Value::Text(text)) if text == JSON => {
            let body = &file[split.body..];
            serde_json::from_str(body).map_err(|e| JsonBodyError::InvalidJson(e.to_string()))
        }
        Some(other) => Err(JsonBodyError::UnrecognizedContentType {
            found: describe(other),
        }),
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    #[test]
    fn a_document_missing_content_type_fails_with_its_own_distinct_error() {
        let file = "---\ntitle: No content type\n---\n\n{}\n";

        let error = read_json_body::<serde_json::Value>(file).unwrap_err();

        assert_eq!(error, JsonBodyError::MissingContentType);
    }

    #[test]
    fn a_document_with_no_frontmatter_block_at_all_is_reported_as_its_own_case() {
        let file = "just a body, no frontmatter\n";

        let error = read_json_body::<serde_json::Value>(file).unwrap_err();

        assert_eq!(error, JsonBodyError::MissingContentType);
    }

    #[test]
    fn a_document_with_an_unrecognized_content_type_fails_with_its_own_distinct_error() {
        let file = "---\ntitle: Wrong content type\ncontent_type: yaml\n---\n\n{}\n";

        let error = read_json_body::<serde_json::Value>(file).unwrap_err();

        assert_eq!(
            error,
            JsonBodyError::UnrecognizedContentType {
                found: "`yaml`".to_owned()
            }
        );
    }

    #[test]
    fn a_content_type_field_with_no_value_is_unrecognized_not_missing() {
        // `content_type:` with nothing after it is present (a field with a name), just not
        // `json` -- told apart from an absent field the same way the rest of typdoc-core tells
        // `Value::Empty` apart from a field that was never written at all.
        let file = "---\ntitle: Bare content type\ncontent_type:\n---\n\n{}\n";

        let error = read_json_body::<serde_json::Value>(file).unwrap_err();

        assert_eq!(
            error,
            JsonBodyError::UnrecognizedContentType {
                found: "an empty field".to_owned()
            }
        );
    }

    #[test]
    fn a_body_that_is_not_valid_json_despite_declaring_content_type_json_fails_with_its_own_distinct_error()
     {
        let file = "---\ntitle: Broken JSON\ncontent_type: json\n---\n\nnot json at all\n";

        let error = read_json_body::<serde_json::Value>(file).unwrap_err();

        assert!(
            matches!(error, JsonBodyError::InvalidJson(_)),
            "expected InvalidJson, got {error:?}"
        );
    }

    #[test]
    fn the_three_error_cases_are_distinct_from_each_other() {
        let missing =
            read_json_body::<serde_json::Value>("---\ntitle: x\n---\n\n{}\n").unwrap_err();
        let unrecognized =
            read_json_body::<serde_json::Value>("---\ntitle: x\ncontent_type: xml\n---\n\n{}\n")
                .unwrap_err();
        let invalid =
            read_json_body::<serde_json::Value>("---\ntitle: x\ncontent_type: json\n---\n\nnope\n")
                .unwrap_err();

        assert_ne!(missing, unrecognized);
        assert_ne!(missing, invalid);
        assert_ne!(unrecognized, invalid);
    }

    #[test]
    fn a_content_type_json_document_round_trips_a_caller_given_type() {
        #[derive(Debug, Deserialize, PartialEq, Eq)]
        struct Payload {
            numbers: Vec<u32>,
        }

        let file = "---\ntitle: Good\ncontent_type: json\n---\n\n{\"numbers\": [1, 2, 3]}\n";

        let payload: Payload = read_json_body(file).expect("a valid json body");

        assert_eq!(
            payload,
            Payload {
                numbers: vec![1, 2, 3]
            }
        );
    }

    #[test]
    fn dispatch_never_looks_at_a_path_because_none_is_ever_given() {
        // Ticket 6's requirement, restated by ticket 11: nothing about `read_json_body`'s
        // behavior may depend on the document's path. The strongest proof available at this
        // seam is structural: the function's only input is the file's own text, so there is no
        // path for it to read even by accident. Two files with unrelated "paths" implied by
        // their content but identical frontmatter and body must behave identically.
        let a = "---\ntitle: docs/design/catalog/rules.md\ncontent_type: json\n---\n\n[1]\n";
        let b = "---\ntitle: totally/unrelated/path.md\ncontent_type: json\n---\n\n[1]\n";

        let a_value: Vec<u32> = read_json_body(a).expect("a");
        let b_value: Vec<u32> = read_json_body(b).expect("b");

        assert_eq!(a_value, b_value);
    }

    /// Ticket 11's fourth test: each of ticket 10's four real catalog documents round-trips
    /// through the helper into a type matching its actual content.
    mod real_catalog_documents {
        use super::*;

        fn read(relative: &str) -> String {
            let path = typdoc_testkit::fixtures::root().join(relative);
            std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
        }

        #[derive(Debug, Deserialize, PartialEq, Eq)]
        struct RuleEntry {
            id: String,
            configurable: bool,
        }

        #[derive(Debug, Deserialize, PartialEq, Eq)]
        struct RulesCatalog {
            rules: Vec<RuleEntry>,
        }

        #[derive(Debug, Deserialize, PartialEq, Eq)]
        struct CommandsCatalog {
            commands: Vec<String>,
        }

        #[derive(Debug, Deserialize, PartialEq, Eq)]
        struct ExitCodesCatalog {
            codes: Vec<i64>,
        }

        #[derive(Debug, Deserialize, PartialEq, Eq)]
        struct FrontmatterLossesCatalog {
            losses: Vec<String>,
        }

        #[test]
        fn rules_md_round_trips_into_the_rules_catalog_shape() {
            let file = read("docs/design/catalog/rules.md");

            let catalog: RulesCatalog = read_json_body(&file).expect("rules.md is valid json");

            assert_eq!(catalog.rules.len(), 24, "{:?}", catalog.rules);
            assert!(
                catalog
                    .rules
                    .iter()
                    .any(|entry| entry.id == "schema.valid" && !entry.configurable)
            );
            assert!(
                catalog
                    .rules
                    .iter()
                    .any(|entry| entry.id == "body.links" && entry.configurable)
            );
        }

        #[test]
        fn commands_md_round_trips_into_the_commands_catalog_shape() {
            let file = read("docs/design/catalog/commands.md");

            let catalog: CommandsCatalog =
                read_json_body(&file).expect("commands.md is valid json");

            assert_eq!(
                catalog.commands,
                vec![
                    "new", "get", "list", "set", "toc", "refs", "mv", "pull", "validate"
                ]
            );
        }

        #[test]
        fn exit_codes_md_round_trips_into_the_exit_codes_catalog_shape() {
            let file = read("docs/design/catalog/exit-codes.md");

            let catalog: ExitCodesCatalog =
                read_json_body(&file).expect("exit-codes.md is valid json");

            assert_eq!(catalog.codes, vec![0, 1, 2, 3, 4, 5, 6, 7]);
        }

        #[test]
        fn frontmatter_losses_md_round_trips_into_the_losses_catalog_shape() {
            let file = read("docs/design/catalog/frontmatter-losses.md");

            let catalog: FrontmatterLossesCatalog =
                read_json_body(&file).expect("frontmatter-losses.md is valid json");

            assert_eq!(catalog.losses.len(), 7, "{:?}", catalog.losses);
            assert!(
                catalog
                    .losses
                    .iter()
                    .any(|loss| loss == "Comments, anywhere in the block")
            );
        }
    }
}
