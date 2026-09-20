use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::argument::DocumentArg;
use crate::body::{self, Heading};
use crate::config::{CONFIG_FILE, Collection, Config, Level, Report, Rules, config_file};
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
    /// `schema.valid`, gathered once at load: it is always on and never per-document, so its
    /// level is always `error` and there is nothing left to compute once `validate` runs.
    schema_findings: Vec<Finding>,
    /// `filename.pattern`'s candidates, gathered once at load: the path and namespace of every
    /// file directly in a coded collection's folder that fits no collection there. It is
    /// configurable, so its level is decided when `validate` runs, not here.
    stray_files: Vec<(String, String)>,
}

struct Loaded {
    name: String,
    schema: Resolved,
    /// The schema file the collection names directly, relative to the project folder: `schema`
    /// resolved, not a parent reached through `extends`. Kept so `schema.valid`'s duplicate
    /// name and code check can name the file a conflict is with.
    schema_path: String,
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
        let mut schema_state = schema::Checked::default();
        let mut schema_findings = Vec::new();
        for collection in &config.collections {
            let template = read_template(collection, &mut report);
            let schema_load = match schema::load(root, collection, &mut report) {
                Ok(Some(schema_load)) => schema_load,
                Ok(None) => continue,
                Err(unreadable) => {
                    // A fault with no id yet does not hide the config errors that have one.
                    report.finish()?;
                    return Err(unreadable);
                }
            };
            for found in schema::check(&schema_load, &mut schema_state) {
                schema_findings.push(validate::schema_finding(
                    &found.path,
                    found.field.as_deref(),
                    found.message,
                ));
            }
            let schema_path = schema_load.chain[0].path.clone();
            let schema = schema::merge(&schema_load.chain);
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
                }),
                Err(message) => {
                    report.add("config.match-template", &collection.path, message);
                    continue;
                }
            }
            loaded.push(Loaded {
                name: collection.name.clone(),
                schema,
                schema_path,
                validation: collection.validation.clone(),
            });
        }
        report.finish()?;
        schema_findings.extend(duplicate_schema_findings(&loaded));
        for alias in &config.import_names {
            if schema::is_scheme_name(alias) {
                schema_findings.push(validate::schema_finding(
                    CONFIG_FILE,
                    None,
                    format!(
                        "the import name `{alias}` has the shape of a URL scheme, and the two would be told apart wrongly"
                    ),
                ));
            }
        }
        validate::order(&mut schema_findings);
        let index = Index::build(root, &config.namespaces, &members)?;
        let stray_files = crate::index::stray_files(root, &config.namespaces, &members)?;
        Ok(Project {
            root: root.to_owned(),
            config,
            index,
            collections: loaded,
            schema_findings,
            stray_files,
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
            let mut findings = self.schema_findings.clone();
            validate::order(&mut findings);
            return Ok(ValidateReport {
                scope: ValidateScope::Schemas,
                strict,
                namespaces: scope.namespaces,
                documents: 0,
                paths: None,
                findings,
            });
        }
        if args.is_empty() {
            let scope = self.scope(None, flag, env)?;
            let mut findings = self.schema_findings.clone();
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
            // An overlapping path has no one collection to check its frontmatter against, so it
            // was never checked (contract item 8's reasoning for a document whose block cannot
            // be parsed does not reach this far: that document at least had a schema to check
            // it with). `documents` stays a count of documents whose frontmatter was looked at;
            // the namespace is still added, since the scope did cover it.
            for (path, namespace_idx, collections) in self.index.overlaps() {
                let namespace = &self.config.namespaces[namespace_idx].name;
                if !scope.contains(namespace) {
                    continue;
                }
                namespaces.insert(namespace.clone());
                findings.push(validate::overlap_finding(
                    path,
                    namespace,
                    overlap_message(collections),
                ));
            }
            for (path, namespace) in &self.stray_files {
                if !scope.contains(namespace) {
                    continue;
                }
                if let Some(level) = validate::effective_level(
                    Level::Error,
                    "filename.pattern",
                    &self.config.validation,
                    &Rules::new(),
                    strict,
                ) {
                    findings.push(validate::stray_file_finding(
                        level,
                        path,
                        namespace,
                        "this file fits no collection's match template in this folder".to_owned(),
                    ));
                }
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
        // An overlapping path named directly is not refused here the way `get` and `toc` refuse
        // it (unlike them, `validate` exists to say what is wrong with a project, and design
        // line 567 names only two things that stop it before any report — an ambiguous key and
        // an argument that names no document — and an overlap is neither). It contributes a
        // `collections.overlap` finding, the same shape the whole-project scan gives it, and no
        // other argument's report is lost over it. It was never checked against a schema (there
        // is none to check it with), so, like the whole-project scan, it does not add to
        // `documents`; kept apart from `paths` for the same reason, so `paths` keeps meaning
        // "the path of each document checked" and stays in step with `documents.len()`. Tracked
        // by its own set so naming it twice reports it once.
        let mut overlapping = BTreeSet::new();
        for arg in args {
            let scope = self.scope(arg.namespace_prefix(), flag, env)?;
            if let DocumentArg::Path { path, .. } = arg
                && let Some((namespace_idx, collections)) = self.index.overlap(path)
            {
                if overlapping.insert(path.clone()) {
                    let namespace = &self.config.namespaces[namespace_idx].name;
                    namespaces.insert(namespace.clone());
                    findings.push(validate::overlap_finding(
                        path,
                        namespace,
                        overlap_message(collections),
                    ));
                }
                continue;
            }
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
    /// project's `validation.global`, and `strict` give them; plus `keys.unique` when its key is
    /// also used by another document of the same namespace. Always on, so `strict` does not
    /// change it. `collections.overlap` is never checked here: a path it is true of is not in
    /// the index at all (`Index::build`), so this is never reached for one; it is a finding of
    /// the whole-project scan instead.
    fn check_entry(&self, path: &str, entry: &Indexed, text: &str, strict: bool) -> Vec<Finding> {
        let collection = &self.collections[entry.collection];
        let namespace = &self.config.namespaces[entry.namespace].name;
        let name = DocName {
            path,
            namespace,
            collection: &collection.name,
            key: entry.key.as_deref(),
        };
        let mut findings = validate::check_document(
            text,
            &collection.schema,
            &self.config.validation,
            &collection.validation,
            strict,
            &name,
        );
        if let Some(key) = &entry.key
            && let Some(group) = self.index.key_group(entry.namespace, key)
            && group.len() > 1
        {
            let others: Vec<&str> = group
                .iter()
                .map(String::as_str)
                .filter(|other| *other != path)
                .collect();
            findings.push(validate::duplicate_key_finding(
                path,
                namespace,
                &collection.name,
                key,
                format!(
                    "the key `{key}` is also used in this namespace by `{}`",
                    others.join("`, `")
                ),
            ));
        }
        findings
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
                if let Some((_, collections)) = self.index.overlap(path) {
                    return Err(Error::Config {
                        file: self.root.join(path),
                        message: format!(
                            "{}, so there is no one schema to read it with: see collections.overlap in a validate report",
                            overlap_message(collections)
                        ),
                    });
                }
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

/// The message a `collections.overlap` finding and a refused direct read of the same path share:
/// which collections matched it.
fn overlap_message(collections: &[String]) -> String {
    format!(
        "this file is matched by more than one collection: `{}`",
        collections.join("`, `")
    )
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

/// `schema.valid`'s findings for a schema name or code shared by more than one collection's own
/// schema file: two different files, since `config.coded-schema-shared` already covers two
/// collections naming the exact same coded schema file. One finding per extra file, at the file
/// that comes later in collection order (collections are already sorted by name), naming the
/// first file that used it, the same convention `config.coded-schema-shared` uses.
fn duplicate_schema_findings(loaded: &[Loaded]) -> Vec<Finding> {
    let mut findings = Vec::new();
    let mut first_by_name: BTreeMap<&str, &str> = BTreeMap::new();
    let mut first_by_code: BTreeMap<&str, &str> = BTreeMap::new();
    for item in loaded {
        match first_by_name.get(item.schema.name.as_str()) {
            Some(&earlier) if earlier != item.schema_path => {
                findings.push(validate::schema_finding(
                    &item.schema_path,
                    None,
                    format!(
                        "the schema name `{}` is also used by `{earlier}`",
                        item.schema.name
                    ),
                ));
            }
            _ => {
                first_by_name.insert(&item.schema.name, &item.schema_path);
            }
        }
        if let Some(code) = &item.schema.code {
            match first_by_code.get(code.as_str()) {
                Some(&earlier) if earlier != item.schema_path => {
                    findings.push(validate::schema_finding(
                        &item.schema_path,
                        None,
                        format!("the schema code `{code}` is also used by `{earlier}`"),
                    ));
                }
                _ => {
                    first_by_code.insert(code, &item.schema_path);
                }
            }
        }
    }
    findings
}
