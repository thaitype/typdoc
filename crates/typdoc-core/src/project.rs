use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::argument::DocumentArg;
use crate::body::{self, Heading};
use crate::config::{Collection, Config, Report, Rules, config_file};
use crate::document::Document;
use crate::env::Env;
use crate::error::Error;
use crate::frontmatter;
use crate::index::{Entry as Indexed, Index, Member};
use crate::schema::{self, Resolved};
use crate::scope::{self, Scope};
use crate::template::Template;
use crate::validate::{self, DocName, Finding, ValidateScope};

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

/// The headings of one document, named as the design names a document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Toc {
    /// Relative to the project folder.
    pub path: String,
    pub namespace: String,
    /// Present only when the schema has a code.
    pub key: Option<String>,
    pub headings: Vec<Heading>,
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
    /// The collection file's own `validation`, merged over the project's `validation.global`.
    validation: Rules,
}

/// The report of a `validate` run, in the shape the design's summary and findings hold, before
/// the CLI turns it into JSON.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidateReport {
    pub scope: ValidateScope,
    pub strict: bool,
    /// Sorted, each once.
    pub namespaces: Vec<String>,
    /// 0 for `Schemas`.
    pub documents: usize,
    /// Only for `Paths`: sorted, each once, matching the `path` of every finding.
    pub paths: Option<Vec<String>>,
    /// In the order the design guarantees.
    pub findings: Vec<Finding>,
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
                validation: collection.validation.clone(),
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

    pub fn get(&self, arg: &DocumentArg, scope: &Scope, env: &dyn Env) -> Result<Document, Error> {
        let (path, entry, text) = self.resolve(arg, scope, env)?;
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
            path,
            namespace: self.config.namespaces[entry.namespace].name.clone(),
            key: entry.key.clone(),
            code: collection.schema.code.clone(),
            collection: collection.name.clone(),
            schema: collection.schema.name.clone(),
            fields,
        })
    }

    /// The headings of a document's body, in the order of `line`.
    pub fn toc(&self, arg: &DocumentArg, scope: &Scope, env: &dyn Env) -> Result<Toc, Error> {
        let (path, entry, text) = self.resolve(arg, scope, env)?;
        let headings = body::headings(&text).map_err(|message| Error::Frontmatter {
            file: entry.file.clone(),
            message,
        })?;
        Ok(Toc {
            path,
            namespace: self.config.namespaces[entry.namespace].name.clone(),
            key: entry.key.clone(),
            headings,
        })
    }

    /// The report of `validate`: `Schemas` when `schemas_only`, `Paths` when `args` is not
    /// empty, and `All` (every document in scope) otherwise. Combining `schemas_only` with
    /// arguments is a caller error and is refused before this runs (design: "combining either
    /// with arguments is bad arguments"), so it is not checked again here.
    pub fn validate(
        &self,
        args: &[DocumentArg],
        schemas_only: bool,
        strict: bool,
        flag: Option<&str>,
        env: &dyn Env,
    ) -> Result<ValidateReport, Error> {
        if schemas_only {
            let scope = self.scope(None, flag, env)?;
            return Ok(ValidateReport {
                scope: ValidateScope::Schemas,
                strict,
                namespaces: scope.namespaces,
                documents: 0,
                paths: None,
                findings: Vec::new(),
            });
        }
        if args.is_empty() {
            let scope = self.scope(None, flag, env)?;
            let mut findings = Vec::new();
            let mut namespaces = BTreeSet::new();
            let mut documents = 0usize;
            for (path, entry) in self.index.iter() {
                let namespace = &self.config.namespaces[entry.namespace].name;
                if !scope.contains(namespace) {
                    continue;
                }
                namespaces.insert(namespace.clone());
                documents += 1;
                let text = fs::read_to_string(&entry.file).map_err(Error::io_at(&entry.file))?;
                findings.extend(self.check_entry(path, entry, &text, strict));
            }
            validate::order(&mut findings);
            return Ok(ValidateReport {
                scope: ValidateScope::All,
                strict,
                namespaces: namespaces.into_iter().collect(),
                documents,
                paths: None,
                findings,
            });
        }
        let mut findings = Vec::new();
        let mut namespaces = BTreeSet::new();
        let mut paths = BTreeSet::new();
        for arg in args {
            let scope = self.scope(arg.namespace_prefix(), flag, env)?;
            let (path, entry, text) = self.resolve(arg, &scope, env)?;
            if paths.insert(path.clone()) {
                let namespace = &self.config.namespaces[entry.namespace].name;
                namespaces.insert(namespace.clone());
                findings.extend(self.check_entry(&path, entry, &text, strict));
            }
        }
        validate::order(&mut findings);
        Ok(ValidateReport {
            scope: ValidateScope::Paths,
            strict,
            namespaces: namespaces.into_iter().collect(),
            documents: paths.len(),
            paths: Some(paths.into_iter().collect()),
            findings,
        })
    }

    /// The findings of one document already read: `frontmatter.parse`, `frontmatter.types` and
    /// `frontmatter.unknown`, at the level the collection's own `validation`, merged over the
    /// project's `validation.global`, and `strict` give them.
    fn check_entry(&self, path: &str, entry: &Indexed, text: &str, strict: bool) -> Vec<Finding> {
        let collection = &self.collections[entry.collection];
        let name = DocName {
            path,
            namespace: &self.config.namespaces[entry.namespace].name,
            collection: &collection.name,
            key: entry.key.as_deref(),
        };
        validate::check_document(
            text,
            &collection.schema,
            &self.config.validation,
            &collection.validation,
            strict,
            &name,
        )
    }

    /// The path a document argument names, its place in the index, and the text of the file.
    /// A path is looked up as it stands, relative to the project folder, whether or not it
    /// carries a namespace prefix: a prefix only chooses scope (validated by the caller through
    /// `Project::scope` before this runs) and does not change what the path is read against,
    /// since the path already names the file (ticket 4). A key is resolved against the
    /// namespaces in `scope`, since the same key can be issued once in each of several.
    fn resolve(
        &self,
        arg: &DocumentArg,
        scope: &Scope,
        env: &dyn Env,
    ) -> Result<(String, &Indexed, String), Error> {
        let path = match arg {
            DocumentArg::Path { path, .. } => {
                if self.index.get(path).is_none() {
                    return Err(self.not_found(path, env));
                }
                path.clone()
            }
            DocumentArg::Key { namespace, key } => {
                self.resolve_key(namespace.as_deref(), key, scope)?
            }
        };
        let entry = self
            .index
            .get(&path)
            .expect("the path was just looked up above");
        let text = fs::read_to_string(&entry.file).map_err(Error::io_at(&entry.file))?;
        Ok((path, entry, text))
    }

    /// The path of the document `key` names, in the namespace `prefix` names or, absent that,
    /// in every namespace of `scope`. More than one match is `Error::AmbiguousKey`; none is
    /// `Error::NotFound`, with no `./name` hint, since a key names no place on disk.
    fn resolve_key(&self, prefix: Option<&str>, key: &str, scope: &Scope) -> Result<String, Error> {
        let found: Vec<(String, String)> = scope
            .namespaces
            .iter()
            .filter_map(|name| self.namespace_index(name).map(|index| (name, index)))
            .filter_map(|(name, index)| {
                self.index
                    .key(index, key)
                    .map(|path| (name.clone(), path.to_owned()))
            })
            .collect();
        match found.len() {
            0 => Err(Error::NotFound {
                path: printed_key(prefix, key),
                hint: false,
            }),
            1 => Ok(found.into_iter().next().expect("checked above").1),
            _ => Err(Error::AmbiguousKey {
                key: key.to_owned(),
                candidates: found
                    .into_iter()
                    .map(|(namespace, _)| format!("{namespace}:{key}"))
                    .collect(),
            }),
        }
    }

    fn namespace_index(&self, name: &str) -> Option<usize> {
        self.config.namespaces.iter().position(|n| n.name == name)
    }

    /// A path relative to the project that names nothing in the index; the message names
    /// `./path`, relative to the current directory, when a file is there, as a suggestion and
    /// not a substitution.
    fn not_found(&self, path: &str, env: &dyn Env) -> Error {
        let hint = env.current_dir().is_ok_and(|cwd| cwd.join(path).is_file());
        Error::NotFound {
            path: path.to_owned(),
            hint,
        }
    }
}

/// The text an ambiguous or missing key is reported by: the prefix the argument gave, if any,
/// put back in front of it.
fn printed_key(prefix: Option<&str>, key: &str) -> String {
    match prefix {
        Some(namespace) => format!("{namespace}:{key}"),
        None => key.to_owned(),
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
