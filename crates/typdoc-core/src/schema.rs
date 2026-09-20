use std::path::Path;

use serde::Deserialize;

use crate::config::read_json;
use crate::error::Error;

#[derive(Debug, Deserialize)]
pub struct Schema {
    pub name: String,
    #[allow(dead_code, reason = "read so that a schema without fields is refused")]
    fields: serde_json::Map<String, serde_json::Value>,
}

/// A schema file named by a collection, relative to the project folder.
pub fn load(project: &Path, relative: &str, collection_file: &Path) -> Result<Schema, Error> {
    if relative.contains("://") {
        return Err(Error::Config {
            file: collection_file.to_owned(),
            message: format!(
                "the schema `{relative}` is a URL, and remote schemas are not read yet"
            ),
        });
    }
    match read_json(&project.join(relative)) {
        Err(Error::Io { source, .. }) if source.kind() == std::io::ErrorKind::NotFound => {
            Err(Error::Config {
                file: collection_file.to_owned(),
                message: format!("the schema `{relative}` does not exist"),
            })
        }
        read => read,
    }
}
