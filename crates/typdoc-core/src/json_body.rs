//! Reading a catalog document's JSON body (SPC-11).

use std::collections::BTreeMap;

use serde::de::DeserializeOwned;

use crate::document::Value;
use crate::frontmatter;
use crate::schema::Resolved;

const CONTENT_TYPE: &str = "content_type";

const JSON: &str = "json";

/// Why [`read_json_body`] could not read a document's body into the type it was asked for.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum JsonBodyError {
    /// The frontmatter block could not be read at all (for example, never closed). Reported as
    /// itself rather than folded into [`JsonBodyError::MissingContentType`].
    #[error("the frontmatter block could not be read: {0}")]
    Frontmatter(String),

    #[error("the document has no `content_type` field")]
    MissingContentType,

    /// `content_type` is present but is not `json`. `found` describes what was there instead,
    /// for a message a person can act on.
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
/// `T`, deciding from the document's own `content_type` field alone.
pub fn read_json_body<T: DeserializeOwned>(file: &str) -> Result<T, JsonBodyError> {
    let split = frontmatter::split(file).map_err(JsonBodyError::Frontmatter)?;
    let fields = match split.block {
        Some(block) => {
            // An empty schema leaves every field as written, uncoerced.
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
        let a = "---\ntitle: docs/design/catalog/rules.md\ncontent_type: json\n---\n\n[1]\n";
        let b = "---\ntitle: totally/unrelated/path.md\ncontent_type: json\n---\n\n[1]\n";

        let a_value: Vec<u32> = read_json_body(a).expect("a");
        let b_value: Vec<u32> = read_json_body(b).expect("b");

        assert_eq!(a_value, b_value);
    }

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
