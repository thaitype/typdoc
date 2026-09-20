use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use crate::config::{self, config_file};
use crate::document::Document;
use crate::env::Env;
use crate::error::Error;
use crate::frontmatter;
use crate::index::Index;
use crate::schema;

const NAMESPACE: &str = "default";

/// The folder that holds `.typdoc/config.json`: `TYPDOC_DIR` when it is set, and otherwise
/// the nearest one above the current directory, that directory included.
pub fn discover(env: &dyn Env) -> Result<PathBuf, Error> {
    let cwd = env.current_dir().map_err(Error::io_at(Path::new(".")))?;
    if let Some(dir) = env.var("TYPDOC_DIR").filter(|v| !v.is_empty()) {
        let dir = cwd.join(dir);
        return if config_file(&dir).is_file() {
            Ok(dir)
        } else {
            Err(Error::NoProjectAt { dir })
        };
    }
    cwd.ancestors()
        .find(|dir| config_file(dir).is_file())
        .map(Path::to_owned)
        .ok_or(Error::NoProject { from: cwd })
}

/// The argument of a command that names a document.
#[derive(Debug, PartialEq, Eq)]
pub enum DocumentArg {
    /// Relative to the project folder.
    Path(String),
}

impl DocumentArg {
    pub fn parse(arg: &OsStr) -> Result<DocumentArg, Error> {
        let arg = arg.to_str().ok_or_else(|| {
            Error::BadArgument(format!(
                "{arg:?} is not valid UTF-8, so it names no document"
            ))
        })?;
        if !arg.ends_with(".md") {
            return Err(Error::BadArgument(format!(
                "`{arg}` is not a path: a path ends in `.md`, and a key is not read yet"
            )));
        }
        if arg.starts_with('/') || arg.starts_with("./") || arg.starts_with("../") {
            return Err(Error::BadArgument(format!(
                "`{arg}` is a path on disk, which is not read yet: give the path from the project folder"
            )));
        }
        Ok(DocumentArg::Path(arg.to_owned()))
    }
}

pub struct Project {
    index: Index,
    collections: Vec<Loaded>,
}

struct Loaded {
    name: String,
    schema: String,
}

impl Project {
    pub fn load(root: &Path) -> Result<Project, Error> {
        config::check_version(root)?;
        let collections = config::collections(root)?;
        let mut loaded = Vec::new();
        for collection in &collections {
            let schema = schema::load(root, &collection.schema, &collection.file)?;
            loaded.push(Loaded {
                name: collection.name.clone(),
                schema: schema.name,
            });
        }
        let index = Index::build(root, &collections)?;
        Ok(Project {
            index,
            collections: loaded,
        })
    }

    pub fn get(&self, arg: &DocumentArg) -> Result<Document, Error> {
        let DocumentArg::Path(path) = arg;
        let entry = self
            .index
            .get(path)
            .ok_or_else(|| Error::NotFound { path: path.clone() })?;
        let text = fs::read_to_string(&entry.file).map_err(Error::io_at(&entry.file))?;
        let bad = |message| Error::Frontmatter {
            file: entry.file.clone(),
            message,
        };
        let fields = match frontmatter::block(&text).map_err(bad)? {
            Some(block) => frontmatter::fields(block).map_err(bad)?,
            None => Vec::new(),
        };
        let collection = &self.collections[entry.collection];
        Ok(Document {
            path: path.clone(),
            namespace: NAMESPACE.to_owned(),
            collection: collection.name.clone(),
            schema: collection.schema.clone(),
            fields,
        })
    }
}
