use std::collections::BTreeMap;
use std::collections::btree_map::Entry;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use crate::config::{Collection, Config, Report, config_file};
use crate::document::Document;
use crate::env::Env;
use crate::error::Error;
use crate::frontmatter;
use crate::index::{Index, Member};
use crate::schema::{self, Resolved};
use crate::scope::{self, Scope};
use crate::template::Template;

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
    root: PathBuf,
    config: Config,
    index: Index,
    collections: Vec<Loaded>,
}

struct Loaded {
    name: String,
    schema: Resolved,
}

impl Project {
    pub fn load(root: &Path) -> Result<Project, Error> {
        let mut report = Report::default();
        let config = Config::load(root, &mut report)?;
        let mut loaded = Vec::new();
        let mut members = Vec::new();
        let mut coded: BTreeMap<String, &str> = BTreeMap::new();
        for collection in &config.collections {
            let template = read_template(collection, &mut report);
            let schema = match schema::load(root, collection, &mut report) {
                Ok(Some(schema)) => schema,
                Ok(None) => continue,
                Err(unreadable) => {
                    // A fault with no id yet does not hide the config errors that have one.
                    report.finish()?;
                    return Err(unreadable);
                }
            };
            if schema.code.is_some() {
                match coded.entry(schema::identity(collection)) {
                    Entry::Vacant(free) => {
                        free.insert(&collection.name);
                    }
                    Entry::Occupied(first) => report.add(
                        "config.coded-schema-shared",
                        &collection.path,
                        format!(
                            "the schema `{}` has a code and the collection `{}` uses it already: one coded schema serves one collection",
                            collection.schema,
                            first.get()
                        ),
                    ),
                }
            }
            let Some(template) = template else { continue };
            match template.bind(&collection.pattern, schema.code.as_deref()) {
                Ok(template) => members.push(Member {
                    name: collection.name.clone(),
                    template,
                    file: collection.file.clone(),
                }),
                Err(message) => {
                    report.add("config.match-template", &collection.path, message);
                    continue;
                }
            }
            loaded.push(Loaded {
                name: collection.name.clone(),
                schema,
            });
        }
        report.finish()?;
        let index = Index::build(root, &config.namespaces, &members)?;
        Ok(Project {
            root: root.to_owned(),
            config,
            index,
            collections: loaded,
        })
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    /// The namespaces a command reads. `prefix` is the namespace an argument names.
    pub fn scope(
        &self,
        prefix: Option<&str>,
        flag: Option<&str>,
        env: &dyn Env,
    ) -> Result<Scope, Error> {
        scope::choose(&self.config.namespaces, &self.root, prefix, flag, env)
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
        let collection = &self.collections[entry.collection];
        let fields = match frontmatter::block(&text).map_err(bad)? {
            Some(block) => frontmatter::fields(block, &collection.schema).map_err(bad)?,
            None => Vec::new(),
        };
        Ok(Document {
            path: path.clone(),
            namespace: self.config.namespaces[entry.namespace].name.clone(),
            collection: collection.name.clone(),
            schema: collection.schema.name.clone(),
            fields,
        })
    }
}

/// The template of a collection as it is written, before it is bound to a schema. A template
/// that cannot be read is a config error.
fn read_template(collection: &Collection, report: &mut Report) -> Option<Template> {
    match Template::parse(&collection.pattern) {
        Ok(template) => Some(template),
        Err(message) => {
            report.add("config.match-template", &collection.path, message);
            None
        }
    }
}
