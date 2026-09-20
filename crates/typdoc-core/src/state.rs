//! The state file, `.typdoc/state/<namespace>.json`: the highest number `typdoc new` has
//! allocated per collection in one namespace. Read only in this story (contract: "`state/<
//! namespace>.json` is read and never written"; the allocation itself, and the writer, are
//! story 2's, per decision 2, "The state file's `last` per collection is read for
//! `state.missing`, the allocation itself is story 2").

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use crate::config::Namespace;
use crate::error::Error;

const STATE_DIR: &str = ".typdoc/state";

/// The path of a namespace's state file, relative to the project folder.
pub(crate) fn file_path(namespace: &str) -> String {
    format!("{STATE_DIR}/{namespace}.json")
}

/// One namespace's state file: the `last` recorded for each collection named there, by the
/// collection's name. An entry with no `last`, or one whose value is not a whole number, is
/// read as not recorded at all: no id in the design's table is about the shape of one entry (a
/// state file is never hand-edited under the design's own account: it is written, and
/// re-verified, by `typdoc new`), and `state.missing` already reports a collection with no
/// usable record the same way it reports one with none.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct StateFile {
    pub last: BTreeMap<String, u64>,
}

impl StateFile {
    pub(crate) fn has(&self, collection: &str) -> bool {
        self.last.contains_key(collection)
    }
}

/// Reads one namespace's state file. A missing file is not an error: an empty `StateFile` is
/// exactly what "no record kept yet" means, the same reading `state.missing` gives a namespace
/// whose collection is new. A file that cannot be read as a JSON object has no id of its own in
/// the design's table, so it stops the command the same id-less way an unreadable schema
/// already does (`schema::load`'s own `Schema::parse` failure).
pub(crate) fn read(root: &Path, namespace: &str) -> Result<StateFile, Error> {
    let relative = file_path(namespace);
    let file = root.join(&relative);
    let bytes = match fs::read(&file) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(StateFile::default()),
        Err(source) => return Err(Error::Io { file, source }),
    };
    let value: serde_json::Value = serde_json::from_slice(&bytes).map_err(|e| Error::Config {
        file: file.clone(),
        message: format!("cannot be parsed: {e}"),
    })?;
    let serde_json::Value::Object(top) = value else {
        return Err(Error::Config {
            file,
            message: "must be one JSON object".to_owned(),
        });
    };
    let mut last = BTreeMap::new();
    for (name, entry) in top {
        if let Some(value) = entry.get("last").and_then(serde_json::Value::as_u64) {
            last.insert(name, value);
        }
    }
    Ok(StateFile { last })
}

/// The state files in `.typdoc/state/` that match no namespace of `namespaces`
/// (`config.state-orphan`), by their path from the project folder, sorted. Every `*.json` file
/// there is read as a state file, the same way `.typdoc/collections/` treats its own files;
/// anything else is ignored.
pub(crate) fn orphans(root: &Path, namespaces: &[Namespace]) -> Result<Vec<String>, Error> {
    let dir = root.join(STATE_DIR);
    let entries = match fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(source) => return Err(Error::Io { file: dir, source }),
    };
    let known: std::collections::BTreeSet<&str> =
        namespaces.iter().map(|n| n.name.as_str()).collect();
    let mut found = Vec::new();
    for entry in entries {
        let entry = entry.map_err(Error::io_at(&dir))?;
        let file_name = entry.file_name().to_string_lossy().into_owned();
        let Some(name) = file_name.strip_suffix(".json") else {
            continue;
        };
        if !known.contains(name) {
            found.push(format!("{STATE_DIR}/{file_name}"));
        }
    }
    found.sort();
    Ok(found)
}

// No filesystem-touching unit tests here: every other module that reads project files
// (`config.rs`, `schema.rs`, `index.rs`) is exercised through the built binary in
// `crates/typdoc/tests`, at the seam the design's own testing strategy names ("what a person or
// a program can see: output, exit codes"), and this module follows the same precedent — see
// `crates/typdoc/tests/state.rs`.
