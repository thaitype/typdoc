use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::argument::DocumentArg;
use crate::body::{self, Heading};
use crate::config::{CONFIG_FILE, Collection, Config, Level, RefBase, Report, Rules, config_file};
use crate::document::{Document, Value};
use crate::env::Env;
use crate::error::Error;
use crate::frontmatter;
use crate::index::{Entry as Indexed, Index, Member};
use crate::refs;
use crate::schema::{self, Auto, FieldType, Resolved};
use crate::scope::{self, Scope};
use crate::template::Template;
use crate::validate::{self, DocName, Finding, Severity, ValidateScope};

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

/// The whole-project context `refs.resolve`, `refs.target`, `refs.codedByPath` and `refs.moved`
/// read beside a document's own frontmatter, bundled because the three always travel together
/// from `Project::validate` through `check_entry` to `check_refs`.
struct RefProject<'a> {
    /// The code of every coded schema in the project (the bare-key ref form's "the code exists
    /// in this project" condition).
    codes: BTreeSet<String>,
    /// The name and code of every collection's schema, by the collection's position.
    schemas: Vec<refs::SchemaInfo<'a>>,
    /// A written ref that no longer resolves, to the current key or path of the document that
    /// recorded moving away from it (`auto: moves`).
    moved: BTreeMap<String, String>,
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
    /// How a relative ref in a document of this collection is resolved: from the document's own
    /// folder or from its namespace's. The sibling-prefixed path form ignores this; only the
    /// unprefixed form reads it (design.md's Refs table).
    ref_base: RefBase,
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
                ref_base: collection.ref_base,
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
            findings.extend(self.shadowed_names_findings(strict));
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
            let (ref_project, acyclic) = self.ref_project()?;
            let mut findings = self.schema_findings.clone();
            findings.extend(self.shadowed_names_findings(strict));
            findings.extend(acyclic);
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
                findings.extend(self.check_entry(path, entry, &text, strict, &ref_project));
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
        let (ref_project, acyclic) = self.ref_project()?;
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
                findings.extend(self.check_entry(&path, entry, &text, strict, &ref_project));
            }
        }
        // `refs.acyclic` is always on and its cycles are project-wide, but only a cycle that
        // passes through one of the documents actually named is reported here: unlike the
        // whole-project scan, every finding in this scope is about a document the caller named,
        // and a cycle elsewhere in the project is that other document's own `validate` run to
        // report, not this one's.
        findings.extend(
            acyclic
                .into_iter()
                .filter(|finding| paths.contains(&finding.path)),
        );
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
    /// the whole-project scan instead. `refs` gives `refs.resolve`, `refs.target`,
    /// `refs.codedByPath` and `refs.moved`, skipped when `frontmatter.parse` already fired for
    /// this document (the design: "no other rule is evaluated for that file").
    fn check_entry(
        &self,
        path: &str,
        entry: &Indexed,
        text: &str,
        strict: bool,
        ref_project: &RefProject,
    ) -> Vec<Finding> {
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
        if !findings.iter().any(|f| f.rule == "frontmatter.parse") {
            findings.extend(self.check_refs(path, entry, text, &name, strict, ref_project));
        }
        findings
    }

    /// `refs.resolve`, `refs.target`, `refs.codedByPath` and `refs.moved` for every `ref` and
    /// `ref[]` field of one document already known to parse. A value that does not fit its
    /// field's type is skipped: `frontmatter.types` already reported it, and a value that is not
    /// text or a list of text names nothing a ref form could read.
    fn check_refs(
        &self,
        path: &str,
        entry: &Indexed,
        text: &str,
        name: &DocName,
        strict: bool,
        ref_project: &RefProject,
    ) -> Vec<Finding> {
        let collection = &self.collections[entry.collection];
        let fields = match parsed_fields(text, &collection.schema) {
            Some(fields) => fields,
            None => return Vec::new(),
        };
        let ctx = refs::Ctx {
            doc_namespace: entry.namespace,
            doc_path: path,
            ref_base: collection.ref_base,
            namespaces: &self.config.namespaces,
            codes: &ref_project.codes,
            index: &self.index,
            root: &self.root,
        };
        let mut findings = Vec::new();
        for (field_name, value) in &fields {
            let Some(field) = collection.schema.field(field_name) else {
                continue;
            };
            if !matches!(field.kind, FieldType::Ref | FieldType::RefList)
                || !crate::coerce::fits(&field.kind, value)
            {
                continue;
            }
            for written in ref_values(value) {
                match refs::resolve_one(written, &ctx) {
                    Err(reason) => findings.extend(self.unresolved_ref_finding(
                        name,
                        field_name,
                        written,
                        reason,
                        &ref_project.moved,
                        &collection.validation,
                        strict,
                    )),
                    Ok(resolved) => {
                        if !refs::target_allowed(
                            field.target.as_ref(),
                            &resolved,
                            &ref_project.schemas,
                        ) {
                            findings.push(validate::finding(
                                name,
                                Severity::Error,
                                "refs.target",
                                Some(field_name),
                                format!(
                                    "the ref `{written}` targets `{}`, which `target` does not allow",
                                    resolved.path
                                ),
                            ));
                        }
                        let coded = resolved
                            .collection
                            .is_some_and(|index| ref_project.schemas[index].code.is_some());
                        if coded
                            && resolved.via == refs::Via::Path
                            && let Some(level) = validate::effective_level(
                                Level::Warn,
                                "refs.codedByPath",
                                &self.config.validation,
                                &collection.validation,
                                strict,
                            )
                        {
                            findings.push(validate::finding(
                                name,
                                level,
                                "refs.codedByPath",
                                Some(field_name),
                                format!(
                                    "the ref `{written}` names a coded document by path: use its key instead"
                                ),
                            ));
                        }
                    }
                }
            }
        }
        findings
    }

    /// `refs.moved` when `written` matches a recorded move, `refs.resolve` otherwise (`None` only
    /// when `refs.moved` fires but is configured `off`): the one place that decides between the
    /// two ordinary-missing-target findings, so `check_refs` reads as one branch per outcome of
    /// `resolve_one` rather than a decision nested inside it.
    #[allow(
        clippy::too_many_arguments,
        reason = "each part is independent context a caller already holds (the document's name,
    which field and ref, why it failed, the project's moved records, the collection's own rule
    levels, strict); bundling them would hide which one changes across the two call sites that
    would use it, `check_refs` and a future `body.mentions` (ticket 11, also `refs.moved`)"
    )]
    fn unresolved_ref_finding(
        &self,
        name: &DocName,
        field_name: &str,
        written: &str,
        reason: refs::Reason,
        moved: &BTreeMap<String, String>,
        collection: &Rules,
        strict: bool,
    ) -> Option<Finding> {
        if let Some(new_id) = moved.get(written) {
            let level = validate::effective_level(
                Level::Error,
                "refs.moved",
                &self.config.validation,
                collection,
                strict,
            )?;
            return Some(validate::finding(
                name,
                level,
                "refs.moved",
                Some(field_name),
                format!("the ref `{written}` no longer resolves: it was moved to `{new_id}`"),
            ));
        }
        Some(validate::finding(
            name,
            Severity::Error,
            "refs.resolve",
            Some(field_name),
            reason_message(written, reason),
        ))
    }

    /// The code of every coded schema in the project, for the bare-key ref form's "the code
    /// exists in this project" condition (design.md's Refs table).
    fn project_codes(&self) -> BTreeSet<String> {
        self.collections
            .iter()
            .filter_map(|collection| collection.schema.code.clone())
            .collect()
    }

    /// The name and code of every collection's schema, by the collection's position, for
    /// `refs.target` and `refs.codedByPath`.
    fn schema_infos(&self) -> Vec<refs::SchemaInfo<'_>> {
        self.collections
            .iter()
            .map(|collection| refs::SchemaInfo {
                name: &collection.schema.name,
                code: collection.schema.code.as_deref(),
            })
            .collect()
    }

    /// The whole-project context `check_entry` and `check_refs` read beside a document's own
    /// frontmatter (`codes`, `schemas`, and the `refs.moved` map `prescan_refs` builds), bundled
    /// into one reference since the three always travel together from here down to `check_refs`;
    /// `refs.acyclic`'s findings are returned alongside rather than folded in, since whether they
    /// are reported depends on the scope (see `validate`'s two callers of this).
    fn ref_project(&self) -> Result<(RefProject<'_>, Vec<Finding>), Error> {
        let codes = self.project_codes();
        let schemas = self.schema_infos();
        let (moved, acyclic) = self.prescan_refs(&codes)?;
        Ok((
            RefProject {
                codes,
                schemas,
                moved,
            },
            acyclic,
        ))
    }

    /// `names.shadowed`: a namespace name that is also an import alias, so `name:` and `name::`
    /// reach different documents. A fact about the config, not about a document, so it needs no
    /// document read and is the same in every scope that reports it (`All` and `Schemas`, the
    /// same two `schema_findings` reaches, never `Paths`: see `check_entry`'s callers).
    fn shadowed_names_findings(&self, strict: bool) -> Vec<Finding> {
        let Some(level) = validate::effective_level(
            Level::Warn,
            "names.shadowed",
            &self.config.validation,
            &Rules::new(),
            strict,
        ) else {
            return Vec::new();
        };
        self.config
            .namespaces
            .iter()
            .map(|namespace| namespace.name.as_str())
            .filter(|name| self.config.import_names.iter().any(|alias| alias == name))
            .map(|name| {
                validate::names_shadowed_finding(
                    level,
                    CONFIG_FILE,
                    format!(
                        "the name `{name}` is both a sibling namespace and an import alias: `{name}:` and `{name}::` reach different documents"
                    ),
                )
            })
            .collect()
    }

    /// The whole-project pass `refs.moved` and `refs.acyclic` both need before any single
    /// document's refs can be judged: a moved record can be recorded in a document outside the
    /// scope of the run that reads it, and a cycle can pass through documents outside it too.
    /// Reads every document once, distinct from the read `check_entry`'s caller already does for
    /// the ones in scope: cheap next to a network call, and this crate makes none (contract item
    /// 21, "no promised figure for the cost of a run").
    ///
    /// Returns the map from a written ref that no longer resolves to the current key or path of
    /// the document that recorded moving away from it (`auto: moves`), and the `refs.acyclic`
    /// findings of every cycle found through a field marked `acyclic` (one finding per document
    /// on a cycle, per field, since each one's own edge is what is wrong with it).
    fn prescan_refs(
        &self,
        codes: &BTreeSet<String>,
    ) -> Result<(BTreeMap<String, String>, Vec<Finding>), Error> {
        let mut moved: BTreeMap<String, String> = BTreeMap::new();
        let mut edges: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
        for (path, entry) in self.index.iter() {
            let collection = &self.collections[entry.collection];
            let text = fs::read_to_string(&entry.file).map_err(Error::io_at(&entry.file))?;
            let Some(fields) = parsed_fields(&text, &collection.schema) else {
                continue;
            };
            let ctx = refs::Ctx {
                doc_namespace: entry.namespace,
                doc_path: path,
                ref_base: collection.ref_base,
                namespaces: &self.config.namespaces,
                codes,
                index: &self.index,
                root: &self.root,
            };
            let identity = entry.key.clone().unwrap_or_else(|| path.to_owned());
            for (field_name, value) in &fields {
                let Some(field) = collection.schema.field(field_name) else {
                    continue;
                };
                if field.auto == Some(Auto::Moves)
                    && field.kind == FieldType::List
                    && let Value::List(items) = value
                {
                    for item in items {
                        moved
                            .entry(item.clone())
                            .or_insert_with(|| identity.clone());
                    }
                }
                if field.is_acyclic()
                    && matches!(field.kind, FieldType::Ref | FieldType::RefList)
                    && crate::coerce::fits(&field.kind, value)
                {
                    for written in ref_values(value) {
                        if let Ok(resolved) = refs::resolve_one(written, &ctx) {
                            edges
                                .entry(field_name.clone())
                                .or_default()
                                .push((path.to_owned(), resolved.path));
                        }
                    }
                }
            }
        }
        let mut findings = Vec::new();
        for (field_name, field_edges) in &edges {
            for cyclic_path in refs::cyclic_nodes(field_edges) {
                let Some(entry) = self.index.get(&cyclic_path) else {
                    continue;
                };
                let collection = &self.collections[entry.collection];
                let namespace = &self.config.namespaces[entry.namespace].name;
                let name = DocName {
                    path: &cyclic_path,
                    namespace,
                    collection: &collection.name,
                    key: entry.key.as_deref(),
                };
                findings.push(validate::finding(
                    &name,
                    Severity::Error,
                    "refs.acyclic",
                    Some(field_name),
                    format!("a cycle passes through `{field_name}`"),
                ));
            }
        }
        // Not sorted here: the caller merges this into a larger set of findings and orders that
        // once, so sorting this slice first would only be thrown away.
        Ok((moved, findings))
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

/// The fields of a document already read, or `None` when its block cannot be parsed: the ref
/// rules read no document `frontmatter.parse` has already refused (design: "no other rule is
/// evaluated for that file"), and this is the one place that reads a block a second time to get
/// them, since `validate::check_document` does not hand its own parse back out.
fn parsed_fields(text: &str, schema: &Resolved) -> Option<Vec<(String, Value)>> {
    let block = frontmatter::block(text).ok()??;
    frontmatter::fields(block, schema).ok()
}

/// The written ref or refs a field's already-typed value holds: one for `ref`, each item of the
/// list for `ref[]`. Called only once `coerce::fits` has shown the value matches the field's
/// kind, so the other variants of `Value` never reach here.
fn ref_values(value: &Value) -> Vec<&str> {
    match value {
        Value::Text(text) => vec![text.as_str()],
        Value::List(items) => items.iter().map(String::as_str).collect(),
        Value::Number(_) | Value::Bool(_) | Value::Date(_) | Value::Datetime(_) => Vec::new(),
    }
}

/// The message for a ref that does not resolve, naming why (`refs.resolve`'s two reasons; the
/// third, `import-absent`, is ticket 17's, since this story does not resolve an import).
fn reason_message(written: &str, reason: refs::Reason) -> String {
    match reason {
        refs::Reason::NotFound => format!("the ref `{written}` does not resolve: not found"),
        refs::Reason::BadPrefix => {
            format!("the ref `{written}` does not resolve: its prefix names no namespace")
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
