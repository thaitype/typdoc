use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::error::Error;

pub const TYPDOC_DIR: &str = ".typdoc";

/// Keys the design gives `config.json` that this version does not read.
const CONFIG_KEYS_NOT_READ: [&str; 4] = ["namespaces", "imports", "validation", "lock"];

/// Keys the design gives a collection file that this version does not read.
const COLLECTION_KEYS_NOT_READ: [&str; 2] = ["refBase", "validation"];

type Rest = serde_json::Map<String, serde_json::Value>;

#[derive(Debug, Deserialize)]
struct ConfigFile {
    version: u64,
    #[serde(flatten)]
    rest: Rest,
}

#[derive(Debug, Deserialize)]
struct CollectionFile {
    #[serde(rename = "match")]
    pattern: String,
    schema: String,
    #[serde(flatten)]
    rest: Rest,
}

#[derive(Debug)]
pub struct Collection {
    pub name: String,
    pub pattern: String,
    /// Relative to the project folder.
    pub schema: String,
    /// The collection file, for messages.
    pub file: PathBuf,
}

pub fn config_file(project: &Path) -> PathBuf {
    project.join(TYPDOC_DIR).join("config.json")
}

pub fn read_json<T: for<'de> Deserialize<'de>>(file: &Path) -> Result<T, Error> {
    let text = fs::read_to_string(file).map_err(Error::io_at(file))?;
    serde_json::from_str(&text).map_err(|e| Error::Config {
        file: file.to_owned(),
        message: e.to_string(),
    })
}

/// Every key a file may hold is either read or refused by name, so none is ignored.
fn refuse_other_keys(file: &Path, rest: &Rest, not_read: &[&str]) -> Result<(), Error> {
    let Some(key) = rest.keys().next() else {
        return Ok(());
    };
    let message = if not_read.contains(&key.as_str()) {
        format!("the key `{key}` is not read yet")
    } else {
        format!("`{key}` is an unknown key")
    };
    Err(Error::Config {
        file: file.to_owned(),
        message,
    })
}

/// Reads `config.json` and refuses what this version does not read.
pub fn check_version(project: &Path) -> Result<(), Error> {
    let file = config_file(project);
    let config: ConfigFile = read_json(&file)?;
    if config.version != 1 {
        return Err(Error::Config {
            file,
            message: format!("version {} is not known", config.version),
        });
    }
    refuse_other_keys(&file, &config.rest, &CONFIG_KEYS_NOT_READ)
}

/// Every `*.json` file in `.typdoc/collections/`, sorted by name.
pub fn collections(project: &Path) -> Result<Vec<Collection>, Error> {
    let dir = project.join(TYPDOC_DIR).join("collections");
    let entries = match fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(source) => return Err(Error::Io { file: dir, source }),
    };
    let mut found = Vec::new();
    for entry in entries {
        let entry = entry.map_err(Error::io_at(&dir))?;
        let file = entry.path();
        let Some(name) = file
            .file_name()
            .and_then(|n| n.to_str())
            .and_then(|n| n.strip_suffix(".json"))
        else {
            continue;
        };
        let parsed: CollectionFile = read_json(&file)?;
        refuse_other_keys(&file, &parsed.rest, &COLLECTION_KEYS_NOT_READ)?;
        found.push(Collection {
            name: name.to_owned(),
            pattern: parsed.pattern,
            schema: parsed.schema,
            file,
        });
    }
    found.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(found)
}

impl Collection {
    /// Only names in the project folder, with `*`, are read.
    pub fn check_pattern(&self) -> Result<(), Error> {
        let pattern = &self.pattern;
        if pattern.contains(['/', '{', '}']) || pattern.contains("**") {
            return Err(Error::Config {
                file: self.file.clone(),
                message: format!(
                    "the match `{pattern}` is not read yet: only names in the project folder, with `*`, are"
                ),
            });
        }
        Ok(())
    }
}
