use std::cmp::Ordering;
use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::{DateTime, FixedOffset, NaiveDate};

use crate::argument::DocumentArg;
use crate::body::{self, Heading};
use crate::config::{
    CONFIG_FILE, Collection, Config, LEFTOVER_TEMP_FILE, Level, LockMode, Namespace, RefBase,
    Report, Rules, TYPDOC_DIR, config_file,
};
use crate::document::{Document, Value};
use crate::env::{Deps, Env};
use crate::error::Error;
use crate::frontmatter::{self, FrontmatterWriter, YamlSerdeWriter};
use crate::fs::{Fs, create_exclusively, write_atomically};
use crate::index::{Entry as Indexed, Index, Member};
use crate::lines::Position;
use crate::links::{self, BodyLink, BodyLinks};
use crate::mv::{self, ContentChange, MvReport, RewrittenRef, UnrewrittenReason, UnrewrittenRef};
use crate::namespace_lock::{
    self, NamespaceLock, acquire, local_namespace_lock_path, order_locks, release,
};
use crate::query::{self, Condition, Dir, FieldRef, PlainCondition, Quant, RefCondition, RefField};
use crate::refs;
use crate::schema::{self, Auto, Field, FieldType, Resolved};
use crate::scope::{self, Scope, Source};
use crate::state;
use crate::template::{Step, Template};
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

/// The headings of one document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Toc {
    /// Relative to the project the document belongs to.
    pub path: String,
    pub namespace: String,
    /// Present only when the schema has a code.
    pub key: Option<String>,
    /// The alias this document was reached through, when it belongs to an imported project.
    pub project: Option<String>,
    pub headings: Vec<Heading>,
}

/// The name of a document at the other end of a reference, once it is known to exist. `key` is
/// present only for a coded document, `project` is the alias of the imported project the document
/// belongs to (`None` for this project), and `namespace` is `None` for a file outside every
/// namespace folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefName {
    pub path: String,
    pub namespace: Option<String>,
    pub key: Option<String>,
    pub project: Option<String>,
}

impl RefName {
    /// The path alone is not enough: an imported project can hold a document at a path this one
    /// also has.
    fn is_same_document(&self, other: &RefName) -> bool {
        self.project == other.project
            && self.namespace == other.namespace
            && self.path == other.path
    }
}

/// What one written ref resolves to, or the id of why it does not (`not-found`, `bad-prefix` or
/// `import-absent`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefOutcome {
    Resolved(RefName),
    Unresolved(&'static str),
}

/// `refs`' two directions: the refs a document holds, or the refs that hold it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefsDirection {
    Out,
    In,
}

/// One entry of a `refs` report. `other` is the target for `Out`, and for `In` the document that
/// holds the ref, which is always resolved. `field` is `"$body"` for a body link, the only kind
/// with a `position`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefsReference {
    pub other: RefOutcome,
    pub field: String,
    pub written: String,
    pub position: Option<Position>,
}

/// The refs `mv` will rewrite, by the path of the document that holds them.
type RewriteByHolder = BTreeMap<String, Vec<RefsReference>>;

/// The report of a `refs` run. `refs` is in its guaranteed order: the fields in document order,
/// each value as written, then `$body` by position; for `In`, grouped by the holder's path first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefsReport {
    pub document: RefName,
    pub direction: RefsDirection,
    pub refs: Vec<RefsReference>,
}

/// `collection` is the holder's index into `Project::collections`, so `holder` is read under its
/// own schema without reading its file again.
struct IncomingRef {
    holder: Document,
    collection: usize,
    reference: RefsReference,
}

struct RefEvalCtx<'a> {
    me: &'a RefName,
    me_entry: &'a Indexed,
    me_fields: &'a [(String, Value)],
    me_body: &'a BodyLinks,
    codes: &'a BTreeSet<String>,
    incoming: Option<&'a [IncomingRef]>,
}

/// One `--sort field[:asc|:desc]` of a `list` run, already parsed.
#[derive(Debug, Clone)]
pub struct SortKey {
    pub field: FieldRef,
    pub desc: bool,
}

/// `list`'s filter, already parsed by the caller. Empty `collections` and `codes` mean every
/// collection, `wheres` are ANDed, and `sort` is in priority order. `--limit`, `--fields` and
/// `--ids` change neither which documents match nor their order, so the caller applies them.
#[derive(Debug)]
pub struct ListFilter<'a> {
    pub collections: &'a [String],
    pub codes: &'a [String],
    pub wheres: &'a [Condition],
    pub sort: &'a [SortKey],
}

/// `list`'s result: the matched documents, sorted but not cut to `--limit`, and one line for
/// every dangling ref a `ref.*` condition reached, ready to print to stderr.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListResult {
    pub documents: Vec<Document>,
    pub dangling_refs: Vec<String>,
}

pub struct Project {
    root: PathBuf,
    config: Config,
    index: Index,
    collections: Vec<Loaded>,
    /// `schema.valid` is always on and not per document, so its findings are final at load.
    schema_findings: Vec<Finding>,
    /// `filename.pattern`'s candidates, as (path, namespace): every file directly in a coded
    /// collection's folder that fits no collection there. The rule is configurable, so its level
    /// is decided when `validate` runs.
    stray_files: Vec<(String, String)>,
    /// `--audit`'s own walk needs these to know which folders whose names begin with `.` a
    /// `match` names as plain text, and so which of them a run reads at all.
    members: Vec<Member>,
    /// Every alias of `config.imports`, merged with the machine file's. An imported project's own
    /// imports are not followed. An alias missing here names no import, so an import prefix
    /// using it is `bad-prefix`.
    imports: BTreeMap<String, ImportState>,
    /// Every namespace's state file as read at load, never written back: a write command reads
    /// the file again under its lock and writes it through `state::write`.
    state: BTreeMap<String, state::StateFile>,
}

pub(crate) enum ImportState {
    Absent(crate::imports::Absence),
    Loaded(Box<Project>),
}

struct RefProject {
    /// A bare-key ref needs its code to exist in this project.
    codes: BTreeSet<String>,
    /// A written ref that no longer resolves, to the current key or path of the document that
    /// recorded moving away from it (`auto: moves`).
    moved: BTreeMap<String, String>,
}

struct PrescanAccum {
    moved: BTreeMap<String, String>,
    edges: BTreeMap<String, Vec<(String, String)>>,
}

struct BodyDocContext<'a> {
    name: &'a DocName<'a>,
    doc_path: &'a str,
    doc_text: &'a str,
    ctx: &'a refs::Ctx<'a>,
    ignore: &'a [Template],
    moved: &'a BTreeMap<String, String>,
    collection: &'a Rules,
    strict: bool,
    audit: bool,
    links_level: Option<Severity>,
    anchors_level: Option<Severity>,
}

struct Loaded {
    name: String,
    schema: Resolved,
    /// The schema file the collection names directly, not a parent reached through `extends`,
    /// so `schema.valid` can name the file a duplicate name or code conflicts with.
    schema_path: String,
    /// The collection file's own `validation`, merged over the project's `validation.global`.
    validation: Rules,
    /// Read only for an unprefixed relative ref: a sibling-prefixed path is always read from
    /// that namespace's folder.
    ref_base: RefBase,
}

/// The report of a `validate` run, before the CLI turns it into JSON.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidateReport {
    pub scope: ValidateScope,
    pub strict: bool,
    /// Sorted, each once.
    pub namespaces: Vec<String>,
    /// 0 for `Schemas`. Never counts a file matched by more than one collection, which has no one
    /// schema to be checked against, nor, under `--audit`, a file with no frontmatter.
    pub documents: usize,
    /// Only for `Paths`: sorted, each once, matching the `path` of every finding.
    pub paths: Option<Vec<String>>,
    /// Ordered by `validate::order`.
    pub findings: Vec<Finding>,
    /// Only for `--audit`.
    pub audit: Option<AuditReport>,
}

/// The number of documents one collection holds, for `audit.collections`. It includes a document
/// with no frontmatter, which the collection matched and nothing evaluated. A file matched by
/// more than one collection is held by none of them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditCollection {
    pub name: String,
    pub documents: usize,
}

/// One file matched by more than one collection and the names of the collections that match it,
/// for `audit.overlapping`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditOverlap {
    pub path: String,
    /// Sorted by name.
    pub collections: Vec<String>,
}

/// One directory entry a `match` or a `namespaces` glob reached and did not read (a symbolic
/// link, a name that is not valid UTF-8, or a leftover temp file), for `audit.not_read`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditNotRead {
    pub path: String,
    pub reason: String,
}

/// `--audit`'s report, beside `findings`. Its three file lists never overlap: a file in no
/// collection has no schema to say what its frontmatter should hold, so it is only in
/// `uncollected`, and a file matched by more than one collection is reported as
/// `collections.overlap` and is only in `overlapping`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AuditReport {
    /// Sorted by name, a collection that holds no document in scope included with 0.
    pub collections: Vec<AuditCollection>,
    /// Sorted, each once.
    pub uncollected: Vec<String>,
    /// Files in a collection with no frontmatter block at all, sorted, each once. An empty block
    /// is frontmatter, and a block that fails to parse is a `frontmatter.parse` finding instead.
    pub no_frontmatter: Vec<String>,
    /// Sorted by `path`, each once. Counted here and in `summary.overlapping` so that the
    /// summary's account of the run is complete.
    pub overlapping: Vec<AuditOverlap>,
    /// Sorted by `path`, each once. Counted beside `unreported`, not inside it: each entry is
    /// already reported in `findings`, under `files.unreadable` or `files.leftover`.
    pub not_read: Vec<AuditNotRead>,
}

impl Project {
    /// Loads the project at `root`, its schemas, its index, and every import it configures. An
    /// imported project's own `imports` are read for `schema.valid`'s name-collision check but
    /// are never loaded.
    pub fn load(root: &Path, env: &dyn Env) -> Result<Project, Error> {
        Project::load_inner(root, env, true)
    }

    fn load_inner(root: &Path, env: &dyn Env, follow_imports: bool) -> Result<Project, Error> {
        let mut report = Report::default();
        let config = Config::load(root, &mut report)?;
        let mut loaded = Vec::new();
        let mut members = Vec::new();
        let mut coded: BTreeMap<String, &str> = BTreeMap::new();
        let mut schema_state = schema::Checked::default();
        let mut schema_findings = Vec::new();
        let mut qualified_targets: Vec<schema::QualifiedTarget> = Vec::new();
        for collection in &config.collections {
            let template = read_template(collection, &mut report);
            let schema_load = match schema::load(root, collection, &mut report) {
                Ok(Some(schema_load)) => schema_load,
                Ok(None) => continue,
                Err(unreadable) => {
                    // A fault with no id does not hide the config errors that have one.
                    report.finish()?;
                    return Err(unreadable);
                }
            };
            let (problems, targets) = schema::check(&schema_load, &mut schema_state);
            for found in problems {
                schema_findings.push(validate::schema_finding(
                    &found.path,
                    found.field.as_deref(),
                    found.message,
                ));
            }
            qualified_targets.extend(targets);
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
        // Read while `report` is open, so `config.state-orphan` and `config.state-uncoded` are
        // reported with every other config error.
        let state_by_namespace = read_state(root, &config, &loaded, &mut report)?;
        // Found while `report` is open, so a bad `TYPDOC_CONFIG_DIR` (`config.config-dir`) is
        // reported with every other config error rather than as a later failure.
        let machine_file = follow_imports
            .then(|| crate::imports::imports_file_path(env, &mut report))
            .flatten();
        report.finish()?;
        schema_findings.extend(duplicate_schema_findings(&loaded));
        for alias in config.imports.keys() {
            if schema::reserved_url_scheme(alias) {
                schema_findings.push(reserved_alias_finding(CONFIG_FILE, alias));
            }
        }
        let index = Index::build(root, &config.namespaces, &members)?;
        let stray_files = crate::index::stray_files(root, &config.namespaces, &members)?;
        let imports = if follow_imports {
            let machine = crate::imports::read_machine_file(machine_file.as_ref())?;
            if let Some(file) = &machine_file {
                for alias in machine.keys().filter(|alias| {
                    !config.imports.contains_key(*alias) && schema::reserved_url_scheme(alias)
                }) {
                    schema_findings
                        .push(reserved_alias_finding(&file.display().to_string(), alias));
                }
            }
            let merged = crate::imports::merge(&config.imports, machine);
            let mut resolved = BTreeMap::new();
            for (alias, raw) in &merged {
                resolved.insert(alias.clone(), resolve_import(root, raw, env)?);
            }
            resolved
        } else {
            BTreeMap::new()
        };
        schema_findings.extend(schema_drift_findings(&qualified_targets, &imports));
        validate::order(&mut schema_findings);
        Ok(Project {
            root: root.to_owned(),
            config,
            index,
            collections: loaded,
            schema_findings,
            stray_files,
            members,
            imports,
            state: state_by_namespace,
        })
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub(crate) fn namespaces(&self) -> &[Namespace] {
        &self.config.namespaces
    }

    pub(crate) fn index_ref(&self) -> &Index {
        &self.index
    }

    pub(crate) fn root_ref(&self) -> &Path {
        &self.root
    }

    pub(crate) fn codes(&self) -> BTreeSet<String> {
        self.project_codes()
    }

    /// The namespaces a command reads. `prefix` is the namespace an argument names.
    pub fn scope(
        &self,
        prefix: Option<&str>,
        flag: Option<&str>,
        env: &dyn Env,
    ) -> Result<Scope, Error> {
        let loaded: BTreeMap<&str, Vec<String>> = self
            .imports
            .iter()
            .filter_map(|(alias, state)| match state {
                ImportState::Loaded(project) => Some((
                    alias.as_str(),
                    project
                        .config
                        .namespaces
                        .iter()
                        .map(|n| n.name.clone())
                        .collect(),
                )),
                ImportState::Absent(_) => None,
            })
            .collect();
        let listing: Vec<scope::ImportListing> = self
            .imports
            .keys()
            .map(|alias| scope::ImportListing {
                alias,
                namespaces: loaded.get(alias.as_str()).map(Vec::as_slice),
            })
            .collect();
        scope::choose(
            &self.config.namespaces,
            &self.root,
            prefix,
            flag,
            env,
            &listing,
        )
    }

    /// An import absent on this machine is bad arguments here, not the `imports.absent` finding a
    /// ref gets: an argument is a direct request for that document, as `--namespace 'alias::*'`
    /// is.
    fn imported(&self, alias: &str) -> Result<&Project, Error> {
        match self.imports.get(alias) {
            None => {
                let known: Vec<&str> = self.imports.keys().map(String::as_str).collect();
                Err(Error::BadArgument(crate::imports::unknown_alias_message(
                    alias, &known,
                )))
            }
            Some(ImportState::Absent(absence)) => Err(Error::BadArgument(format!(
                "the import `{alias}` is absent on this machine: {}",
                absence.message()
            ))),
            Some(ImportState::Loaded(project)) => Ok(project),
        }
    }

    pub fn get(&self, arg: &DocumentArg, scope: &Scope, env: &dyn Env) -> Result<Document, Error> {
        if let Some(alias) = arg.project_prefix() {
            let imported = self.imported(alias)?;
            let inner = imported_scope(imported, arg)?;
            let mut document = imported.get(&arg.without_project_prefix(), &inner, env)?;
            document.project = Some(alias.to_owned());
            return Ok(document);
        }
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
            namespace: Some(self.config.namespaces[entry.namespace].name.clone()),
            key: entry.key.clone(),
            code: collection.schema.code.clone(),
            collection: collection.name.clone(),
            schema: collection.schema.name.clone(),
            project: None,
            fields,
        })
    }

    /// `typdoc set`: writes `sets` to the document `arg` names, deciding every `ifs` condition
    /// under the same namespace lock as the write (SPC-10). Returns the document as it stands
    /// after the write. A file matched by no collection has no schema, so only its `--if` is
    /// checked.
    pub fn set(
        &self,
        arg: &DocumentArg,
        scope: &Scope,
        deps: &Deps,
        sets: &[SetOp],
        ifs: &[(String, Condition)],
        lock_timeout: Duration,
    ) -> Result<Document, Error> {
        if let Some(alias) = arg.project_prefix() {
            return Err(Error::BadArgument(format!(
                "`{alias}::` cannot be written: no command writes into another project, which is \
                 read-only here"
            )));
        }
        match self.resolve_write_target(arg, scope, deps.env)? {
            WriteTarget::Loose { path } => self.set_loose(&path, deps, sets, ifs, lock_timeout),
            WriteTarget::Collected { path } => {
                self.set_collected(&path, deps, sets, ifs, lock_timeout)
            }
        }
    }

    /// `git-common` is refused rather than attempted: nothing here resolves
    /// `git rev-parse --git-common-dir`, and taking the `local` lock path instead would take the
    /// wrong lock file without saying so.
    fn acquire_lock_for<'d>(
        &self,
        namespace_name: &str,
        deps: &Deps<'d>,
        lock_timeout: Duration,
    ) -> Result<NamespaceLock<'d>, Error> {
        let lock_path = match self.config.lock {
            LockMode::Local => {
                namespace_lock::local_namespace_lock_path(&self.root, namespace_name)
            }
            LockMode::GitCommon => {
                return Err(Error::Io {
                    file: self.root.clone(),
                    source: io::Error::other(
                        "this project's `lock` is `git-common`, which no write command supports \
                         yet",
                    ),
                });
            }
        };
        let host = deps.env.hostname();
        namespace_lock::acquire(deps.fs, deps.clock, lock_path, &host, lock_timeout)
    }

    /// Unlike [`Project::resolve`], a path matched by no collection is not an error as long as
    /// the file exists: `set` reaches such a file the same way a ref does.
    fn resolve_write_target(
        &self,
        arg: &DocumentArg,
        scope: &Scope,
        env: &dyn Env,
    ) -> Result<WriteTarget, Error> {
        let path = match arg {
            DocumentArg::Path { path, .. } => {
                if let Some((_, collections)) = self.index.overlap(path) {
                    return Err(Error::Config {
                        file: self.root.join(path),
                        message: format!(
                            "{}, so there is no one schema to write it with: see \
                             collections.overlap in a validate report",
                            overlap_message(collections)
                        ),
                    });
                }
                if self.index.get(path).is_some() {
                    path.clone()
                } else if self.root.join(path).is_file() {
                    return Ok(WriteTarget::Loose { path: path.clone() });
                } else {
                    return Err(self.not_found(path, env));
                }
            }
            DocumentArg::Key { namespace, key, .. } => {
                self.resolve_key(namespace.as_deref(), key, scope)?
            }
        };
        Ok(WriteTarget::Collected { path })
    }

    /// A write to a file matched by no collection, with no schema to validate against. A value
    /// is always kept as plain text: there is no field type to say a comma splits it into a list.
    ///
    /// `write_atomically` needs a [`NamespaceLock`], and this file belongs to no namespace, so
    /// every such write shares `.typdoc/locks/.loose.lock`. Its name starts with `.`, so no
    /// namespace can have it.
    fn set_loose(
        &self,
        path: &str,
        deps: &Deps,
        sets: &[SetOp],
        ifs: &[(String, Condition)],
        lock_timeout: Duration,
    ) -> Result<Document, Error> {
        for (raw, condition) in ifs {
            if matches!(condition, Condition::Ref(_)) {
                return Err(refby_if_unsupported(raw));
            }
        }
        let no_schema = Resolved::new(String::new(), None, BTreeMap::new());
        let file = self.root.join(path);

        // Read and `--if` under the lock, so the condition cannot go stale before the write
        // (SPC-10).
        let lock = self.acquire_lock_for(".loose", deps, lock_timeout)?;

        let text = fs::read_to_string(&file).map_err(Error::io_at(&file))?;
        let split = frontmatter::split(&text).map_err(|message| Error::Frontmatter {
            file: file.clone(),
            message,
        })?;
        let before_fields = read_fields(split.block, &no_schema, &file)?;
        let doc_now = Document {
            path: path.to_owned(),
            namespace: None,
            key: None,
            code: None,
            collection: String::new(),
            schema: String::new(),
            project: None,
            fields: before_fields.clone(),
        };
        let false_conditions = evaluate_plain_ifs(ifs, &no_schema, &doc_now, path)?;
        if !false_conditions.is_empty() {
            return Err(Error::IfFalse {
                findings: false_conditions,
            });
        }

        let mut writer = YamlSerdeWriter::new(before_fields);
        apply_ops(&mut writer, sets, &no_schema)?;
        let block_text = writer.finish().map_err(|message| Error::Frontmatter {
            file: file.clone(),
            message,
        })?;
        let after_fields = read_fields(Some(&block_text), &no_schema, &file)?;
        let candidate = splice(&block_text, &text[split.body..]);

        write_atomically(deps.fs, &lock, &file, candidate.as_bytes()).map_err(|source| {
            Error::Io {
                file: file.clone(),
                source,
            }
        })?;
        Ok(Document {
            path: path.to_owned(),
            namespace: None,
            key: None,
            code: None,
            collection: String::new(),
            schema: String::new(),
            project: None,
            fields: after_fields,
        })
    }

    fn set_collected(
        &self,
        path: &str,
        deps: &Deps,
        sets: &[SetOp],
        ifs: &[(String, Condition)],
        lock_timeout: Duration,
    ) -> Result<Document, Error> {
        #[expect(
            clippy::expect_used,
            reason = "`resolve_write_target` only ever returns `WriteTarget::Collected` for a \
                      path `self.index.get` just found `Some` for (the `Path` branch) or a path \
                      `resolve_key` read out of the same index (the `Key` branch); the index has \
                      no mutator between that check and here"
        )]
        let entry = self
            .index
            .get(path)
            .expect("just resolved through the index");
        let collection = &self.collections[entry.collection];
        let schema = &collection.schema;
        let namespace_name = self.config.namespaces[entry.namespace].name.clone();
        let name = DocName {
            path,
            namespace: &namespace_name,
            collection: &collection.name,
            key: entry.key.as_deref(),
        };

        for (raw, condition) in ifs {
            if matches!(condition, Condition::Ref(_)) {
                return Err(refby_if_unsupported(raw));
            }
        }
        for op in sets {
            if let Some(field) = schema.field(op.field())
                && field.auto.is_some()
            {
                return Err(Error::Invalid {
                    findings: vec![validate::finding(
                        &name,
                        Severity::Error,
                        "frontmatter.types",
                        Some(op.field()),
                        format!(
                            "the field `{}` is set automatically (`auto`) and cannot be written \
                             directly",
                            op.field()
                        ),
                    )],
                });
            }
        }

        let lock = self.acquire_lock_for(&namespace_name, deps, lock_timeout)?;

        let file = entry.file.clone();
        let text = fs::read_to_string(&file).map_err(Error::io_at(&file))?;
        let split = frontmatter::split(&text).map_err(|message| Error::Frontmatter {
            file: file.clone(),
            message,
        })?;
        let before_fields = read_fields(split.block, schema, &file)?;

        let doc_now = Document {
            path: path.to_owned(),
            namespace: Some(namespace_name.clone()),
            key: entry.key.clone(),
            code: schema.code.clone(),
            collection: collection.name.clone(),
            schema: schema.name.clone(),
            project: None,
            fields: before_fields.clone(),
        };
        let false_conditions = evaluate_plain_ifs(ifs, schema, &doc_now, path)?;
        if !false_conditions.is_empty() {
            return Err(Error::IfFalse {
                findings: false_conditions,
            });
        }

        let mut writer = YamlSerdeWriter::new(before_fields.clone());
        apply_ops(&mut writer, sets, schema)?;
        let block_v1 = writer.finish().map_err(|message| Error::Frontmatter {
            file: file.clone(),
            message,
        })?;
        let after_v1 = read_fields(Some(&block_v1), schema, &file)?;
        if fields_changed(&before_fields, &after_v1) {
            for (field_name, field) in schema.fields() {
                if matches!(field.auto, Some(Auto::Update)) {
                    writer.set_scalar(field_name, deps.clock.now().to_rfc3339());
                }
            }
        }
        let block_final = writer.finish().map_err(|message| Error::Frontmatter {
            file: file.clone(),
            message,
        })?;
        let after_final = read_fields(Some(&block_final), schema, &file)?;
        let candidate = splice(&block_final, &text[split.body..]);

        let mut findings = validate::check_document(
            &candidate,
            schema,
            &self.config.validation,
            &collection.validation,
            false,
            false,
            &name,
        );
        findings.extend(validate::check_transitions(
            schema,
            &before_fields,
            &after_final,
            &name,
        ));
        if !findings.iter().any(|f| f.rule == "frontmatter.parse") {
            // `refs.acyclic` against the state this write produces: `path` is scanned from
            // `candidate`, not from disk. Only a cycle on `path` through a field this write
            // changed refuses it; a cycle elsewhere, or one that was already there, is
            // `validate`'s to report (SPC-2).
            let (ref_project, acyclic) = self.ref_project_for_candidate(path, entry, &candidate)?;
            let before_map = fields_map(&before_fields);
            let after_map = fields_map(&after_final);
            findings.extend(acyclic.into_iter().filter(|finding| {
                finding.path == path
                    && finding
                        .field
                        .as_deref()
                        .is_some_and(|field| before_map.get(field) != after_map.get(field))
            }));
            findings.extend(self.check_refs(
                path,
                entry,
                &candidate,
                &name,
                false,
                false,
                &ref_project,
            ));
        }
        if findings.iter().any(|f| f.level == Severity::Error) {
            return Err(Error::Invalid { findings });
        }

        write_atomically(deps.fs, &lock, &file, candidate.as_bytes()).map_err(|source| {
            Error::Io {
                file: file.clone(),
                source,
            }
        })?;

        Ok(Document {
            path: path.to_owned(),
            namespace: Some(namespace_name),
            key: entry.key.clone(),
            code: schema.code.clone(),
            collection: collection.name.clone(),
            schema: schema.name.clone(),
            project: None,
            fields: after_final,
        })
    }

    /// `typdoc new`: creates a document, under a key allocated for a coded schema or at the path
    /// given for an uncoded one. A path must match a collection.
    pub fn new_document(
        &self,
        target: &NewTarget,
        deps: &Deps,
        sets: &[SetOp],
        lock_timeout: Duration,
        namespace_flag: Option<&str>,
    ) -> Result<Document, Error> {
        match target {
            NewTarget::Coded { code, title } => {
                self.new_coded(code, title, deps, sets, lock_timeout, namespace_flag)
            }
            NewTarget::Path { path } => self.new_uncoded(path, deps, sets, lock_timeout),
        }
    }

    /// The coded form: `typdoc new <CODE> "<title>"`.
    ///
    /// The number is allocated under the lock, from the state file read again from disk rather
    /// than `self.state`, which may predate the lock (SPC-10). The index's highest existing
    /// number is as old as the load, which is safe: a process that won the lock first raised
    /// `last` before releasing it.
    ///
    /// Validation runs before the state write, so a refused `--set` spends no number. The state
    /// write runs before the document is created, so an interruption between them leaves a
    /// skipped number, never one a later `new` could issue again (SPC-8).
    fn new_coded(
        &self,
        code: &str,
        title: &str,
        deps: &Deps,
        sets: &[SetOp],
        lock_timeout: Duration,
        namespace_flag: Option<&str>,
    ) -> Result<Document, Error> {
        let collection_idx = self
            .collections
            .iter()
            .position(|collection| collection.schema.code.as_deref() == Some(code))
            .ok_or_else(|| {
                Error::BadArgument(format!(
                    "`{code}` is not the code of any coded schema in this project"
                ))
            })?;
        let collection = &self.collections[collection_idx];
        let schema = &collection.schema;

        let scope = self.scope(None, namespace_flag, deps.env)?;
        let namespace_name = match scope.namespaces.as_slice() {
            [one] => one.clone(),
            other => {
                return Err(Error::AmbiguousScope {
                    candidates: other.to_vec(),
                });
            }
        };
        #[expect(
            clippy::expect_used,
            reason = "`namespace_name` came from `scope.namespaces`, which `Project::scope` only \
                      ever fills with names of `self.config.namespaces`"
        )]
        let namespace_idx = self
            .namespace_index(&namespace_name)
            .expect("a scoped namespace name is a namespace of this project");

        let mut all_sets = Vec::with_capacity(sets.len() + 1);
        all_sets.push(SetOp::Set {
            field: "title".to_owned(),
            raw: title.to_owned(),
        });
        all_sets.extend_from_slice(sets);

        let lock = self.acquire_lock_for(&namespace_name, deps, lock_timeout)?;

        let (key, relative, next) = self.allocate_key(namespace_idx, collection_idx)?;
        let file = self.root.join(&relative);

        let name = DocName {
            path: &relative,
            namespace: &namespace_name,
            collection: &collection.name,
            key: Some(key.as_str()),
        };
        let now = deps.clock.now().to_rfc3339();
        let block = new_block(schema, &all_sets, &now, &file)?;
        let candidate = splice(&block, "");

        let findings = self.validate_new_candidate(
            collection_idx,
            namespace_idx,
            &file,
            Some(key.clone()),
            &relative,
            &candidate,
            &all_sets,
            schema,
            &collection.validation,
            &name,
        )?;
        if findings.iter().any(|f| f.level == Severity::Error) {
            return Err(Error::Invalid { findings });
        }

        // Should not be reachable, since nobody has used this number. Checked before `last` is
        // raised, so a refusal leaves the state file untouched; `O_EXCL` on the create below
        // stays the real enforcement (SPC-10).
        if deps.fs.exists(&file).map_err(Error::io_at(&file))? {
            return Err(Error::AlreadyExists {
                path: file.display().to_string(),
                message: format!(
                    "`{relative}` already exists: the key `{key}` was just allocated and should \
                     not be reachable"
                ),
            });
        }

        state::write(
            deps.fs,
            &lock,
            &self.root,
            &namespace_name,
            &collection.name,
            next,
        )?;

        create_document_or_exists(
            deps,
            &lock,
            &file,
            candidate.as_bytes(),
            format!(
                "`{relative}` already exists: the key `{key}` was just allocated and should not \
                 be reachable, but the file system enforces the refusal either way"
            ),
        )?;

        let after_fields = read_fields(Some(&block), schema, &file)?;
        Ok(Document {
            path: relative,
            namespace: Some(namespace_name),
            key: Some(key),
            code: schema.code.clone(),
            collection: collection.name.clone(),
            schema: schema.name.clone(),
            project: None,
            fields: after_fields,
        })
    }

    /// The path-identified form: `typdoc new <path>`. The namespace and collection come from the
    /// path and the config, which no concurrent writer changes, so they are resolved before the
    /// lock.
    ///
    /// `--namespace` and `TYPDOC_NAMESPACE` play no part: the path already names its namespace
    /// folder. A `--namespace` given with a path is ignored, not checked against it.
    fn new_uncoded(
        &self,
        path: &str,
        deps: &Deps,
        sets: &[SetOp],
        lock_timeout: Duration,
    ) -> Result<Document, Error> {
        let (namespace_idx, collection_idx) = self.resolve_uncoded_target(path)?;
        let collection = &self.collections[collection_idx];
        let schema = &collection.schema;
        let namespace_name = self.config.namespaces[namespace_idx].name.clone();

        let lock = self.acquire_lock_for(&namespace_name, deps, lock_timeout)?;

        let file = self.root.join(path);
        let name = DocName {
            path,
            namespace: &namespace_name,
            collection: &collection.name,
            key: None,
        };
        let now = deps.clock.now().to_rfc3339();
        let block = new_block(schema, sets, &now, &file)?;
        let candidate = splice(&block, "");

        let findings = self.validate_new_candidate(
            collection_idx,
            namespace_idx,
            &file,
            None,
            path,
            &candidate,
            sets,
            schema,
            &collection.validation,
            &name,
        )?;
        if findings.iter().any(|f| f.level == Severity::Error) {
            return Err(Error::Invalid { findings });
        }

        create_document_or_exists(
            deps,
            &lock,
            &file,
            candidate.as_bytes(),
            format!("`{path}` already exists: choose another path or open the file that is there"),
        )?;

        let after_fields = read_fields(Some(&block), schema, &file)?;
        Ok(Document {
            path: path.to_owned(),
            namespace: Some(namespace_name),
            key: None,
            code: None,
            collection: collection.name.clone(),
            schema: schema.name.clone(),
            project: None,
            fields: after_fields,
        })
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "each argument is context `new_coded` and `new_uncoded` already hold from their \
                  own earlier steps; bundling them into a type built only to make one call \
                  shorter would be a type with no use beyond this one function, the same \
                  reasoning `check_body`'s own `#[allow(clippy::too_many_arguments)]` already \
                  gives beside it in this file"
    )]
    fn validate_new_candidate(
        &self,
        collection_idx: usize,
        namespace_idx: usize,
        file: &Path,
        key: Option<String>,
        path: &str,
        candidate: &str,
        sets: &[SetOp],
        schema: &Resolved,
        collection_validation: &Rules,
        name: &DocName,
    ) -> Result<Vec<Finding>, Error> {
        let mut findings = check_auto_direct(schema, sets, name);
        findings.extend(validate::check_document(
            candidate,
            schema,
            &self.config.validation,
            collection_validation,
            false,
            false,
            name,
        ));
        if !findings.iter().any(|f| f.rule == "frontmatter.parse") {
            let entry = Indexed {
                collection: collection_idx,
                namespace: namespace_idx,
                file: file.to_owned(),
                key,
            };
            // This document is not in `self.index` yet; `ref_project_for_candidate` scans it
            // anyway, so a document on disk that names its key or path sees it for this scan.
            let (ref_project, acyclic) = self.ref_project_for_candidate(path, &entry, candidate)?;
            findings.extend(acyclic.into_iter().filter(|finding| finding.path == path));
            findings.extend(self.check_refs(
                path,
                &entry,
                candidate,
                name,
                false,
                false,
                &ref_project,
            ));
        }
        Ok(findings)
    }

    /// The namespace and the uncoded collection of a path not yet on disk, which the index
    /// cannot answer for. More than one matching collection is refused as `collections.overlap`.
    fn resolve_uncoded_target(&self, path: &str) -> Result<(usize, usize), Error> {
        let namespace_idx = self
            .config
            .namespaces
            .iter()
            .position(|space| {
                space.folder.is_empty() || path.starts_with(&format!("{}/", space.folder))
            })
            .ok_or_else(|| {
                Error::BadArgument(format!(
                    "`{path}` is not inside any namespace of this project"
                ))
            })?;
        let namespace = &self.config.namespaces[namespace_idx];
        let below = if namespace.folder.is_empty() {
            path.to_owned()
        } else {
            #[expect(
                clippy::expect_used,
                reason = "`namespace_idx` was found above by testing exactly this condition"
            )]
            path.strip_prefix(&format!("{}/", namespace.folder))
                .expect("the namespace was found by this same prefix test")
                .to_owned()
        };

        let matches: Vec<usize> = self
            .collections
            .iter()
            .enumerate()
            .filter(|(_, collection)| collection.schema.code.is_none())
            .filter(|(i, _)| ignore_matches(&self.members[*i].template, &below))
            .map(|(i, _)| i)
            .collect();
        match matches.len() {
            1 => Ok((namespace_idx, matches[0])),
            0 => {
                let coded = self.collections.iter().enumerate().find(|(i, collection)| {
                    collection.schema.code.is_some()
                        && ignore_matches(&self.members[*i].template, &below)
                });
                if let Some((_, collection)) = coded {
                    return Err(Error::BadArgument(format!(
                        "`{path}` matches the coded collection `{}`: create it with `new <CODE> \
                         \"<title>\"` instead",
                        collection.name
                    )));
                }
                Err(Error::BadArgument(format!(
                    "`{path}` matches no collection: `new` needs the path to fit an uncoded \
                     collection's `match`"
                )))
            }
            _ => Err(Error::Config {
                file: self.root.join(path),
                message: format!(
                    "{}, so there is no one schema to write it with: see collections.overlap in \
                     a validate report",
                    overlap_message(
                        &matches
                            .iter()
                            .map(|&i| self.collections[i].name.clone())
                            .collect::<Vec<_>>()
                    )
                ),
            }),
        }
    }

    fn highest_existing(&self, namespace_idx: usize, collection_idx: usize) -> Option<u64> {
        self.index
            .iter()
            .filter(|(_, entry)| {
                entry.namespace == namespace_idx && entry.collection == collection_idx
            })
            .filter_map(|(_, entry)| entry.key.as_deref())
            .filter_map(|key| match key_sort_value(key) {
                SortValue::Key(_, number) => number,
                _ => None,
            })
            .max()
    }

    /// The next key in `(namespace_idx, collection_idx)`, its project-relative path, and its
    /// number. The caller must hold that namespace's lock, and then write the number to
    /// the state file before creating the document: this only decides the number.
    ///
    /// Refuses on `state.malformed` and `state.missing`: a guessed number is a key that already
    /// belongs to a document (SPC-8).
    fn allocate_key(
        &self,
        namespace_idx: usize,
        collection_idx: usize,
    ) -> Result<(String, String, u64), Error> {
        let namespace_name = &self.config.namespaces[namespace_idx].name;
        let collection = &self.collections[collection_idx];
        #[expect(
            clippy::expect_used,
            reason = "every collection in `self.collections` this function is ever asked about \
                      already has a code: `new_coded` finds `collection_idx` by position among \
                      collections whose `schema.code` is `Some`, and `mv_renumber` finds it from \
                      an index entry whose own `key` is `Some`, which `Index::build` only sets \
                      through a coded template (`Template::key`), itself only ever bound to a \
                      collection whose schema carries a code (`Template::bind`)"
        )]
        let code = collection
            .schema
            .code
            .as_deref()
            .expect("only ever asked about a coded collection");
        #[expect(
            clippy::expect_used,
            reason = "`self.members` and `self.collections` are pushed together, once per \
                      collection, only after `Template::bind` has already succeeded for it \
                      (`load_inner`'s own loop: `members.push` happens before `loaded.push`, and \
                      a `bind` failure `continue`s before either), so `collection_idx`, always a \
                      valid position in `self.collections`, is also one in `self.members`"
        )]
        let template = &self
            .members
            .get(collection_idx)
            .expect("kept in lockstep with collections")
            .template;

        let state = state::read(&self.root, namespace_name)?;
        if let Some(found) = state.malformed.get(&collection.name) {
            return Err(Error::Invalid {
                findings: vec![validate::state_malformed_finding(
                    &state::file_path(namespace_name),
                    namespace_name,
                    &collection.name,
                    format!(
                        "the collection `{}`'s `last` in its state file is {found}, not a whole \
                         number that can be held: expected a non-negative integer",
                        collection.name
                    ),
                )],
            });
        }
        let highest = self.highest_existing(namespace_idx, collection_idx);
        if !state.has(&collection.name) && highest.is_some() {
            return Err(Error::Invalid {
                findings: vec![validate::state_missing_finding(
                    &state::file_path(namespace_name),
                    namespace_name,
                    &collection.name,
                    format!(
                        "the collection `{}` has documents in this namespace and no `last` \
                         recorded in its state file",
                        collection.name
                    ),
                )],
            });
        }
        let last = state.last.get(&collection.name).copied().unwrap_or(0);
        let next = highest.unwrap_or(0).max(last) + 1;
        let key = format!("{code}-{next}");
        #[expect(
            clippy::expect_used,
            reason = "a coded collection's template is bound by `Template::bind` at load time, \
                      which refuses a wildcard and binds every `{key}` to the schema's code once \
                      one is given (a template that fails to bind is `config.match-template`, and \
                      that collection is never in `self.collections`), so `render` only returns \
                      `None` for a shape a bound coded template cannot have"
        )]
        let below = template
            .render(&key)
            .expect("a bound coded template always renders its own key");
        let namespace_folder = &self.config.namespaces[namespace_idx].folder;
        let relative = if namespace_folder.is_empty() {
            below
        } else {
            format!("{namespace_folder}/{below}")
        };
        Ok((key, relative, next))
    }

    /// `list`: every document of `scope` whose collection is selected and whose fields satisfy
    /// every `--where` condition, sorted by `--sort` with ties broken in key or path order. The
    /// caller applies `--limit`.
    ///
    /// A field unknown to every selected schema is an error, while a document whose own schema
    /// lacks it counts as absent. `query::evaluate` sees one schema at a time and cannot tell the
    /// two apart, so the scope-wide check is made here, before any document is read. A
    /// `ref.*`/`refby.*` field is checked against the whole project instead, since arrows leave
    /// the selected collections.
    pub fn list(&self, scope: &Scope, filter: &ListFilter) -> Result<ListResult, Error> {
        let selected = self.select_collections(filter.collections, filter.codes)?;
        for condition in filter.wheres {
            match condition {
                Condition::Plain(plain) => {
                    if let FieldRef::Named(name) = &plain.field
                        && !selected
                            .iter()
                            .any(|&i| self.collections[i].schema.field(name).is_some())
                    {
                        return Err(Error::BadArgument(
                            query::QueryError::UnknownField(name.clone()).to_string(),
                        ));
                    }
                }
                Condition::Ref(ref_condition) => self.check_ref_condition_scope(ref_condition)?,
            }
        }
        let codes = self.project_codes();
        // Only `ref.*` reads the candidate's own body links; `refby.*` reads the other documents'
        // refs, gathered once here.
        let needs_own_body = filter
            .wheres
            .iter()
            .any(|c| matches!(c, Condition::Ref(r) if r.dir == Dir::Ref));
        let incoming = if filter
            .wheres
            .iter()
            .any(|c| matches!(c, Condition::Ref(r) if r.dir == Dir::RefBy))
        {
            Some(self.incoming_refs(&codes)?)
        } else {
            None
        };
        let mut matched: Vec<(usize, Document)> = Vec::new();
        let mut dangling_refs: Vec<String> = Vec::new();
        for (path, entry) in self.index.iter() {
            if !selected.contains(&entry.collection) {
                continue;
            }
            let namespace = &self.config.namespaces[entry.namespace].name;
            if !scope.contains(namespace) {
                continue;
            }
            let collection = &self.collections[entry.collection];
            let text = fs::read_to_string(&entry.file).map_err(Error::io_at(&entry.file))?;
            let Some(fields) = parsed_fields(&text, &collection.schema) else {
                continue;
            };
            let doc = Document {
                path: path.to_owned(),
                namespace: Some(namespace.clone()),
                key: entry.key.clone(),
                code: collection.schema.code.clone(),
                collection: collection.name.clone(),
                schema: collection.schema.name.clone(),
                project: None,
                fields: fields.clone(),
            };
            #[expect(
                clippy::expect_used,
                reason = "`links::scan` returns an error only when `frontmatter::split` does on the \
                          same text, and `frontmatter::block` is `split` with the body offset dropped; \
                          `parsed_fields` above returned `Some` for `text`, which needs `block(&text)` \
                          to be `Ok`, and a `None` there is a `continue`"
            )]
            let body = if needs_own_body {
                links::scan(&text).expect("frontmatter.parse already refused an unclosed block")
            } else {
                BodyLinks::default()
            };
            let me = RefName {
                path: path.to_owned(),
                namespace: Some(namespace.clone()),
                key: entry.key.clone(),
                project: None,
            };
            let ref_ctx = RefEvalCtx {
                me: &me,
                me_entry: entry,
                me_fields: &fields,
                me_body: &body,
                codes: &codes,
                incoming: incoming.as_deref(),
            };
            let mut keep = true;
            for condition in filter.wheres {
                let matches = match condition {
                    Condition::Plain(plain) => condition_matches(plain, &collection.schema, &doc)?,
                    Condition::Ref(ref_condition) => {
                        self.evaluate_ref_condition(ref_condition, &ref_ctx, &mut dangling_refs)?
                    }
                };
                if !matches {
                    keep = false;
                    break;
                }
            }
            if keep {
                matched.push((entry.collection, doc));
            }
        }
        matched.sort_by(|(ca, a), (cb, b)| {
            for key in filter.sort {
                let ord = sort_compare(
                    key,
                    &self.collections[*ca].schema,
                    a,
                    &self.collections[*cb].schema,
                    b,
                );
                if ord != Ordering::Equal {
                    return ord;
                }
            }
            compare_identity(a, b)
        });
        // Several candidates or conditions can reach one dangling ref.
        dangling_refs.sort();
        dangling_refs.dedup();
        Ok(ListResult {
            documents: matched.into_iter().map(|(_, doc)| doc).collect(),
            dangling_refs,
        })
    }

    /// `list`, widened to the imports `scope.imports` names (`--namespace 'chief::*'`). Each
    /// import runs its own `list` against its own schemas. An import's dangling-ref lines are
    /// prefixed with its alias, since a bare path from another project is ambiguous with this
    /// one's.
    pub fn list_all(&self, scope: &Scope, filter: &ListFilter) -> Result<ListResult, Error> {
        let mut result = self.list(scope, filter)?;
        if scope.imports.is_empty() {
            return Ok(result);
        }
        for (alias, namespaces) in &scope.imports {
            #[expect(
                clippy::unreachable,
                reason = "`scope.imports` is filled only by `scope::select`, reached through \
                          `Project::scope`, which lists the keys of `self.imports` and gives an \
                          alias no namespaces when its state is `Absent`; `select` returns an error \
                          for an alias outside that list and for one with no namespaces, so an alias \
                          in a `Scope` this project's `scope` returned is a `Loaded` entry of \
                          `self.imports`. A `Scope` built by hand with another alias would reach it"
            )]
            let Some(ImportState::Loaded(imported)) = self.imports.get(alias) else {
                unreachable!(
                    "scope::select only ever names an alias this project has loaded: {alias}"
                )
            };
            let import_scope = Scope {
                source: scope.source,
                namespaces: namespaces.clone(),
                imports: Vec::new(),
            };
            let mut theirs = imported.list(&import_scope, filter)?;
            for doc in &mut theirs.documents {
                doc.project = Some(alias.clone());
            }
            result.documents.extend(theirs.documents);
            result.dangling_refs.extend(
                theirs
                    .dangling_refs
                    .into_iter()
                    .map(|w| format!("{alias}::{w}")),
            );
        }
        result.documents.sort_by(|a, b| {
            for key in filter.sort {
                let schema_a = self.schema_for(a);
                let schema_b = self.schema_for(b);
                let ord = sort_compare(key, schema_a, a, schema_b, b);
                if ord != Ordering::Equal {
                    return ord;
                }
            }
            compare_identity(a, b)
        });
        result.dangling_refs.sort();
        result.dangling_refs.dedup();
        Ok(result)
    }

    #[expect(
        clippy::panic,
        reason = "the only caller is the sort in `list_all`, on documents that `Project::list` \
                  built with `collection: collection.name.clone()` from an entry of the \
                  `collections` of the project it ran on: `self` when `doc.project` is `None`, and \
                  otherwise the import `doc.project` names, which `list_all` reached through the \
                  alias check of its own loop over `scope.imports`; `schema_of_collection` looks \
                  that name up in the `collections` of that same project"
    )]
    fn schema_for(&self, doc: &Document) -> &Resolved {
        let project = match &doc.project {
            None => self,
            Some(alias) => match self.imports.get(alias) {
                Some(ImportState::Loaded(imported)) => imported,
                _ => self,
            },
        };
        project
            .schema_of_collection(&doc.collection)
            .unwrap_or_else(|| {
                panic!(
                    "list_all only ever builds a Document from a real collection: {} has none named {}",
                    doc.path, doc.collection
                )
            })
    }

    fn schema_of_collection(&self, name: &str) -> Option<&Resolved> {
        self.collections
            .iter()
            .find(|c| c.name == name)
            .map(|c| &c.schema)
    }

    /// A name or a code that matches nothing is refused, as an unknown namespace is: it is more
    /// likely a typo than an intended empty scope.
    fn select_collections(
        &self,
        collections: &[String],
        codes: &[String],
    ) -> Result<Vec<usize>, Error> {
        if collections.is_empty() && codes.is_empty() {
            return Ok((0..self.collections.len()).collect());
        }
        let mut selected = BTreeSet::new();
        for name in collections {
            let found = self
                .collections
                .iter()
                .position(|c| &c.name == name)
                .ok_or_else(|| {
                    Error::BadArgument(format!("`{name}` is not a collection of this project"))
                })?;
            selected.insert(found);
        }
        for code in codes {
            let matches: Vec<usize> = self
                .collections
                .iter()
                .enumerate()
                .filter(|(_, c)| c.schema.code.as_deref() == Some(code.as_str()))
                .map(|(i, _)| i)
                .collect();
            if matches.is_empty() {
                return Err(Error::BadArgument(format!(
                    "`{code}` is not the code of any schema in this project"
                )));
            }
            selected.extend(matches);
        }
        Ok(selected.into_iter().collect())
    }

    /// Never narrowed to the selected collections: the arrows a `ref.*`/`refby.*` condition
    /// follows are not bound by them.
    fn schemas_declaring_ref_field(&self, field: &RefField) -> Vec<usize> {
        let RefField::Named(name) = field else {
            return (0..self.collections.len()).collect();
        };
        self.collections
            .iter()
            .enumerate()
            .filter(|(_, c)| {
                matches!(
                    c.schema.field(name).map(|f| &f.kind),
                    Some(FieldType::Ref | FieldType::RefList)
                )
            })
            .map(|(i, _)| i)
            .collect()
    }

    /// The scope of the condition after `ref.*(f)`/`refby.*(f)`. An invalid `target`
    /// (`Target::Other`) restricts nothing, like `"*"`: it is already reported under
    /// `schema.valid`.
    fn ref_condition_inner_scope(
        &self,
        dir: Dir,
        field: &RefField,
        declaring: &[usize],
    ) -> Vec<usize> {
        if matches!(field, RefField::Body) {
            return (0..self.collections.len()).collect();
        }
        if matches!(dir, Dir::RefBy) {
            return declaring.to_vec();
        }
        #[expect(
            clippy::unreachable,
            reason = "`RefField` has two variants, `Named` and `Body`, and the first `if` of this \
                      function returns when `field` is `Body`"
        )]
        let RefField::Named(name) = field else {
            unreachable!("RefField::Body already returned above")
        };
        let mut unrestricted = false;
        let mut names: BTreeSet<&str> = BTreeSet::new();
        for &i in declaring {
            let target = self.collections[i]
                .schema
                .field(name)
                .and_then(|f| f.target.as_ref());
            match target {
                None | Some(schema::Target::Any) | Some(schema::Target::Other(_)) => {
                    unrestricted = true;
                }
                Some(schema::Target::Schemas(list)) => {
                    names.extend(list.iter().map(String::as_str));
                }
            }
        }
        if unrestricted {
            return (0..self.collections.len()).collect();
        }
        self.collections
            .iter()
            .enumerate()
            .filter(|(_, c)| names.contains(c.schema.name.as_str()))
            .map(|(i, _)| i)
            .collect()
    }

    fn check_ref_condition_scope(&self, condition: &RefCondition) -> Result<(), Error> {
        let declaring = self.schemas_declaring_ref_field(&condition.field);
        if !matches!(condition.field, RefField::Body) && declaring.is_empty() {
            return Err(Error::BadArgument(
                query::QueryError::UnknownField(condition.field.name().to_owned()).to_string(),
            ));
        }
        let Some(inner) = &condition.inner else {
            return Ok(());
        };
        let FieldRef::Named(name) = &inner.field else {
            return Ok(());
        };
        let inner_scope =
            self.ref_condition_inner_scope(condition.dir, &condition.field, &declaring);
        if !inner_scope
            .iter()
            .any(|&i| self.collections[i].schema.field(name).is_some())
        {
            return Err(Error::BadArgument(
                query::QueryError::UnknownField(name.clone()).to_string(),
            ));
        }
        Ok(())
    }

    /// A dangling ref is counted as an arrow, and every one a `ref.*` arrow reaches is a warning,
    /// whatever the inner condition decides.
    fn evaluate_ref_condition(
        &self,
        condition: &RefCondition,
        ctx: &RefEvalCtx<'_>,
        warnings: &mut Vec<String>,
    ) -> Result<bool, Error> {
        let wanted = condition.field.name();
        match condition.dir {
            Dir::Ref => {
                let refs: Vec<RefsReference> = self
                    .document_out_refs(
                        &ctx.me.path,
                        ctx.me_entry,
                        ctx.me_fields,
                        ctx.me_body,
                        ctx.codes,
                    )
                    .into_iter()
                    .filter(|r| r.field == wanted)
                    .collect();
                let mut results = Vec::with_capacity(refs.len());
                for r in &refs {
                    if let RefOutcome::Unresolved(reason_id) = &r.other {
                        warnings.push(format!(
                            "{}: the ref `{}` in `{}` does not resolve ({reason_id})",
                            ctx.me.path, r.written, r.field
                        ));
                    }
                    results.push(match &condition.inner {
                        None => true,
                        Some(inner) => match &r.other {
                            RefOutcome::Unresolved(_) => inner.op.absent_result(),
                            RefOutcome::Resolved(target) => {
                                self.evaluate_reached_target(target, inner)?
                            }
                        },
                    });
                }
                Ok(combine_quant(condition.quant, results.into_iter()))
            }
            Dir::RefBy => {
                #[expect(
                    clippy::expect_used,
                    reason = "`evaluate_ref_condition` has one caller, `Project::list`, which builds \
                              the one `RefEvalCtx` and sets `incoming` to `Some` whenever a \
                              `Condition::Ref` with `dir == Dir::RefBy` is among `filter.wheres`; \
                              the `condition` it passes is one of those `filter.wheres`, and this \
                              arm runs only for `Dir::RefBy`"
                )]
                let incoming = ctx
                    .incoming
                    .expect("Project::list precomputes this whenever a refby condition is present");
                let mut results = Vec::new();
                for item in incoming {
                    if item.reference.field != wanted {
                        continue;
                    }
                    let RefOutcome::Resolved(target) = &item.reference.other else {
                        continue;
                    };
                    if !target.is_same_document(ctx.me) {
                        continue;
                    }
                    results.push(match &condition.inner {
                        None => true,
                        Some(inner) => condition_matches(
                            inner,
                            &self.collections[item.collection].schema,
                            &item.holder,
                        )?,
                    });
                }
                Ok(combine_quant(condition.quant, results.into_iter()))
            }
        }
    }

    /// `target.project` only ever names a loaded import; the fallback reads this project rather
    /// than crash a query over it.
    fn evaluate_reached_target(
        &self,
        target: &RefName,
        inner: &PlainCondition,
    ) -> Result<bool, Error> {
        match &target.project {
            None => self.evaluate_reached(&target.path, inner),
            Some(alias) => match self.imports.get(alias) {
                Some(ImportState::Loaded(imported)) => {
                    imported.evaluate_reached(&target.path, inner)
                }
                _ => self.evaluate_reached(&target.path, inner),
            },
        }
    }

    /// A target outside every collection (reachable through `target: "*"`) has no schema, so its
    /// named fields read as absent while its pseudo-fields still apply. A target whose block
    /// cannot be parsed reads as absent too, never as an error.
    fn evaluate_reached(&self, path: &str, inner: &PlainCondition) -> Result<bool, Error> {
        let Some(entry) = self.index.get(path) else {
            let name = self.ref_name_of(path);
            let empty_schema = Resolved::new(String::new(), None, BTreeMap::new());
            let doc = Document {
                path: path.to_owned(),
                namespace: name.namespace,
                key: None,
                code: None,
                collection: String::new(),
                schema: String::new(),
                project: None,
                fields: Vec::new(),
            };
            return condition_matches(inner, &empty_schema, &doc);
        };
        let collection = &self.collections[entry.collection];
        let text = fs::read_to_string(&entry.file).map_err(Error::io_at(&entry.file))?;
        let Some(fields) = parsed_fields(&text, &collection.schema) else {
            return Ok(inner.op.absent_result());
        };
        let doc = Document {
            path: path.to_owned(),
            namespace: Some(self.config.namespaces[entry.namespace].name.clone()),
            key: entry.key.clone(),
            code: collection.schema.code.clone(),
            collection: collection.name.clone(),
            schema: collection.schema.name.clone(),
            project: None,
            fields,
        };
        condition_matches(inner, &collection.schema, &doc)
    }

    fn incoming_refs(&self, codes: &BTreeSet<String>) -> Result<Vec<IncomingRef>, Error> {
        let mut holders: Vec<(&str, &Indexed)> = self.index.iter().collect();
        holders.sort_by_key(|(path, _)| *path);
        let mut out = Vec::new();
        for (path, entry) in holders {
            let collection = &self.collections[entry.collection];
            let text = fs::read_to_string(&entry.file).map_err(Error::io_at(&entry.file))?;
            let Some((fields, body)) = parsed_fields_and_body(&text, &collection.schema) else {
                continue;
            };
            let holder = Document {
                path: path.to_owned(),
                namespace: Some(self.config.namespaces[entry.namespace].name.clone()),
                key: entry.key.clone(),
                code: collection.schema.code.clone(),
                collection: collection.name.clone(),
                schema: collection.schema.name.clone(),
                project: None,
                fields: fields.clone(),
            };
            for reference in self.document_out_refs(path, entry, &fields, &body, codes) {
                out.push(IncomingRef {
                    holder: holder.clone(),
                    collection: entry.collection,
                    reference,
                });
            }
        }
        Ok(out)
    }

    /// The headings of a document's body, in the order of `line`.
    pub fn toc(&self, arg: &DocumentArg, scope: &Scope, env: &dyn Env) -> Result<Toc, Error> {
        if let Some(alias) = arg.project_prefix() {
            let imported = self.imported(alias)?;
            let inner = imported_scope(imported, arg)?;
            let mut toc = imported.toc(&arg.without_project_prefix(), &inner, env)?;
            toc.project = Some(alias.to_owned());
            return Ok(toc);
        }
        let (path, entry, text) = self.resolve(arg, scope, env)?;
        let headings = body::headings(&text).map_err(|message| Error::Frontmatter {
            file: entry.file.clone(),
            message,
        })?;
        Ok(Toc {
            path,
            namespace: self.config.namespaces[entry.namespace].name.clone(),
            key: entry.key.clone(),
            project: None,
            headings,
        })
    }

    /// `refs`: the refs the document holds, or with `reverse` every ref in this project that
    /// resolves to it. `field` keeps only the refs held in that field (`"$body"` for body links),
    /// in either direction.
    ///
    /// `reverse` scans every document of this project regardless of `scope`, and no imported
    /// project. With a `project::` argument it is refused: delegating to the import's own reverse
    /// scan would leave out the refs this project holds without saying so.
    pub fn refs(
        &self,
        arg: &DocumentArg,
        scope: &Scope,
        reverse: bool,
        field: Option<&str>,
        env: &dyn Env,
    ) -> Result<RefsReport, Error> {
        if let Some(alias) = arg.project_prefix() {
            if reverse {
                return Err(Error::BadArgument(format!(
                    "`{alias}::` with --reverse is not built yet: refs from this project into an imported document are not scanned; run refs --reverse inside the imported project instead"
                )));
            }
            let imported = self.imported(alias)?;
            let inner = imported_scope(imported, arg)?;
            let mut report =
                imported.refs(&arg.without_project_prefix(), &inner, false, field, env)?;
            report.document.project = Some(alias.to_owned());
            return Ok(report);
        }
        let (path, entry, text) = self.resolve(arg, scope, env)?;
        let document = self.ref_name_of(&path);
        let codes = self.project_codes();
        let bad = |message| Error::Frontmatter {
            file: entry.file.clone(),
            message,
        };
        let collection = &self.collections[entry.collection];
        let fields = match frontmatter::block(&text).map_err(bad)? {
            Some(block) => frontmatter::fields(block, &collection.schema).map_err(bad)?,
            None => Vec::new(),
        };
        #[expect(
            clippy::expect_used,
            reason = "`links::scan` returns an error only when `frontmatter::split` does on the same \
                      text, and `frontmatter::block` is `split` with the body offset dropped; \
                      `frontmatter::block(&text).map_err(bad)?` above already returned the error for \
                      this `text`"
        )]
        let body = links::scan(&text).expect("frontmatter.parse already refused an unclosed block");
        let own = self.document_out_refs(&path, entry, &fields, &body, &codes);

        if !reverse {
            let refs = filtered(own, field);
            return Ok(RefsReport {
                document,
                direction: RefsDirection::Out,
                refs,
            });
        }

        // `in` is ordered by the holder's path, and `Index::iter` promises no order.
        let mut holders: Vec<(&str, &Indexed)> = self.index.iter().collect();
        holders.sort_by_key(|(other_path, _)| *other_path);
        let mut refs = Vec::new();
        for (other_path, other_entry) in holders {
            let other_collection = &self.collections[other_entry.collection];
            let other_text =
                fs::read_to_string(&other_entry.file).map_err(Error::io_at(&other_entry.file))?;
            let Some((other_fields, other_body)) =
                parsed_fields_and_body(&other_text, &other_collection.schema)
            else {
                continue;
            };
            let outgoing =
                self.document_out_refs(other_path, other_entry, &other_fields, &other_body, &codes);
            for reference in filtered(outgoing, field) {
                let RefOutcome::Resolved(target) = &reference.other else {
                    continue;
                };
                if !target.is_same_document(&document) {
                    continue;
                }
                refs.push(RefsReference {
                    other: RefOutcome::Resolved(self.ref_name_of(other_path)),
                    field: reference.field,
                    written: reference.written,
                    position: reference.position,
                });
            }
        }
        Ok(RefsReport {
            document,
            direction: RefsDirection::In,
            refs,
        })
    }

    /// Unresolved refs are kept: `refs` reports them, and `ref.*` counts them.
    fn document_out_refs(
        &self,
        path: &str,
        entry: &Indexed,
        fields: &[(String, Value)],
        body: &BodyLinks,
        codes: &BTreeSet<String>,
    ) -> Vec<RefsReference> {
        let collection = &self.collections[entry.collection];
        let ctx = refs::Ctx {
            doc_namespace: entry.namespace,
            doc_path: path,
            ref_base: collection.ref_base,
            namespaces: &self.config.namespaces,
            codes,
            index: &self.index,
            root: &self.root,
            imports: &self.imports,
        };
        let mut refs = Vec::new();
        for (field_name, value) in fields {
            let Some(field) = collection.schema.field(field_name) else {
                continue;
            };
            if !matches!(field.kind, FieldType::Ref | FieldType::RefList)
                || !crate::coerce::fits(&field.kind, value)
            {
                continue;
            }
            for written in ref_values(value) {
                let other = match refs::resolve_one(written, &ctx) {
                    Ok(resolved) => RefOutcome::Resolved(self.ref_name_of_resolved(&resolved)),
                    Err(reason) => RefOutcome::Unresolved(reason_id(&reason)),
                };
                refs.push(RefsReference {
                    other,
                    field: field_name.clone(),
                    written: written.to_owned(),
                    position: None,
                });
            }
        }
        let mut links: Vec<&BodyLink> = body.links.iter().collect();
        links.sort_by_key(|link| (link.line, link.col));
        for link in links {
            // `[t]()` and `[t](#anchor)` name no other document, so, like a URL-scheme link,
            // they are not refs.
            let Some(target) = &link.target else {
                continue;
            };
            let other = match refs::classify_body(target, &ctx) {
                refs::BodyDestination::Skip => continue,
                refs::BodyDestination::BadPrefix => RefOutcome::Unresolved("bad-prefix"),
                refs::BodyDestination::ImportAbsent(_) => RefOutcome::Unresolved("import-absent"),
                refs::BodyDestination::Path(joined) => {
                    match refs::resolve_path(&joined, &self.index, &self.root) {
                        Ok(resolved) => RefOutcome::Resolved(self.ref_name_of(&resolved.path)),
                        Err(_) => RefOutcome::Unresolved("not-found"),
                    }
                }
                refs::BodyDestination::Import { alias, path } => {
                    self.resolve_import_outcome(&alias, &path)
                }
            };
            refs.push(RefsReference {
                other,
                field: "$body".to_owned(),
                written: link.written.clone(),
                position: Some(Position {
                    line: link.line,
                    col: link.col,
                }),
            });
        }
        refs
    }

    fn ref_name_of(&self, path: &str) -> RefName {
        ref_name_in(&self.config.namespaces, &self.index, None, path)
    }

    /// A ref that crossed an import is named in the imported project's namespaces, not this
    /// one's.
    fn ref_name_of_resolved(&self, resolved: &refs::Resolved) -> RefName {
        match &resolved.project {
            None => self.ref_name_of(&resolved.path),
            Some(alias) => match self.imports.get(alias) {
                Some(ImportState::Loaded(imported)) => ref_name_in(
                    imported.namespaces(),
                    imported.index_ref(),
                    Some(alias),
                    &resolved.path,
                ),
                _ => self.ref_name_of(&resolved.path),
            },
        }
    }

    /// Whether the target has the linked `#anchor` is not checked: that needs the imported
    /// project's headings read under its own rules.
    fn resolve_import_outcome(&self, alias: &str, path: &str) -> RefOutcome {
        #[expect(
            clippy::unreachable,
            reason = "both callers, `document_out_refs` and `check_body_destination` (reached only \
                      from `check_body`), get `alias` from a `BodyDestination::Import` that \
                      `refs::classify_body` builds only in its `Some(ImportState::Loaded(_))` arm \
                      of `ctx.imports.get(alias)`, and each builds that `ctx` with `imports: \
                      &self.imports`, the map read here"
        )]
        let Some(ImportState::Loaded(imported)) = self.imports.get(alias) else {
            unreachable!(
                "classify_body builds BodyDestination::Import only for a loaded import: {alias}"
            )
        };
        match refs::resolve_path(path, imported.index_ref(), imported.root_ref()) {
            Ok(resolved) => RefOutcome::Resolved(ref_name_in(
                imported.namespaces(),
                imported.index_ref(),
                Some(alias),
                &resolved.path,
            )),
            Err(_) => RefOutcome::Unresolved("not-found"),
        }
    }

    /// The report of `validate`: `Schemas` when `schemas_only`, `Paths` when `args` is not
    /// empty, and `All` otherwise. The caller refuses `schemas_only` with arguments and `audit`
    /// with either, so only `All` reads `audit`.
    pub fn validate(
        &self,
        args: &[DocumentArg],
        schemas_only: bool,
        strict: bool,
        audit: bool,
        flag: Option<&str>,
        env: &dyn Env,
    ) -> Result<ValidateReport, Error> {
        if schemas_only {
            let scope = self.scope(None, flag, env)?;
            reject_import_scope(&scope)?;
            let mut findings = self.schema_findings.clone();
            findings.extend(self.shadowed_names_findings(strict, false));
            validate::order(&mut findings);
            return Ok(ValidateReport {
                scope: ValidateScope::Schemas,
                strict,
                namespaces: scope.namespaces,
                documents: 0,
                paths: None,
                findings,
                audit: None,
            });
        }
        if args.is_empty() {
            let scope = self.scope(None, flag, env)?;
            reject_import_scope(&scope)?;
            let (ref_project, acyclic) = self.ref_project()?;
            let mut findings = self.schema_findings.clone();
            findings.extend(self.shadowed_names_findings(strict, audit));
            findings.extend(acyclic);
            let mut namespaces = BTreeSet::new();
            let mut documents = 0usize;
            // `state.missing` reports a collection only once it has a document in the namespace:
            // with none and no record, the collection is new.
            let mut present: BTreeSet<(usize, usize)> = BTreeSet::new();
            let mut highest: BTreeMap<(usize, usize), u64> = BTreeMap::new();
            // `--audit` only. A document with no frontmatter still counts for its collection,
            // which matched it, but is listed rather than evaluated.
            let mut documents_by_collection: BTreeMap<usize, usize> = BTreeMap::new();
            let mut no_frontmatter: Vec<String> = Vec::new();
            for (path, entry) in self.index.iter() {
                let namespace = &self.config.namespaces[entry.namespace].name;
                if !scope.contains(namespace) {
                    continue;
                }
                namespaces.insert(namespace.clone());
                present.insert((entry.namespace, entry.collection));
                if let Some(key) = &entry.key
                    && let SortValue::Key(_, Some(number)) = key_sort_value(key)
                {
                    highest
                        .entry((entry.namespace, entry.collection))
                        .and_modify(|top| *top = (*top).max(number))
                        .or_insert(number);
                }
                let text = fs::read_to_string(&entry.file).map_err(Error::io_at(&entry.file))?;
                if audit {
                    *documents_by_collection.entry(entry.collection).or_insert(0) += 1;
                    if matches!(frontmatter::block(&text), Ok(None)) {
                        no_frontmatter.push(path.to_owned());
                        continue;
                    }
                }
                documents += 1;
                findings.extend(self.check_entry(path, entry, &text, strict, audit, &ref_project));
            }
            findings.extend(self.state_missing_findings(&present, &scope));
            findings.extend(self.state_malformed_findings(&scope));
            findings.extend(self.state_behind_findings(&highest, &scope));
            findings.extend(self.state_retired_findings(&scope));
            // An overlapping path has no one schema to be checked against, so it is not counted
            // in `documents`, though its namespace was covered.
            let mut overlapping: Vec<AuditOverlap> = Vec::new();
            for (path, namespace_idx, collections) in self.index.overlaps() {
                let namespace = &self.config.namespaces[namespace_idx].name;
                if !scope.contains(namespace) {
                    continue;
                }
                namespaces.insert(namespace.clone());
                if audit {
                    overlapping.push(AuditOverlap {
                        path: path.to_owned(),
                        collections: collections.to_vec(),
                    });
                }
                findings.push(validate::overlap_finding(
                    path,
                    namespace,
                    overlap_message(collections),
                ));
            }
            // An entry the walk could not read is no document. The run still answered for every
            // file beside it, so it is a finding, not a failure.
            let mut not_read: Vec<AuditNotRead> = Vec::new();
            for (path, namespace_idx, why) in self.index.unreadable() {
                let namespace = &self.config.namespaces[namespace_idx].name;
                if !scope.contains(namespace) {
                    continue;
                }
                namespaces.insert(namespace.clone());
                if audit {
                    not_read.push(AuditNotRead {
                        path: path.to_owned(),
                        reason: why.to_owned(),
                    });
                }
                findings.push(if why == LEFTOVER_TEMP_FILE {
                    validate::leftover_finding(path, Some(namespace), why.to_owned())
                } else {
                    validate::unreadable_finding(path, Some(namespace), why.to_owned())
                });
            }
            // Reached by a `namespaces` glob, so in no namespace and reported whatever the scope.
            // Only a folder can be a namespace, so none of these is a leftover temp file.
            for skipped in &self.config.skipped {
                if audit {
                    not_read.push(AuditNotRead {
                        path: skipped.path.clone(),
                        reason: skipped.why.to_owned(),
                    });
                }
                findings.push(validate::unreadable_finding(
                    &skipped.path,
                    None,
                    skipped.why.to_owned(),
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
                    audit,
                ) {
                    findings.push(validate::stray_file_finding(
                        level,
                        path,
                        namespace,
                        "this file fits no collection's match template in this folder".to_owned(),
                    ));
                }
            }
            let audit_report = if audit {
                Some(self.audit_report(
                    &scope,
                    &mut namespaces,
                    documents_by_collection,
                    no_frontmatter,
                    overlapping,
                    not_read,
                )?)
            } else {
                None
            };
            // Once per project, whatever the scope: collections belong to the project, not to a
            // namespace (SPC-1).
            if self.collections.is_empty() {
                findings.push(validate::collections_empty_finding(TYPDOC_DIR));
            }
            validate::order(&mut findings);
            return Ok(ValidateReport {
                scope: ValidateScope::All,
                strict,
                namespaces: namespaces.into_iter().collect(),
                documents,
                paths: None,
                findings,
                audit: audit_report,
            });
        }
        let (ref_project, acyclic) = self.ref_project()?;
        let mut findings = Vec::new();
        let mut namespaces = BTreeSet::new();
        let mut paths = BTreeSet::new();
        // An overlapping path named directly is a finding here, not a refusal as in `get` and
        // `toc`: only an ambiguous key or an argument that names no document stops `validate`
        // before its report. It is checked against no schema, so it stays out of `paths`.
        let mut overlapping = BTreeSet::new();
        for arg in args {
            if let Some(alias) = arg.project_prefix() {
                return Err(Error::BadArgument(format!(
                    "`{alias}::` cannot be validated from here: a file is validated only by the project that owns it"
                )));
            }
            let scope = self.scope(arg.namespace_prefix(), flag, env)?;
            reject_import_scope(&scope)?;
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
                findings.extend(self.check_entry(&path, entry, &text, strict, false, &ref_project));
            }
        }
        // Only a cycle through a named document is reported: every finding in this scope is
        // about a document the caller named.
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
            audit: None,
        })
    }

    /// `uncollected` comes from a walk of every namespace folder in scope, never from a
    /// collection's `match`: whether a file matches none is the question being asked.
    fn audit_report(
        &self,
        scope: &Scope,
        namespaces: &mut BTreeSet<String>,
        documents_by_collection: BTreeMap<usize, usize>,
        mut no_frontmatter: Vec<String>,
        mut overlapping: Vec<AuditOverlap>,
        mut not_read: Vec<AuditNotRead>,
    ) -> Result<AuditReport, Error> {
        let mut collections: Vec<AuditCollection> = self
            .collections
            .iter()
            .enumerate()
            .map(|(at, collection)| AuditCollection {
                name: collection.name.clone(),
                documents: documents_by_collection.get(&at).copied().unwrap_or(0),
            })
            .collect();
        collections.sort_by(|a, b| a.name.cmp(&b.name));
        let mut uncollected = Vec::new();
        for (path, namespace_idx) in
            crate::index::all_markdown_files(&self.root, &self.config.namespaces, &self.members)?
        {
            let namespace = &self.config.namespaces[namespace_idx].name;
            if !scope.contains(namespace) {
                continue;
            }
            if self.index.get(&path).is_some() || self.index.overlap(&path).is_some() {
                continue;
            }
            // A namespace whose only file in scope is uncollected was still covered.
            namespaces.insert(namespace.clone());
            uncollected.push(path);
        }
        uncollected.sort();
        no_frontmatter.sort();
        for overlap in &mut overlapping {
            overlap.collections.sort();
        }
        overlapping.sort_by(|a, b| a.path.cmp(&b.path));
        not_read.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(AuditReport {
            collections,
            uncollected,
            no_frontmatter,
            overlapping,
            not_read,
        })
    }

    /// An overlapping path is not in the index, so `collections.overlap` is left to the
    /// whole-project scan.
    fn check_entry(
        &self,
        path: &str,
        entry: &Indexed,
        text: &str,
        strict: bool,
        audit: bool,
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
            audit,
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
            findings.extend(self.check_refs(path, entry, text, &name, strict, audit, ref_project));
            findings.extend(self.check_body(path, entry, text, &name, strict, audit, ref_project));
        }
        findings
    }

    /// A value that does not fit its field's type is skipped: `frontmatter.types` already
    /// reported it.
    #[allow(
        clippy::too_many_arguments,
        reason = "each part is context this document's own check already holds (its path, its
    index entry, its text, its name, strict, audit, the whole-project ref context); bundling them
    would hide which one changes at the call site, the same reasoning this file already gives a
    comparable list"
    )]
    fn check_refs(
        &self,
        path: &str,
        entry: &Indexed,
        text: &str,
        name: &DocName,
        strict: bool,
        audit: bool,
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
            imports: &self.imports,
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
                        audit,
                    )),
                    Ok(resolved) => {
                        let info = self.schema_info_of(&resolved);
                        if !refs::target_allowed(field.target.as_ref(), &resolved, info.as_ref()) {
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
                        let coded = info.is_some_and(|found| found.code.is_some());
                        if coded
                            && resolved.via == refs::Via::Path
                            && let Some(level) = validate::effective_level(
                                Level::Warn,
                                "refs.codedByPath",
                                &self.config.validation,
                                &collection.validation,
                                strict,
                                audit,
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

    #[allow(
        clippy::too_many_arguments,
        reason = "each part is context this document's own check already holds, the same list
    `check_refs` beside it already gives a reason for"
    )]
    fn check_body(
        &self,
        path: &str,
        entry: &Indexed,
        text: &str,
        name: &DocName,
        strict: bool,
        audit: bool,
        ref_project: &RefProject,
    ) -> Vec<Finding> {
        let collection = &self.collections[entry.collection];
        let links_level = validate::effective_level(
            Level::Error,
            "body.links",
            &self.config.validation,
            &collection.validation,
            strict,
            audit,
        );
        let anchors_level = validate::effective_level(
            Level::Error,
            "body.anchors",
            &self.config.validation,
            &collection.validation,
            strict,
            audit,
        );
        let mentions_level = validate::effective_level(
            Level::Off,
            "body.mentions",
            &self.config.validation,
            &collection.validation,
            strict,
            audit,
        );
        if links_level.is_none() && anchors_level.is_none() && mentions_level.is_none() {
            return Vec::new();
        }

        #[expect(
            clippy::expect_used,
            reason = "`links::scan` returns an error only when `frontmatter::split` does on the same \
                      text, and `frontmatter::block` is `split` with the body offset dropped; \
                      `check_entry` is the only caller of `check_body` and calls it only when \
                      `validate::check_document` gave no `frontmatter.parse` finding for this `text`, \
                      a finding it returns at a fixed `Severity::Error` whenever `block(text)` fails"
        )]
        let scanned =
            links::scan(text).expect("frontmatter.parse already refused an unclosed block");
        let ctx = refs::Ctx {
            doc_namespace: entry.namespace,
            doc_path: path,
            ref_base: collection.ref_base,
            namespaces: &self.config.namespaces,
            codes: &ref_project.codes,
            index: &self.index,
            root: &self.root,
            imports: &self.imports,
        };
        let link_options = rule_options(
            "body.links",
            &self.config.validation,
            &collection.validation,
        );
        let ignore: Vec<Template> = ignore_globs(&link_options)
            .into_iter()
            .filter_map(|glob| Template::parse(&glob).ok())
            .collect();

        let doc = BodyDocContext {
            name,
            doc_path: path,
            doc_text: text,
            ctx: &ctx,
            ignore: &ignore,
            moved: &ref_project.moved,
            collection: &collection.validation,
            strict,
            audit,
            links_level,
            anchors_level,
        };
        let mut findings = Vec::new();
        // A reference-style use is checked once, at its definition, not at each use.
        for link in scanned.links.iter().filter(|link| !link.is_reference) {
            self.check_body_destination(
                &link.written,
                link.target.as_deref(),
                link.anchor.as_deref(),
                Position {
                    line: link.line,
                    col: link.col,
                },
                None,
                &doc,
                &mut findings,
            );
        }
        for def in &scanned.definitions {
            self.check_body_destination(
                &def.written,
                def.target.as_deref(),
                def.anchor.as_deref(),
                Position {
                    line: def.line,
                    col: def.col,
                },
                Some(def.uses),
                &doc,
                &mut findings,
            );
        }
        if let Some(level) = links_level {
            for dup in &scanned.duplicate_definitions {
                findings.push(validate::finding_at(
                    name,
                    level,
                    "body.links",
                    None,
                    Position {
                        line: dup.line,
                        col: dup.col,
                    },
                    format!(
                        "already defined at line {}; this definition is ignored",
                        dup.first_line
                    ),
                ));
            }
            for suspect in &scanned.suspects {
                if matches!(
                    refs::classify_body(&suspect.inner, &ctx),
                    refs::BodyDestination::Skip
                ) {
                    continue;
                }
                let message = if suspect.definition {
                    format!(
                        "this line looks like a reference definition but is not one (an unescaped space?): write <{}> or use %20",
                        suspect.inner
                    )
                } else {
                    format!(
                        "this looks like a link but is not one (an unescaped space?): write <{}> or use %20",
                        suspect.inner
                    )
                };
                findings.push(validate::finding_at(
                    name,
                    level,
                    "body.links",
                    None,
                    Position {
                        line: suspect.line,
                        col: suspect.col,
                    },
                    message,
                ));
            }
        }
        if let Some(level) = mentions_level {
            let options = rule_options(
                "body.mentions",
                &self.config.validation,
                &collection.validation,
            );
            let inline_code = bool_option(&options, "inlineCode", true);
            let fenced_code = bool_option(&options, "fencedCode", false);
            #[expect(
                clippy::expect_used,
                reason = "`links::mentions` returns an error only when `frontmatter::split` does on \
                          the same text; `links::scan(text)` earlier in `check_body` ran `split` on \
                          this `text` and returned `Ok`, or this line is not reached"
            )]
            let mentions = links::mentions(text, inline_code, fenced_code)
                .expect("frontmatter.parse already refused an unclosed block");
            for mention in mentions {
                if self.mention_missing(&mention.written, entry.namespace, &ref_project.codes)
                    != Some(true)
                {
                    continue;
                }
                let position = Position {
                    line: mention.line,
                    col: mention.col,
                };
                match self.moved_outcome(
                    name,
                    &mention.written,
                    &mention.written,
                    position,
                    &ref_project.moved,
                    &collection.validation,
                    strict,
                    audit,
                ) {
                    Some(Some(finding)) => findings.push(finding),
                    Some(None) => {}
                    None => findings.push(validate::finding_at(
                        name,
                        level,
                        "body.mentions",
                        None,
                        position,
                        format!("{} not found", mention.written),
                    )),
                }
            }
        }
        findings
    }

    /// `uses` is `Some` only for a reference definition, whose finding says how often it is
    /// used. A `None` target is `[t](#local)`: `body.anchors` reads this document's own headings.
    #[allow(
        clippy::too_many_arguments,
        reason = "five parts genuinely vary per destination (written, target, anchor, position,
    uses); doc and findings are the one context and the one output every call shares. Bundling
    the five would hide which one a caller is passing, the same reasoning `unresolved_ref_finding`
    already gives for a comparable list"
    )]
    fn check_body_destination(
        &self,
        written: &str,
        target: Option<&str>,
        anchor: Option<&str>,
        position: Position,
        uses: Option<usize>,
        doc: &BodyDocContext,
        findings: &mut Vec<Finding>,
    ) {
        let name = doc.name;
        // Looked up by `target`, not `written`: a recorded move never has a `#anchor`, so
        // `[t](old.md#section)` must be looked up as `old.md`.
        let missing = |findings: &mut Vec<Finding>, lookup: &str| match self.moved_outcome(
            name,
            lookup,
            written,
            position,
            doc.moved,
            doc.collection,
            doc.strict,
            doc.audit,
        ) {
            Some(Some(finding)) => findings.push(finding),
            Some(None) => {}
            None => {
                if let Some(level) = doc.links_level {
                    findings.push(validate::finding_at(
                        name,
                        level,
                        "body.links",
                        None,
                        position,
                        missing_target_message(written, uses),
                    ));
                }
            }
        };
        let resolved_path = match target {
            None => Some(doc.doc_path.to_owned()),
            Some(target) => match refs::classify_body(target, doc.ctx) {
                refs::BodyDestination::Skip => return,
                refs::BodyDestination::BadPrefix => {
                    missing(findings, target);
                    None
                }
                refs::BodyDestination::ImportAbsent(absence) => {
                    if let Some(level) =
                        self.imports_absent_level(doc.collection, doc.strict, doc.audit)
                    {
                        findings.push(validate::finding_at(
                            name,
                            level,
                            "imports.absent",
                            None,
                            position,
                            format!(
                                "the link `{target}` does not resolve: {}",
                                absence.message()
                            ),
                        ));
                    }
                    None
                }
                refs::BodyDestination::Path(joined) => {
                    if doc.ignore.iter().any(|glob| ignore_matches(glob, &joined)) {
                        return;
                    }
                    match refs::resolve_path(&joined, &self.index, &self.root) {
                        Ok(resolved) => Some(resolved.path),
                        Err(_) => {
                            missing(findings, target);
                            None
                        }
                    }
                }
                refs::BodyDestination::Import { alias, path } => {
                    if doc.ignore.iter().any(|glob| ignore_matches(glob, &path)) {
                        return;
                    }
                    match self.resolve_import_outcome(&alias, &path) {
                        // The `#anchor` is not checked there (see `resolve_import_outcome`).
                        RefOutcome::Resolved(_) => None,
                        RefOutcome::Unresolved(_) => {
                            missing(findings, target);
                            None
                        }
                    }
                }
            },
        };
        let (Some(level), Some(anchor), Some(target_path)) =
            (doc.anchors_level, anchor, &resolved_path)
        else {
            return;
        };
        let headings = self.target_headings(target_path, doc.doc_path, doc.doc_text);
        let found = headings
            .iter()
            .any(|heading| heading.slug.to_lowercase() == anchor.to_lowercase());
        if !found {
            findings.push(validate::finding_at(
                name,
                level,
                "body.anchors",
                None,
                position,
                format!("the heading `#{anchor}` does not exist in `{target_path}`"),
            ));
        }
    }

    /// A target that cannot be read or parsed has no headings, so an anchor into it is reported
    /// rather than passed.
    fn target_headings(&self, target_path: &str, doc_path: &str, doc_text: &str) -> Vec<Heading> {
        if target_path == doc_path {
            return body::headings(doc_text).unwrap_or_default();
        }
        let text = match self.index.get(target_path) {
            Some(entry) => fs::read_to_string(&entry.file).ok(),
            None => fs::read_to_string(self.root.join(target_path)).ok(),
        };
        text.and_then(|text| body::headings(&text).ok())
            .unwrap_or_default()
    }

    /// `None` when the mention's code is not a code of this project, so `UTF-8` is never
    /// checked; otherwise whether it fails to resolve. A prefix naming no sibling namespace and an
    /// import prefix both read as not found: a mention has one outcome for every failed lookup,
    /// unlike a ref's `bad-prefix`.
    fn mention_missing(
        &self,
        written: &str,
        doc_namespace: usize,
        codes: &BTreeSet<String>,
    ) -> Option<bool> {
        let key = written.rsplit(':').next().unwrap_or(written);
        if !codes.contains(refs::code_of(key)) {
            return None;
        }
        if written.contains("::") {
            return Some(true);
        }
        let namespace = match written.rsplit_once(':') {
            Some((prefix, _)) => match self.namespace_index(prefix) {
                Some(namespace) => namespace,
                None => return Some(true),
            },
            None => doc_namespace,
        };
        Some(self.index.key(namespace, key).is_none())
    }

    /// `None` when `lookup` matches no recorded move, and the caller reports its own missing
    /// finding. `Some(None)` when `refs.moved` is `off` outside `--audit`: `off` means no finding,
    /// not a fallback to another one.
    #[allow(
        clippy::too_many_arguments,
        reason = "each part is independent context the two callers (a body destination, a
    mention) already hold; bundling them would hide which one changes between the two, the same
    reasoning `unresolved_ref_finding` already gives for a comparable list"
    )]
    fn moved_outcome(
        &self,
        name: &DocName,
        lookup: &str,
        display: &str,
        position: Position,
        moved: &BTreeMap<String, String>,
        collection: &Rules,
        strict: bool,
        audit: bool,
    ) -> Option<Option<Finding>> {
        let new_id = moved.get(lookup)?;
        Some(
            validate::effective_level(
                Level::Error,
                "refs.moved",
                &self.config.validation,
                collection,
                strict,
                audit,
            )
            .map(|level| {
                validate::finding_at(
                    name,
                    level,
                    "refs.moved",
                    None,
                    position,
                    format!("the ref `{display}` no longer resolves: it was moved to `{new_id}`"),
                )
            }),
        )
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "each part is independent context `check_refs`, the one caller, already holds (the
    document's name, which field and ref, why it failed, the project's moved records, the
    collection's own rule levels, strict, audit); `body.mentions` checks a moved mention through
    `moved_outcome` instead, so a bundle would only move the same list somewhere else"
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
        audit: bool,
    ) -> Option<Finding> {
        if let Some(new_id) = moved.get(written) {
            let level = validate::effective_level(
                Level::Error,
                "refs.moved",
                &self.config.validation,
                collection,
                strict,
                audit,
            )?;
            return Some(validate::finding(
                name,
                level,
                "refs.moved",
                Some(field_name),
                format!("the ref `{written}` no longer resolves: it was moved to `{new_id}`"),
            ));
        }
        if let refs::Reason::ImportAbsent(absence) = &reason {
            return self
                .imports_absent_level(collection, strict, audit)
                .map(|level| {
                    validate::finding(
                        name,
                        level,
                        "imports.absent",
                        Some(field_name),
                        format!(
                            "the ref `{written}` does not resolve: {}",
                            absence.message()
                        ),
                    )
                });
        }
        Some(validate::finding(
            name,
            Severity::Error,
            "refs.resolve",
            Some(field_name),
            reason_message(written, &reason),
        ))
    }

    /// `None` when the rule is `off` outside `--audit`: a ref into an absent import is then
    /// reported under neither this rule nor `refs.resolve`.
    fn imports_absent_level(
        &self,
        collection: &Rules,
        strict: bool,
        audit: bool,
    ) -> Option<Severity> {
        validate::effective_level(
            Level::Warn,
            "imports.absent",
            &self.config.validation,
            collection,
            strict,
            audit,
        )
    }

    fn project_codes(&self) -> BTreeSet<String> {
        self.collections
            .iter()
            .filter_map(|collection| collection.schema.code.clone())
            .collect()
    }

    /// In collection order, so `refs::Resolved::collection` indexes it.
    pub(crate) fn schema_infos(&self) -> Vec<refs::SchemaInfo<'_>> {
        self.collections
            .iter()
            .map(|collection| refs::SchemaInfo {
                name: &collection.schema.name,
                code: collection.schema.code.as_deref(),
            })
            .collect()
    }

    /// `None` for a target in no collection (reachable through `target: "*"`). An alias that is
    /// not loaded also reads as no schema, though `refs::resolve_into_import` never produces one.
    fn schema_info_of(&self, resolved: &refs::Resolved) -> Option<refs::SchemaInfo<'_>> {
        let collection = resolved.collection?;
        match &resolved.project {
            None => self
                .collections
                .get(collection)
                .map(|found| refs::SchemaInfo {
                    name: &found.schema.name,
                    code: found.schema.code.as_deref(),
                }),
            Some(alias) => match self.imports.get(alias) {
                Some(ImportState::Loaded(imported)) => {
                    imported.schema_infos().into_iter().nth(collection)
                }
                _ => None,
            },
        }
    }

    /// `refs.acyclic`'s findings are returned apart: which of them are reported depends on the
    /// scope.
    fn ref_project(&self) -> Result<(RefProject, Vec<Finding>), Error> {
        self.ref_project_inner(None)
    }

    /// [`Project::ref_project`], with `path` read from `candidate` instead of disk, so a write can
    /// ask whether the text it is about to write closes a cycle.
    fn ref_project_for_candidate(
        &self,
        path: &str,
        entry: &Indexed,
        candidate: &str,
    ) -> Result<(RefProject, Vec<Finding>), Error> {
        self.ref_project_inner(Some((path, entry, candidate)))
    }

    fn ref_project_inner(
        &self,
        candidate: Option<(&str, &Indexed, &str)>,
    ) -> Result<(RefProject, Vec<Finding>), Error> {
        let codes = self.project_codes();
        let (moved, acyclic) = self.prescan_refs(&codes, candidate)?;
        Ok((RefProject { codes, moved }, acyclic))
    }

    /// A fact about the config, not a document, so it is reported for `All` and `Schemas`, never
    /// for `Paths`.
    fn shadowed_names_findings(&self, strict: bool, audit: bool) -> Vec<Finding> {
        let Some(level) = validate::effective_level(
            Level::Warn,
            "names.shadowed",
            &self.config.validation,
            &Rules::new(),
            strict,
            audit,
        ) else {
            return Vec::new();
        };
        self.config
            .namespaces
            .iter()
            .map(|namespace| namespace.name.as_str())
            .filter(|name| self.config.imports.contains_key(*name))
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

    fn state_missing_findings(
        &self,
        present: &BTreeSet<(usize, usize)>,
        scope: &Scope,
    ) -> Vec<Finding> {
        let mut findings = Vec::new();
        for (namespace_idx, namespace) in self.config.namespaces.iter().enumerate() {
            if !scope.contains(&namespace.name) {
                continue;
            }
            let recorded = self.state.get(&namespace.name);
            for (collection_idx, collection) in self.collections.iter().enumerate() {
                if collection.schema.code.is_none() {
                    continue;
                }
                if !present.contains(&(namespace_idx, collection_idx)) {
                    continue;
                }
                if recorded.is_some_and(|state| state.has(&collection.name)) {
                    continue;
                }
                findings.push(validate::state_missing_finding(
                    &state::file_path(&namespace.name),
                    &namespace.name,
                    &collection.name,
                    format!(
                        "the collection `{}` has documents in this namespace and no `last` recorded in its state file",
                        collection.name
                    ),
                ));
            }
        }
        findings
    }

    fn state_in_scope<'a>(
        &'a self,
        scope: &'a Scope,
    ) -> impl Iterator<Item = (usize, &'a Namespace, &'a state::StateFile)> {
        self.config
            .namespaces
            .iter()
            .enumerate()
            .filter_map(move |(namespace_idx, namespace)| {
                if !scope.contains(&namespace.name) {
                    return None;
                }
                let recorded = self.state.get(&namespace.name)?;
                Some((namespace_idx, namespace, recorded))
            })
    }

    /// Reported whatever documents the namespace holds: it is the state file's own text that is
    /// wrong.
    fn state_malformed_findings(&self, scope: &Scope) -> Vec<Finding> {
        let mut findings = Vec::new();
        for (_, namespace, recorded) in self.state_in_scope(scope) {
            for collection in &self.collections {
                if collection.schema.code.is_none() {
                    continue;
                }
                let Some(found) = recorded.malformed.get(&collection.name) else {
                    continue;
                };
                findings.push(validate::state_malformed_finding(
                    &state::file_path(&namespace.name),
                    &namespace.name,
                    &collection.name,
                    format!(
                        "the collection `{}`'s `last` in its state file is {found}, not a whole number that can be held: expected a non-negative integer",
                        collection.name
                    ),
                ));
            }
        }
        findings
    }

    /// A collection with no document left in the namespace has no `highest`, so its `last` is not
    /// reported: that gap is what keeps a retired key retired (SPC-8).
    fn state_behind_findings(
        &self,
        highest: &BTreeMap<(usize, usize), u64>,
        scope: &Scope,
    ) -> Vec<Finding> {
        let mut findings = Vec::new();
        for (namespace_idx, namespace, recorded) in self.state_in_scope(scope) {
            for (collection_idx, collection) in self.collections.iter().enumerate() {
                if collection.schema.code.is_none() {
                    continue;
                }
                let Some(&last) = recorded.last.get(&collection.name) else {
                    continue;
                };
                let Some(&top) = highest.get(&(namespace_idx, collection_idx)) else {
                    continue;
                };
                if last >= top {
                    continue;
                }
                findings.push(validate::state_behind_finding(
                    &state::file_path(&namespace.name),
                    &namespace.name,
                    &collection.name,
                    format!(
                        "the collection `{}`'s `last` in its state file is {last}, lower than the highest existing number {top}: allocation still gives the right number, but the record itself is wrong",
                        collection.name
                    ),
                ));
            }
        }
        findings
    }

    /// A `Project` that loaded holds every configured collection, so a name missing from it is a
    /// collection that is gone, not one broken elsewhere. A finding, not a config error, so it
    /// stops nothing (SPC-8).
    fn state_retired_findings(&self, scope: &Scope) -> Vec<Finding> {
        let mut findings = Vec::new();
        let known: BTreeSet<&str> = self.collections.iter().map(|c| c.name.as_str()).collect();
        for (_, namespace, recorded) in self.state_in_scope(scope) {
            let mut names: BTreeSet<&str> = recorded.last.keys().map(String::as_str).collect();
            names.extend(recorded.malformed.keys().map(String::as_str));
            for name in names {
                if known.contains(name) {
                    continue;
                }
                findings.push(validate::state_retired_finding(
                    &state::file_path(&namespace.name),
                    &namespace.name,
                    name,
                    format!(
                        "the state file records `{name}`, which is not a collection of this project any more: it is kept, since it is the only record that its numbers were issued"
                    ),
                ));
            }
        }
        findings
    }

    /// A whole-project pass: a move can be recorded, and a cycle can pass, outside the scope of
    /// the run that reads them. Returns the `auto: moves` map from a written ref to the current
    /// key or path of the document that moved away from it, and one `refs.acyclic` finding per
    /// document on a cycle, per field.
    ///
    /// `candidate` is one write in progress: its `path` is scanned from its text instead of disk,
    /// and as an extra document when it is not indexed yet.
    fn prescan_refs(
        &self,
        codes: &BTreeSet<String>,
        candidate: Option<(&str, &Indexed, &str)>,
    ) -> Result<(BTreeMap<String, String>, Vec<Finding>), Error> {
        let mut accum = PrescanAccum {
            moved: BTreeMap::new(),
            edges: BTreeMap::new(),
        };
        // Lets a ref to the candidate resolve while it is not indexed yet
        // (`refs::resolve_one_for_candidate`).
        let phantom = candidate.map(|(path, entry, _)| refs::Candidate {
            namespace: entry.namespace,
            key: entry.key.as_deref(),
            path,
        });
        for (path, entry) in self.index.iter() {
            let text = match candidate {
                Some((candidate_path, _, candidate_text)) if candidate_path == path => {
                    candidate_text.to_owned()
                }
                _ => fs::read_to_string(&entry.file).map_err(Error::io_at(&entry.file))?,
            };
            self.prescan_one(path, entry, &text, codes, phantom.as_ref(), &mut accum);
        }
        if let Some((candidate_path, candidate_entry, candidate_text)) = candidate
            && self.index.get(candidate_path).is_none()
        {
            self.prescan_one(
                candidate_path,
                candidate_entry,
                candidate_text,
                codes,
                phantom.as_ref(),
                &mut accum,
            );
        }
        let PrescanAccum { moved, edges } = accum;
        let mut findings = Vec::new();
        for (field_name, field_edges) in &edges {
            for cyclic_path in refs::cyclic_nodes(field_edges) {
                let found = match self.index.get(&cyclic_path) {
                    Some(entry) => Some((entry.collection, entry.namespace, entry.key.as_deref())),
                    None => candidate.and_then(|(candidate_path, candidate_entry, _)| {
                        (candidate_path == cyclic_path).then_some((
                            candidate_entry.collection,
                            candidate_entry.namespace,
                            candidate_entry.key.as_deref(),
                        ))
                    }),
                };
                let Some((collection_idx, namespace_idx, key)) = found else {
                    continue;
                };
                let collection = &self.collections[collection_idx];
                let namespace = &self.config.namespaces[namespace_idx].name;
                let name = DocName {
                    path: &cyclic_path,
                    namespace,
                    collection: &collection.name,
                    key,
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
        Ok((moved, findings))
    }

    fn prescan_one(
        &self,
        path: &str,
        entry: &Indexed,
        text: &str,
        codes: &BTreeSet<String>,
        phantom: Option<&refs::Candidate>,
        accum: &mut PrescanAccum,
    ) {
        let PrescanAccum { moved, edges } = accum;
        let collection = &self.collections[entry.collection];
        let Some(fields) = parsed_fields(text, &collection.schema) else {
            return;
        };
        let ctx = refs::Ctx {
            doc_namespace: entry.namespace,
            doc_path: path,
            ref_base: collection.ref_base,
            namespaces: &self.config.namespaces,
            codes,
            index: &self.index,
            root: &self.root,
            imports: &self.imports,
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
                    let resolved = match phantom {
                        Some(candidate) => {
                            refs::resolve_one_for_candidate(written, &ctx, candidate)
                        }
                        None => refs::resolve_one(written, &ctx),
                    };
                    if let Ok(resolved) = resolved {
                        edges
                            .entry(field_name.clone())
                            .or_default()
                            .push((path.to_owned(), resolved.path));
                    }
                }
            }
        }
    }

    /// A path is read as it stands, relative to the project folder: a namespace prefix only
    /// chooses scope. A key is resolved against the namespaces in `scope`, since the same key can
    /// be issued once in each.
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
            DocumentArg::Key { namespace, key, .. } => {
                self.resolve_key(namespace.as_deref(), key, scope)?
            }
        };
        #[expect(
            clippy::expect_used,
            reason = "the `Path` arm above returns `NotFound` unless `self.index.get(path)` is \
                      `Some`, and the `Key` arm's path comes from `resolve_key`, which reads it \
                      from `self.index.key(..)`; `Index::build` binds a key to a path in the same \
                      step that inserts the path's entry, and when it removes an overlapping path's \
                      entry it removes the path from its key group too; `Index` has no other \
                      mutator, so a path in a key group has an entry"
        )]
        let entry = self
            .index
            .get(&path)
            .expect("the path was just looked up above");
        let text = fs::read_to_string(&entry.file).map_err(Error::io_at(&entry.file))?;
        Ok((path, entry, text))
    }

    /// `NotFound` carries no `./name` hint: a key names no place on disk.
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
            #[expect(
                clippy::expect_used,
                reason = "this arm of `match found.len()` runs only when `found.len()` is 1, so \
                          `found.into_iter().next()` is `Some`"
            )]
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

    /// The message names `./path`, relative to the current directory, when a file is there: a
    /// suggestion, not a substitution.
    fn not_found(&self, path: &str, env: &dyn Env) -> Error {
        let hint = env.current_dir().is_ok_and(|cwd| cwd.join(path).is_file());
        Error::NotFound {
            path: path.to_owned(),
            hint,
        }
    }

    /// Moves `from` to `to` within this project, rewriting every ref this project holds to it in
    /// its written form.
    ///
    /// The document itself is renamed last, so a stop partway leaves it where it was and the
    /// same command run again finishes the work: a ref already rewritten resolves to the new path
    /// and is not found again (SPC-2).
    pub fn mv(
        &self,
        from: &DocumentArg,
        to: &DocumentArg,
        scope: &Scope,
        lock_timeout: Duration,
        deps: &Deps,
    ) -> Result<MvReport, Error> {
        if from.project_prefix().is_some() || to.project_prefix().is_some() {
            return Err(Error::BadArgument(
                "mv writes only in the project it is run in: an argument naming a document of \
                 another project is bad arguments"
                    .to_owned(),
            ));
        }

        let (from_path, from_entry, from_text) = self.resolve(from, scope, deps.env)?;
        let from_namespace = from_entry.namespace;
        let from_key = from_entry.key.clone();
        let from_file = from_entry.file.clone();
        let from_collection = from_entry.collection;

        let to_path = self.mv_destination_path(to, scope)?;
        let to_full = self.root.join(&to_path);

        // Identity first: a refusal here writes nothing and needs no lock. `same_file` is also
        // true for the same path given twice, which gets its own message (SPC-2).
        if from_path == to_path {
            return Err(Error::AlreadyExists {
                path: to_path.clone(),
                message: format!("`{to_path}` already names this document: nothing to move"),
            });
        }
        if deps
            .fs
            .same_file(&from_file, &to_full)
            .map_err(Error::io_at(&to_full))?
        {
            return Err(Error::AlreadyExists {
                path: to_path.clone(),
                message: format!(
                    "`{to_path}` is not a different file from `{from_path}`: the file system \
                     does not tell the two names apart, so no change was made"
                ),
            });
        }

        // A coded document's path is fixed by its key, and the one path its key names was
        // refused above, so only `--renumber` can move it (SPC-2).
        if let Some(key) = &from_key {
            let to_namespace = mv::namespace_of(&self.config.namespaces, &to_path);
            let message = if to_namespace != Some(from_namespace) {
                format!(
                    "`{from_path}` is a coded document: its key `{key}` belongs to the \
                     namespace that issued it, so `mv` cannot move it to another namespace; use \
                     `mv --renumber` instead"
                )
            } else {
                format!(
                    "`{from_path}` is a coded document: its path is fixed by its key `{key}` \
                     within its own namespace, so it cannot be moved to `{to_path}`"
                )
            };
            return Err(Error::BadArgument(message));
        }

        let to_namespace = mv::namespace_of(&self.config.namespaces, &to_path);
        let to_below = to_namespace
            .map(|ns| strip_namespace_folder(&to_path, &self.config.namespaces[ns].folder));
        let to_collection = to_below.as_deref().and_then(|below| {
            self.members
                .iter()
                .position(|member| member.template.matches_path(below))
        });
        // A document without a code has no key to fill a coded template with.
        if let Some(ci) = to_collection
            && self.collections[ci].schema.code.is_some()
        {
            return Err(Error::BadArgument(format!(
                "`{to_path}` is in the coded collection `{}`, and a document without a code \
                 cannot move into it; `typdoc new` allocates a key there",
                self.collections[ci].name
            )));
        }

        let (rewrite_by_holder, unrewritten) = self.mv_reverse_scan(from, scope, deps)?;
        let locks = self.mv_lock(
            from_namespace,
            to_namespace,
            &rewrite_by_holder,
            deps,
            lock_timeout,
        )?;

        // The authoritative check, under the lock; the one above rules out only the same file
        // (SPC-10).
        if deps.fs.exists(&to_full).map_err(Error::io_at(&to_full))? {
            return Err(Error::AlreadyExists {
                path: to_path.clone(),
                message: format!("`{to_path}` already exists: nothing was written"),
            });
        }

        let (mut changes, rewritten) =
            self.mv_rewrite_changes(&rewrite_by_holder, &to_path, None)?;
        if let Some(change) = self.mv_document_change(
            &from_path,
            from_collection,
            &from_text,
            &from_file,
            to_collection,
        )? {
            changes.push(change);
        }

        self.mv_finish(deps, locks, &changes, &from_file, &to_full)?;

        let (document, findings) =
            self.mv_result(&to_path, to_namespace, to_collection, &to_full)?;
        Ok(MvReport {
            document,
            rewritten,
            unrewritten,
            findings,
        })
    }

    /// `typdoc mv FROM --renumber NAMESPACE`: moves a coded document to another namespace of
    /// this project under a new key, allocated as `new` allocates one.
    ///
    /// The destination's state is written before `mv::commit` runs, so before the document
    /// appears under its new key wherever an interruption lands. The source's state is never
    /// written: moving a document out issues nothing there (SPC-2).
    pub fn mv_renumber(
        &self,
        from: &DocumentArg,
        namespace: &str,
        scope: &Scope,
        lock_timeout: Duration,
        deps: &Deps,
    ) -> Result<MvReport, Error> {
        if from.project_prefix().is_some() {
            return Err(Error::BadArgument(
                "mv writes only in the project it is run in: an argument naming a document of \
                 another project is bad arguments"
                    .to_owned(),
            ));
        }
        if namespace.contains("::") {
            return Err(Error::BadArgument(format!(
                "`{namespace}` cannot name another project: --renumber moves a document to a \
                 namespace of this project only; reading across projects is written \
                 `project::namespace:key`, and no command writes into another project"
            )));
        }

        let (from_path, from_entry, from_text) = self.resolve(from, scope, deps.env)?;
        let from_namespace = from_entry.namespace;
        let from_file = from_entry.file.clone();
        let from_collection = from_entry.collection;
        let Some(from_key) = from_entry.key.clone() else {
            return Err(Error::BadArgument(format!(
                "`{from_path}` has no code: --renumber moves a coded document to another \
                 namespace under a new key, and this document has none to renumber"
            )));
        };

        let Some(to_namespace) = self.namespace_index(namespace) else {
            return Err(Error::BadArgument(format!(
                "`{namespace}` is not a namespace of this project, which has: {}",
                self.config
                    .namespaces
                    .iter()
                    .map(|n| n.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )));
        };
        if to_namespace == from_namespace {
            return Err(Error::BadArgument(format!(
                "`{namespace}` is the namespace `{from_path}` is already in: renumbering into \
                 it would retire the key `{from_key}` for good while the document never moved, \
                 so nothing is written"
            )));
        }

        let (rewrite_by_holder, unrewritten) = self.mv_reverse_scan(from, scope, deps)?;
        let locks = self.mv_lock(
            from_namespace,
            Some(to_namespace),
            &rewrite_by_holder,
            deps,
            lock_timeout,
        )?;
        #[expect(
            clippy::expect_used,
            reason = "`mv_lock` always locks at least the source namespace, inserted \
                      unconditionally into the set it builds"
        )]
        let proof = locks
            .first()
            .expect("mv --renumber always locks at least the source namespace");

        let to_namespace_name = self.config.namespaces[to_namespace].name.clone();
        let (new_key, to_path, next) = self.allocate_key(to_namespace, from_collection)?;
        let to_full = self.root.join(&to_path);

        // Should not be reachable, but checked under the lock before anything is written
        // (SPC-10).
        if deps.fs.exists(&to_full).map_err(Error::io_at(&to_full))? {
            return Err(Error::AlreadyExists {
                path: to_path.clone(),
                message: format!(
                    "`{to_path}` already exists: the key `{new_key}` was just allocated and \
                     should not be reachable"
                ),
            });
        }

        let (mut changes, rewritten) =
            self.mv_rewrite_changes(&rewrite_by_holder, &to_path, Some((&from_key, &new_key)))?;
        // With its namespace prefix: a bare key would not say which namespace issued it.
        let previous_name = format!("{}:{from_key}", self.config.namespaces[from_namespace].name);
        if let Some(change) = self.mv_document_change(
            &previous_name,
            from_collection,
            &from_text,
            &from_file,
            Some(from_collection),
        )? {
            changes.push(change);
        }

        // Before `mv::commit`: a number recorded and then not used is an ordinary skip, while a
        // document under a number not recorded is how a number is issued twice (SPC-2).
        state::write(
            deps.fs,
            proof,
            &self.root,
            &to_namespace_name,
            &self.collections[from_collection].name,
            next,
        )?;

        self.mv_finish(deps, locks, &changes, &from_file, &to_full)?;

        let (document, findings) = self.mv_result(
            &to_path,
            Some(to_namespace),
            Some(from_collection),
            &to_full,
        )?;
        Ok(MvReport {
            document,
            rewritten,
            unrewritten,
            findings,
        })
    }

    /// Read before any lock is taken: it decides which namespaces to lock. A field named
    /// `"$body"` is always a body link, since no schema field may start with `$`.
    fn mv_reverse_scan(
        &self,
        from: &DocumentArg,
        scope: &Scope,
        deps: &Deps,
    ) -> Result<(RewriteByHolder, Vec<UnrewrittenRef>), Error> {
        let reverse = self.refs(from, scope, true, None, deps.env)?;
        let mut rewrite_by_holder: RewriteByHolder = BTreeMap::new();
        let mut unrewritten: Vec<UnrewrittenRef> = Vec::new();
        for reference in reverse.refs {
            let RefOutcome::Resolved(holder) = &reference.other else {
                continue; // a reverse scan resolves the holder itself, always
            };
            if reference.field == "$body" {
                let links_off = self.index.get(&holder.path).is_none_or(|entry| {
                    let collection = &self.collections[entry.collection];
                    validate::effective_level(
                        Level::Error,
                        "body.links",
                        &self.config.validation,
                        &collection.validation,
                        false,
                        false,
                    )
                    .is_none()
                });
                if links_off {
                    unrewritten.push(UnrewrittenRef {
                        reference,
                        reason: UnrewrittenReason::LinksRuleOff,
                    });
                    continue;
                }
            }
            rewrite_by_holder
                .entry(holder.path.clone())
                .or_default()
                .push(reference);
        }
        unrewritten.extend(self.mv_reverse_mentions(&reverse.document)?);
        Ok((rewrite_by_holder, unrewritten))
    }

    /// A mention is never rewritten, so every mention of `from`'s key is reported (SPC-2). A
    /// mention is always a key, so a plain `mv`, which cannot move a coded document, reads no
    /// file here.
    ///
    /// A second full read of the project: the reverse scan keeps no raw text, and
    /// `links::mentions` needs the whole file to compute positions.
    fn mv_reverse_mentions(&self, from: &RefName) -> Result<Vec<UnrewrittenRef>, Error> {
        let Some(from_key) = &from.key else {
            return Ok(Vec::new());
        };
        let mut holders: Vec<(&str, &Indexed)> = self.index.iter().collect();
        holders.sort_by_key(|(path, _)| *path);
        let mut found = Vec::new();
        for (path, entry) in holders {
            let collection = &self.collections[entry.collection];
            let text = fs::read_to_string(&entry.file).map_err(Error::io_at(&entry.file))?;
            let options = rule_options(
                "body.mentions",
                &self.config.validation,
                &collection.validation,
            );
            let inline_code = bool_option(&options, "inlineCode", true);
            let fenced_code = bool_option(&options, "fencedCode", false);
            // A document whose frontmatter does not parse contributes nothing, as in the reverse
            // scan.
            let Ok(mentions) = links::mentions(&text, inline_code, fenced_code) else {
                continue;
            };
            for mention in mentions {
                // Compared with `from`'s key rather than checked for resolving: before the move
                // it still resolves.
                let key = mention
                    .written
                    .rsplit(':')
                    .next()
                    .unwrap_or(&mention.written);
                if key != from_key {
                    continue;
                }
                found.push(UnrewrittenRef {
                    reference: RefsReference {
                        other: RefOutcome::Resolved(self.ref_name_of(path)),
                        field: "$body".to_owned(),
                        written: mention.written.clone(),
                        position: Some(Position {
                            line: mention.line,
                            col: mention.col,
                        }),
                    },
                    reason: UnrewrittenReason::Mention,
                });
            }
        }
        Ok(found)
    }

    /// Never a namespace touched only by an unrewritten ref: nothing is written there.
    fn mv_lock<'d>(
        &self,
        from_namespace: usize,
        to_namespace: Option<usize>,
        rewrite_by_holder: &RewriteByHolder,
        deps: &Deps<'d>,
        lock_timeout: Duration,
    ) -> Result<Vec<NamespaceLock<'d>>, Error> {
        let mut namespace_names: BTreeSet<String> = BTreeSet::new();
        namespace_names.insert(self.config.namespaces[from_namespace].name.clone());
        if let Some(ns) = to_namespace {
            namespace_names.insert(self.config.namespaces[ns].name.clone());
        }
        for holder_path in rewrite_by_holder.keys() {
            if let Some(entry) = self.index.get(holder_path) {
                namespace_names.insert(self.config.namespaces[entry.namespace].name.clone());
            }
        }
        let mut lock_paths = Vec::with_capacity(namespace_names.len());
        for name in &namespace_names {
            lock_paths.push(canonical_lock_path(
                deps.fs,
                local_namespace_lock_path(&self.root, name),
            )?);
        }
        let host = deps.env.hostname();
        let mut locks: Vec<NamespaceLock> = Vec::with_capacity(lock_paths.len());
        for path in order_locks(None, lock_paths) {
            locks.push(acquire(deps.fs, deps.clock, path, &host, lock_timeout)?);
        }
        Ok(locks)
    }

    fn mv_rewrite_changes(
        &self,
        rewrite_by_holder: &RewriteByHolder,
        to_path: &str,
        key_rewrite: Option<(&str, &str)>,
    ) -> Result<(Vec<ContentChange>, Vec<RewrittenRef>), Error> {
        let mut changes = Vec::new();
        let mut rewritten = Vec::new();
        for (holder_path, refs) in rewrite_by_holder {
            #[expect(
                clippy::expect_used,
                reason = "every path in `rewrite_by_holder` came from `self.index.get(holder_path)` \
                          succeeding while `mv_reverse_scan` built it"
            )]
            let entry = self
                .index
                .get(holder_path)
                .expect("holder paths in this map were already looked up above");
            let text = fs::read_to_string(&entry.file).map_err(Error::io_at(&entry.file))?;
            let (new_text, holder_rewritten) =
                self.rewrite_holder(holder_path, entry, &text, refs, to_path, key_rewrite)?;
            if new_text != text {
                changes.push(ContentChange {
                    path: entry.file.clone(),
                    bytes: new_text.into_bytes(),
                });
                rewritten.extend(holder_rewritten);
            }
        }
        Ok((changes, rewritten))
    }

    /// A rename cannot create the folder it lands in, so the destination's parent is created
    /// first.
    fn mv_finish(
        &self,
        deps: &Deps,
        locks: Vec<NamespaceLock>,
        changes: &[ContentChange],
        from_file: &Path,
        to_full: &Path,
    ) -> Result<(), Error> {
        #[expect(
            clippy::expect_used,
            reason = "`mv_lock` always locks at least the source namespace, inserted \
                      unconditionally into the set it builds"
        )]
        let proof = locks
            .first()
            .expect("mv/mv --renumber always locks at least the source namespace");

        if let Some(parent) = to_full.parent() {
            deps.fs
                .create_dir_all(parent)
                .map_err(Error::io_at(parent))?;
        }

        mv::commit(deps.fs, proof, changes, from_file, to_full).map_err(|source| Error::Io {
            file: to_full.to_owned(),
            source,
        })?;

        for lock in locks {
            let _ = release(lock);
        }
        Ok(())
    }

    /// A key names the path it already has, which is then refused as existing: `mv` never
    /// invents a key for its destination.
    fn mv_destination_path(&self, to: &DocumentArg, scope: &Scope) -> Result<String, Error> {
        match to {
            DocumentArg::Path { path, .. } => Ok(path.clone()),
            DocumentArg::Key { namespace, key, .. } => {
                self.resolve_key(namespace.as_deref(), key, scope)
            }
        }
    }

    /// A [`RewrittenRef`] is returned only for a ref actually rewritten, never for a body link
    /// left alone because its line no longer matches.
    fn rewrite_holder(
        &self,
        holder_path: &str,
        entry: &Indexed,
        text: &str,
        refs: &[RefsReference],
        new_target: &str,
        key_rewrite: Option<(&str, &str)>,
    ) -> Result<(String, Vec<RewrittenRef>), Error> {
        let collection = &self.collections[entry.collection];
        let bad = |message| Error::Frontmatter {
            file: entry.file.clone(),
            message,
        };
        let split = frontmatter::split(text).map_err(bad)?;
        let fields = match split.block {
            Some(block) => frontmatter::fields(block, &collection.schema).map_err(bad)?,
            None => Vec::new(),
        };
        let mut writer = frontmatter::YamlSerdeWriter::new(fields.clone());
        let mut frontmatter_touched = false;
        let mut body = text.to_owned();
        let mut body_touched = false;
        let mut rewritten = Vec::new();

        for reference in refs {
            let new_written = mv::rewritten_path_ref(
                &reference.written,
                collection.ref_base,
                entry.namespace,
                holder_path,
                &self.config.namespaces,
                new_target,
                key_rewrite,
            );
            let after = new_written.clone();
            if reference.field == "$body" {
                let Some(position) = reference.position else {
                    continue;
                };
                let Some((line_start, line_end)) = mv::line_span(&body, position.line) else {
                    continue;
                };
                let Some(new_line) = mv::splice_body_destination(
                    &body[line_start..line_end],
                    position.col,
                    &reference.written,
                    &new_written,
                ) else {
                    // Leave the line untouched rather than guess at a shape
                    // `mv::splice_body_destination` does not cover.
                    continue;
                };
                body.replace_range(line_start..line_end, &new_line);
                body_touched = true;
            } else {
                match fields.iter().find(|(name, _)| name == &reference.field) {
                    Some((_, Value::List(_))) => {
                        writer.replace_item(&reference.field, &reference.written, new_written);
                    }
                    _ => writer.set_scalar(&reference.field, new_written),
                }
                frontmatter_touched = true;
            }
            rewritten.push(RewrittenRef {
                document: holder_path.to_owned(),
                field: reference.field.clone(),
                before: reference.written.clone(),
                after,
            });
        }

        if !frontmatter_touched && !body_touched {
            return Ok((text.to_owned(), rewritten));
        }
        if !frontmatter_touched {
            return Ok((body, rewritten));
        }
        let new_block = writer.finish().map_err(bad)?;
        // Split again: the body may have been spliced above.
        let body_split = frontmatter::split(&body).map_err(bad)?;
        Ok((
            assemble_frontmatter(split.block.is_some(), &new_block, &body[body_split.body..]),
            rewritten,
        ))
    }

    /// `None` also when the `auto: moves` field already ends with `previous_name`: a re-run after
    /// a stop between this update and the document's own rename must not append it twice.
    fn mv_document_change(
        &self,
        previous_name: &str,
        from_collection: usize,
        from_text: &str,
        from_file: &Path,
        to_collection: Option<usize>,
    ) -> Result<Option<ContentChange>, Error> {
        let Some(ci) = to_collection else {
            return Ok(None);
        };
        let dest_schema = &self.collections[ci].schema;
        let Some((field_name, _)) = dest_schema
            .fields()
            .find(|(_, field)| field.auto == Some(Auto::Moves))
        else {
            return Ok(None);
        };
        let bad = |message| Error::Frontmatter {
            file: from_file.to_owned(),
            message,
        };
        // Read with the source's schema, the one `from_text` is written against, which may not
        // have the `auto: moves` field; `append_item` adds it either way.
        let block = frontmatter::block(from_text).map_err(bad)?;
        let fields = match block {
            Some(b) => {
                frontmatter::fields(b, &self.collections[from_collection].schema).map_err(bad)?
            }
            None => Vec::new(),
        };
        let already_recorded = fields
            .iter()
            .find(|(name, _)| name == field_name)
            .is_some_and(|(_, value)| {
                matches!(value, Value::List(items) if items.last().map(String::as_str) == Some(previous_name))
            });
        if already_recorded {
            return Ok(None);
        }
        let mut writer = frontmatter::YamlSerdeWriter::new(fields);
        writer.append_item(field_name, previous_name.to_owned());
        let new_block = writer.finish().map_err(bad)?;
        let split = frontmatter::split(from_text).map_err(bad)?;
        let new_text =
            assemble_frontmatter(split.block.is_some(), &new_block, &from_text[split.body..]);
        if new_text == from_text {
            return Ok(None);
        }
        Ok(Some(ContentChange {
            path: from_file.to_owned(),
            bytes: new_text.into_bytes(),
        }))
    }

    /// What the destination's schema rejects is reported, never refused (SPC-2). A document that
    /// left every collection has no schema, so its fields are read uncoerced.
    fn mv_result(
        &self,
        to_path: &str,
        to_namespace: Option<usize>,
        to_collection: Option<usize>,
        to_full: &Path,
    ) -> Result<(Document, Vec<Finding>), Error> {
        let text = fs::read_to_string(to_full).map_err(Error::io_at(to_full))?;
        let bad = |message| Error::Frontmatter {
            file: to_full.to_owned(),
            message,
        };
        match to_collection {
            Some(ci) => {
                let collection = &self.collections[ci];
                let fields = match frontmatter::block(&text).map_err(bad)? {
                    Some(block) => frontmatter::fields(block, &collection.schema).map_err(bad)?,
                    None => Vec::new(),
                };
                #[expect(
                    clippy::expect_used,
                    reason = "`to_collection` is `Some` only when `to_namespace` is too: both come \
                              from matching `to_below`, which is itself `Some` only when \
                              `to_namespace` is"
                )]
                let namespace_idx =
                    to_namespace.expect("to_collection is Some only when to_namespace is");
                let namespace_name = self.config.namespaces[namespace_idx].name.clone();
                // `Some` only for a coded destination, which only `--renumber` reaches.
                let below =
                    strip_namespace_folder(to_path, &self.config.namespaces[namespace_idx].folder);
                let key = self.members[ci].template.key(&below);
                let name = DocName {
                    path: to_path,
                    namespace: &namespace_name,
                    collection: &collection.name,
                    key: key.as_deref(),
                };
                let findings = validate::check_document(
                    &text,
                    &collection.schema,
                    &self.config.validation,
                    &collection.validation,
                    false,
                    false,
                    &name,
                );
                Ok((
                    Document {
                        path: to_path.to_owned(),
                        namespace: Some(namespace_name),
                        key,
                        code: collection.schema.code.clone(),
                        collection: collection.name.clone(),
                        schema: collection.schema.name.clone(),
                        project: None,
                        fields,
                    },
                    findings,
                ))
            }
            None => {
                let empty_schema = Resolved::new(String::new(), None, BTreeMap::new());
                let fields = match frontmatter::block(&text).map_err(bad)? {
                    Some(block) => frontmatter::fields(block, &empty_schema).map_err(bad)?,
                    None => Vec::new(),
                };
                Ok((
                    Document {
                        path: to_path.to_owned(),
                        namespace: to_namespace.map(|i| self.config.namespaces[i].name.clone()),
                        key: None,
                        code: None,
                        collection: String::new(),
                        schema: String::new(),
                        project: None,
                        fields,
                    },
                    Vec::new(),
                ))
            }
        }
    }
}

fn strip_namespace_folder(path: &str, folder: &str) -> String {
    if folder.is_empty() {
        return path.to_owned();
    }
    path.strip_prefix(folder)
        .and_then(|rest| rest.strip_prefix('/'))
        .unwrap_or(path)
        .to_owned()
}

/// A document that had a block keeps one, even an empty one, and one without is not given one.
fn assemble_frontmatter(has_block: bool, new_block: &str, body: &str) -> String {
    if !has_block {
        return body.to_owned();
    }
    format!("---\n{new_block}---\n{body}")
}

/// Two spellings of one lock file must sort as one lock. The file may not exist yet and cannot
/// be canonicalized, so its directory is created and canonicalized instead (SPC-10).
fn canonical_lock_path(fs: &dyn Fs, path: PathBuf) -> Result<PathBuf, Error> {
    let dir = path.parent().map(Path::to_owned).unwrap_or_default();
    fs.create_dir_all(&dir).map_err(Error::io_at(&dir))?;
    let canonical_dir = std::fs::canonicalize(&dir).map_err(Error::io_at(&dir))?;
    let name = path.file_name().unwrap_or_default();
    Ok(canonical_dir.join(name))
}

/// One `set` argument: `k=v` sets `field` to `raw`, split into a list once the field's type is
/// known, and `k=` removes `field`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetOp {
    Set { field: String, raw: String },
    Remove { field: String },
}

impl SetOp {
    pub fn field(&self) -> &str {
        match self {
            SetOp::Set { field, .. } | SetOp::Remove { field } => field,
        }
    }
}

/// `typdoc new`'s argument: a code with a title, or the path of an uncoded document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NewTarget {
    Coded { code: String, title: String },
    Path { path: String },
}

enum DefaultValue {
    Scalar(String),
    List(Vec<String>),
}

/// A default that does not fit the field's type is left for `frontmatter.types` to report once
/// the block is read back.
fn schema_default(field: &Field) -> Option<DefaultValue> {
    match field.default.as_ref()? {
        serde_json::Value::Array(items) => Some(DefaultValue::List(
            items.iter().map(json_scalar_text).collect(),
        )),
        other => Some(DefaultValue::Scalar(json_scalar_text(other))),
    }
}

/// An object, a nested array or `null` reads as empty, and `frontmatter.types` reports it once
/// the field is read back.
fn json_scalar_text(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(text) => text.clone(),
        serde_json::Value::Bool(_) | serde_json::Value::Number(_) => value.to_string(),
        _ => String::new(),
    }
}

/// `sets` are applied last, so an explicit `--set`, and a coded `new`'s title, win over a
/// default or an `auto` stamp.
fn new_block(schema: &Resolved, sets: &[SetOp], now: &str, file: &Path) -> Result<String, Error> {
    let mut writer = YamlSerdeWriter::new(Vec::new());
    for (field_name, field) in schema.fields() {
        match schema_default(field) {
            Some(DefaultValue::Scalar(text)) => writer.set_scalar(field_name, text),
            Some(DefaultValue::List(items)) => writer.set_list(field_name, items),
            None => {}
        }
    }
    for (field_name, field) in schema.fields() {
        if matches!(field.auto, Some(Auto::Create) | Some(Auto::Update)) {
            writer.set_scalar(field_name, now.to_owned());
        }
    }
    apply_ops(&mut writer, sets, schema)?;
    writer.finish().map_err(|message| Error::Frontmatter {
        file: file.to_owned(),
        message,
    })
}

/// Every offending field is reported, while `set_collected`'s own check stops at the first.
fn check_auto_direct(schema: &Resolved, sets: &[SetOp], name: &DocName) -> Vec<Finding> {
    sets.iter()
        .filter_map(|op| {
            let field = schema.field(op.field())?;
            field.auto.as_ref()?;
            Some(validate::finding(
                name,
                Severity::Error,
                "frontmatter.types",
                Some(op.field()),
                format!(
                    "the field `{}` is set automatically (`auto`) and cannot be written directly",
                    op.field()
                ),
            ))
        })
        .collect()
}

fn create_document_or_exists(
    deps: &Deps,
    lock: &NamespaceLock,
    file: &Path,
    bytes: &[u8],
    message: String,
) -> Result<(), Error> {
    create_exclusively(deps.fs, lock, file, bytes).map_err(|source| {
        if source.kind() == io::ErrorKind::AlreadyExists {
            Error::AlreadyExists {
                path: file.display().to_string(),
                message,
            }
        } else {
            Error::Io {
                file: file.to_owned(),
                source,
            }
        }
    })
}

/// `Loose` is a file matched by no collection, with no schema to validate against.
enum WriteTarget {
    Collected { path: String },
    Loose { path: String },
}

fn read_fields(
    block: Option<&str>,
    schema: &Resolved,
    file: &Path,
) -> Result<Vec<(String, Value)>, Error> {
    match block {
        Some(block) => frontmatter::fields(block, schema).map_err(|message| Error::Frontmatter {
            file: file.to_owned(),
            message,
        }),
        None => Ok(Vec::new()),
    }
}

/// List or scalar is decided here, from `schema`: neither `SetOp` nor the CLI knows a field's
/// type. A field the schema does not name is a scalar. Escapes are read here too, since how a
/// value is split depends on that decision.
fn apply_ops(writer: &mut YamlSerdeWriter, sets: &[SetOp], schema: &Resolved) -> Result<(), Error> {
    for op in sets {
        match op {
            SetOp::Set { field, raw } => {
                let is_list = schema.field(field).is_some_and(|found| {
                    matches!(found.kind, FieldType::List | FieldType::RefList)
                });
                let items = unescape_set_value(raw, is_list)?;
                if is_list {
                    writer.set_list(field, items);
                } else {
                    #[expect(
                        clippy::expect_used,
                        reason = "`unescape_set_value` with `is_list: false` never splits on `,` \
                                  (the only thing that pushes more than one item), so it always \
                                  returns exactly one item for a scalar field"
                    )]
                    let value = items.into_iter().next().expect("one item for a scalar");
                    writer.set_scalar(field, value);
                }
            }
            SetOp::Remove { field } => writer.remove_field(field),
        }
    }
    Ok(())
}

/// Unlike a `--where` value, a bare `*` is always an error: `set` has no glob.
fn unescape_set_value(raw: &str, split_on_comma: bool) -> Result<Vec<String>, Error> {
    if raw.is_empty() {
        return Ok(if split_on_comma {
            Vec::new()
        } else {
            vec![String::new()]
        });
    }
    let mut items = Vec::new();
    let mut current = String::new();
    let mut chars = raw.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => match chars.next() {
                Some(escaped @ (',' | '*' | '\\')) => current.push(escaped),
                Some(other) => {
                    return Err(Error::BadArgument(format!(
                        "`\\{other}` is not a recognized escape in a `set` value: only `\\,`, \
                         `\\*` and `\\\\` are"
                    )));
                }
                None => {
                    return Err(Error::BadArgument(
                        "a `set` value cannot end with `\\`".to_owned(),
                    ));
                }
            },
            '*' => {
                return Err(Error::BadArgument(
                    "a bare `*` is not allowed in a `set` value: `set` has no wildcard \
                     matching, unlike `--where`/`--if` (escape it as `\\*` for a literal `*`)"
                        .to_owned(),
                ));
            }
            ',' if split_on_comma => items.push(std::mem::take(&mut current)),
            other => current.push(other),
        }
    }
    items.push(current);
    Ok(items)
}

/// Compared by typed value, not by text. A field only one side has counts as changed.
fn fields_changed(before: &[(String, Value)], after: &[(String, Value)]) -> bool {
    fields_map(before) != fields_map(after)
}

fn fields_map(fields: &[(String, Value)]) -> BTreeMap<&str, &Value> {
    fields
        .iter()
        .map(|(name, value)| (name.as_str(), value))
        .collect()
}

/// `body` is kept byte for byte: a write touches only the frontmatter block.
fn splice(block_text: &str, body: &str) -> String {
    format!("---\n{block_text}---\n{body}")
}

/// Every false condition is reported, not only the first.
fn evaluate_plain_ifs(
    ifs: &[(String, Condition)],
    schema: &Resolved,
    doc: &Document,
    path: &str,
) -> Result<Vec<Finding>, Error> {
    let mut false_conditions = Vec::new();
    for (raw, condition) in ifs {
        let Condition::Plain(plain) = condition else {
            continue; // refused by the caller before this function is ever reached
        };
        let ok =
            query::evaluate(plain, schema, doc).map_err(|e| Error::BadArgument(e.to_string()))?;
        if !ok {
            false_conditions.push(Finding {
                level: Severity::Error,
                rule: "set.if",
                message: format!("`{raw}` is false"),
                path: path.to_owned(),
                namespace: doc.namespace.clone(),
                collection: (!doc.collection.is_empty()).then(|| doc.collection.clone()),
                key: doc.key.clone(),
                field: None,
                position: None,
            });
        }
    }
    Ok(false_conditions)
}

/// A `ref.*`/`refby.*` `--if` would need the whole-project reverse scan for one compare-and-set,
/// so it is refused rather than evaluated wrongly.
fn refby_if_unsupported(raw: &str) -> Error {
    Error::BadArgument(format!(
        "`--if {raw}` is a ref.*/refby.* condition, which `--if` does not support yet: only a \
         plain condition (`field op value`) can be given to `--if`"
    ))
}

/// `None` when the block cannot be parsed. Parses a second time, since
/// `validate::check_document` does not return its own parse.
fn parsed_fields(text: &str, schema: &Resolved) -> Option<Vec<(String, Value)>> {
    let block = frontmatter::block(text).ok()??;
    frontmatter::fields(block, schema).ok()
}

/// A field `schema` lacks is known to another schema in scope, which the caller has checked, so
/// here it is absent, never `query::evaluate`'s `UnknownField`.
fn condition_matches(
    condition: &PlainCondition,
    schema: &Resolved,
    doc: &Document,
) -> Result<bool, Error> {
    if let FieldRef::Named(name) = &condition.field
        && schema.field(name).is_none()
    {
        return Ok(condition.op.absent_result());
    }
    query::evaluate(condition, schema, doc).map_err(|e| Error::BadArgument(e.to_string()))
}

fn combine_quant(quant: Quant, items: impl Iterator<Item = bool>) -> bool {
    let mut items = items;
    match quant {
        Quant::All => items.all(|matched| matched),
        Quant::Any => items.any(|matched| matched),
        Quant::None => !items.any(|matched| matched),
    }
}

/// Two fields of one name whose schemas give them different types compare as equal: no rule
/// orders one type against another.
enum SortValue {
    Missing,
    Number(f64),
    Bool(bool),
    Date(NaiveDate),
    Datetime(DateTime<FixedOffset>),
    EnumPos(usize),
    /// The part before the trailing digits, and the digits as a number, so `WF-2` sorts before
    /// `WF-10`.
    Key(String, Option<u64>),
    Text(String),
}

impl SortValue {
    fn cmp_same_type(&self, other: &SortValue) -> Ordering {
        match (self, other) {
            (SortValue::Missing, SortValue::Missing) => Ordering::Equal,
            (SortValue::Number(a), SortValue::Number(b)) => {
                a.partial_cmp(b).unwrap_or(Ordering::Equal)
            }
            (SortValue::Bool(a), SortValue::Bool(b)) => a.cmp(b),
            (SortValue::Date(a), SortValue::Date(b)) => a.cmp(b),
            (SortValue::Datetime(a), SortValue::Datetime(b)) => a.cmp(b),
            (SortValue::EnumPos(a), SortValue::EnumPos(b)) => a.cmp(b),
            (SortValue::Key(pa, na), SortValue::Key(pb, nb)) => {
                pa.cmp(pb).then_with(|| cmp_key_number(*na, *nb))
            }
            (SortValue::Text(a), SortValue::Text(b)) => a.cmp(b),
            _ => Ordering::Equal,
        }
    }
}

fn cmp_key_number(a: Option<u64>, b: Option<u64>) -> Ordering {
    match (a, b) {
        (Some(a), Some(b)) => a.cmp(&b),
        (None, None) => Ordering::Equal,
        (Some(_), None) => Ordering::Greater,
        (None, Some(_)) => Ordering::Less,
    }
}

/// A value that does not fit its declared type sorts as `Missing`.
///
/// A field no schema in scope declares is not an error here, unlike in `--where`: an unknown
/// `--sort` field changes only the order, never which documents are returned.
fn sort_value(field: &FieldRef, schema: &Resolved, doc: &Document) -> SortValue {
    match field {
        FieldRef::Path => SortValue::Text(doc.path.clone()),
        FieldRef::Namespace => doc
            .namespace
            .clone()
            .map_or(SortValue::Missing, SortValue::Text),
        FieldRef::Collection => SortValue::Text(doc.collection.clone()),
        FieldRef::Schema => SortValue::Text(doc.schema.clone()),
        FieldRef::Code => doc.code.clone().map_or(SortValue::Missing, SortValue::Text),
        FieldRef::Key => match &doc.key {
            Some(key) => key_sort_value(key),
            None => SortValue::Missing,
        },
        FieldRef::Named(name) => {
            let Some(schema_field) = schema.field(name) else {
                return SortValue::Missing;
            };
            let Some(value) = query::field_value(field, doc) else {
                return SortValue::Missing;
            };
            named_sort_value(schema_field, &value)
        }
    }
}

fn key_sort_value(key: &str) -> SortValue {
    let split = key
        .rfind(|c: char| !c.is_ascii_digit())
        .map_or(0, |i| i + 1);
    if split == key.len() {
        return SortValue::Key(key.to_owned(), None);
    }
    let (prefix, digits) = key.split_at(split);
    SortValue::Key(prefix.to_owned(), digits.parse::<u64>().ok())
}

fn named_sort_value(field: &Field, value: &Value) -> SortValue {
    match &field.kind {
        FieldType::Number => match value {
            Value::Number(n) => n.as_f64().map_or(SortValue::Missing, SortValue::Number),
            _ => SortValue::Missing,
        },
        FieldType::Bool => match value {
            Value::Bool(b) => SortValue::Bool(*b),
            _ => SortValue::Missing,
        },
        FieldType::Date => match value {
            Value::Date(text) => {
                query::parse_date(text).map_or(SortValue::Missing, SortValue::Date)
            }
            _ => SortValue::Missing,
        },
        FieldType::Datetime => match value {
            Value::Datetime(text) => {
                query::parse_datetime(text).map_or(SortValue::Missing, SortValue::Datetime)
            }
            _ => SortValue::Missing,
        },
        // A bare enum field sorts as the empty text it reads as everywhere else: at whatever
        // position `""` holds in `values`, `Missing` when it holds none.
        FieldType::Enum => match value {
            Value::Text(text) => field
                .values
                .as_deref()
                .and_then(|values| values.iter().position(|v| v == text))
                .map_or(SortValue::Missing, SortValue::EnumPos),
            Value::Empty => field
                .values
                .as_deref()
                .and_then(|values| values.iter().position(String::is_empty))
                .map_or(SortValue::Missing, SortValue::EnumPos),
            _ => SortValue::Missing,
        },
        // `list`, `ref` and `ref[]` have no sort rule of their own and sort as the text written:
        // a fallback that always ties would make `--sort` on them do nothing.
        _ => match value {
            Value::Text(text) => SortValue::Text(text.clone()),
            Value::Empty => SortValue::Text(String::new()),
            Value::List(items) => SortValue::Text(items.join(",")),
            _ => SortValue::Missing,
        },
    }
}

/// `Missing` sorts last whether or not `:desc` reverses the rest.
fn sort_compare(
    key: &SortKey,
    schema_a: &Resolved,
    a: &Document,
    schema_b: &Resolved,
    b: &Document,
) -> Ordering {
    let va = sort_value(&key.field, schema_a, a);
    let vb = sort_value(&key.field, schema_b, b);
    match (
        matches!(va, SortValue::Missing),
        matches!(vb, SortValue::Missing),
    ) {
        (true, true) => Ordering::Equal,
        (true, false) => Ordering::Greater,
        (false, true) => Ordering::Less,
        (false, false) => {
            let ord = va.cmp_same_type(&vb);
            if key.desc { ord.reverse() } else { ord }
        }
    }
}

/// A key against a path compares as text: no rule orders that mix, and any total order will
/// do.
fn compare_identity(a: &Document, b: &Document) -> Ordering {
    match (&a.key, &b.key) {
        (Some(ka), Some(kb)) => key_sort_value(ka).cmp_same_type(&key_sort_value(kb)),
        (None, None) => a.path.cmp(&b.path),
        (Some(ka), None) => ka.as_str().cmp(b.path.as_str()),
        (None, Some(kb)) => a.path.as_str().cmp(kb.as_str()),
    }
}

/// `None` only for a block that cannot be parsed. Unlike `parsed_fields`, a document with no
/// block still has its body links read.
fn parsed_fields_and_body(
    text: &str,
    schema: &Resolved,
) -> Option<(Vec<(String, Value)>, BodyLinks)> {
    let block = frontmatter::block(text).ok()?;
    let fields = match block {
        None => Vec::new(),
        Some(block) => frontmatter::fields(block, schema).ok()?,
    };
    #[expect(
        clippy::expect_used,
        reason = "`links::scan` returns an error only when `frontmatter::split` does on the same \
                  text, and `frontmatter::block` is `split` with the body offset dropped; \
                  `frontmatter::block(text).ok()?` above returns before this line when it is an `Err`"
    )]
    let body = links::scan(text).expect("frontmatter.parse already refused an unclosed block");
    Some((fields, body))
}

/// Called only once `coerce::fits` has shown the value fits a `ref` or `ref[]` field.
fn ref_values(value: &Value) -> Vec<&str> {
    match value {
        Value::Text(text) => vec![text.as_str()],
        // A `ref` written with no value is one ref of no text, as `field: ''` is.
        Value::Empty => vec![""],
        Value::List(items) => items.iter().map(String::as_str).collect(),
        Value::Number(_) | Value::Bool(_) | Value::Date(_) | Value::Datetime(_) => Vec::new(),
    }
}

/// `ImportAbsent` never reaches this: it is reported under `imports.absent` instead.
fn reason_message(written: &str, reason: &refs::Reason) -> String {
    match reason {
        refs::Reason::NotFound => format!("the ref `{written}` does not resolve: not found"),
        refs::Reason::BadPrefix => {
            format!("the ref `{written}` does not resolve: its prefix names no namespace")
        }
        refs::Reason::ImportAbsent(absence) => {
            format!(
                "the ref `{written}` does not resolve: {}",
                absence.message()
            )
        }
    }
}

fn reason_id(reason: &refs::Reason) -> &'static str {
    match reason {
        refs::Reason::NotFound => "not-found",
        refs::Reason::BadPrefix => "bad-prefix",
        refs::Reason::ImportAbsent(_) => "import-absent",
    }
}

fn filtered(refs: Vec<RefsReference>, field: Option<&str>) -> Vec<RefsReference> {
    match field {
        Some(field) => refs.into_iter().filter(|r| r.field == field).collect(),
        None => refs,
    }
}

fn missing_target_message(written: &str, uses: Option<usize>) -> String {
    match uses {
        None => format!("link target missing: {written}"),
        Some(1) => format!("link target missing: {written} (used 1 time)"),
        Some(n) => format!("link target missing: {written} (used {n} times)"),
    }
}

/// Merged option by option, as `level` is: a collection that overrides only `level` keeps every
/// option the global setting gave the rule.
fn rule_options(
    rule: &str,
    global: &Rules,
    collection: &Rules,
) -> serde_json::Map<String, serde_json::Value> {
    let mut merged = global
        .get(rule)
        .map(|setting| setting.options.clone())
        .unwrap_or_default();
    if let Some(setting) = collection.get(rule) {
        merged.extend(setting.options.clone());
    }
    merged
}

fn bool_option(
    options: &serde_json::Map<String, serde_json::Value>,
    key: &str,
    default: bool,
) -> bool {
    options
        .get(key)
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(default)
}

/// Any other shape of the option is refused as `config.rule-unknown` when the config loads.
fn ignore_globs(options: &serde_json::Map<String, serde_json::Value>) -> Vec<String> {
    options
        .get("ignore")
        .and_then(serde_json::Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

/// An `ignore` glob uses the `match` template syntax, not a second glob syntax of its own.
fn ignore_matches(pattern: &Template, path: &str) -> bool {
    fn go(steps: &[Step], parts: &[&str]) -> bool {
        match steps.split_first() {
            None => parts.is_empty(),
            Some((Step::Name(segment), rest)) => matches!(
                parts.split_first(),
                Some((first, more)) if segment.matches(first) && go(rest, more)
            ),
            Some((Step::Folders, rest)) => (0..=parts.len()).any(|skip| go(rest, &parts[skip..])),
        }
    }
    let parts: Vec<&str> = path.split('/').collect();
    go(pattern.steps(), &parts)
}

fn overlap_message(collections: &[String]) -> String {
    format!(
        "this file is matched by more than one collection: `{}`",
        collections.join("`, `")
    )
}

fn printed_key(prefix: Option<&str>, key: &str) -> String {
    match prefix {
        Some(namespace) => format!("{namespace}:{key}"),
        None => key.to_owned(),
    }
}

/// No `validate` run reads an import, so accepting one in scope would give a report that reads
/// as checked and clean when nothing it named was checked.
fn reject_import_scope(scope: &Scope) -> Result<(), Error> {
    if scope.imports.is_empty() {
        return Ok(());
    }
    let named: Vec<&str> = scope
        .imports
        .iter()
        .map(|(alias, _)| alias.as_str())
        .collect();
    Err(Error::BadArgument(format!(
        "--namespace named an import ({}), and validate never reads one: a file is validated only by the project that owns it",
        named.join(", ")
    )))
}

/// A path is not narrowed by scope, so any namespace of `imported` will do. A key into a project
/// with several namespaces must name one even when it is not ambiguous, as a ref must.
fn imported_scope(imported: &Project, arg: &DocumentArg) -> Result<Scope, Error> {
    if let Some(name) = arg.namespace_prefix() {
        let index = imported.namespace_index(name).ok_or_else(|| {
            let known: Vec<&str> = imported
                .config
                .namespaces
                .iter()
                .map(|n| n.name.as_str())
                .collect();
            Error::BadArgument(format!(
                "`{name}` is not a namespace of the imported project, which has: {}",
                known.join(", ")
            ))
        })?;
        return Ok(Scope {
            source: Source::Prefix,
            namespaces: vec![imported.config.namespaces[index].name.clone()],
            imports: Vec::new(),
        });
    }
    if matches!(arg, DocumentArg::Path { .. }) || imported.config.namespaces.len() == 1 {
        return Ok(Scope {
            source: Source::Everything,
            namespaces: imported
                .config
                .namespaces
                .iter()
                .map(|n| n.name.clone())
                .collect(),
            imports: Vec::new(),
        });
    }
    Err(Error::BadArgument(
        "the imported project has more than one namespace: name one, e.g. `alias::namespace:key`"
            .to_owned(),
    ))
}

/// A path outside every collection (which `target: "*"` allows) has no index entry and takes its
/// namespace from its folder, or has none outside every namespace folder.
fn ref_name_in(
    namespaces: &[crate::config::Namespace],
    index: &Index,
    project: Option<&str>,
    path: &str,
) -> RefName {
    let project = project.map(str::to_owned);
    if let Some(entry) = index.get(path) {
        return RefName {
            path: path.to_owned(),
            namespace: Some(namespaces[entry.namespace].name.clone()),
            key: entry.key.clone(),
            project,
        };
    }
    let namespace = namespaces
        .iter()
        .filter(|space| {
            !space.folder.is_empty()
                && (path == space.folder || path.starts_with(&format!("{}/", space.folder)))
        })
        .max_by_key(|space| space.folder.len())
        .or_else(|| namespaces.iter().find(|space| space.folder.is_empty()));
    RefName {
        path: path.to_owned(),
        namespace: namespace.map(|space| space.name.clone()),
        key: None,
        project,
    }
}

/// `config.state-uncoded` is only for a collection this project has whose schema has no code. An
/// entry naming a collection the project does not have is `state.retired`, a finding `validate`
/// reports, not a config error that would stop the load (SPC-8).
fn read_state(
    root: &Path,
    config: &Config,
    loaded: &[Loaded],
    report: &mut Report,
) -> Result<BTreeMap<String, state::StateFile>, Error> {
    let coded_collections: BTreeSet<&str> = loaded
        .iter()
        .filter(|found| found.schema.code.is_some())
        .map(|found| found.name.as_str())
        .collect();
    // Complete: a collection with a config error of its own fails the load before this.
    let known_collections: BTreeSet<&str> =
        loaded.iter().map(|found| found.name.as_str()).collect();
    for orphan in state::orphans(root, &config.namespaces, &config.excluded)? {
        report.add(
            "config.state-orphan",
            &orphan,
            format!(
                "{orphan} matches no current namespace: delete it after removing a namespace, or rename it after renaming a folder"
            ),
        );
    }
    let mut by_namespace: BTreeMap<String, state::StateFile> = BTreeMap::new();
    for namespace in &config.namespaces {
        let read = state::read(root, &namespace.name)?;
        let state_path = state::file_path(&namespace.name);
        for name in read.last.keys().chain(read.malformed.keys()) {
            if !known_collections.contains(name.as_str()) {
                // `state.retired`: not reported here (see above).
                continue;
            }
            if !coded_collections.contains(name.as_str()) {
                report.add(
                    "config.state-uncoded",
                    &state_path,
                    format!(
                        "the state file records `{name}`, which is a collection of this project whose schema has no code: state applies only to a coded collection"
                    ),
                );
            }
        }
        by_namespace.insert(namespace.name.clone(), read);
    }
    Ok(by_namespace)
}

/// An alias that is not configured is reported as a schema renamed away is: either way the
/// target names nothing. An import absent on this machine is left to `imports.absent`.
fn schema_drift_findings(
    targets: &[schema::QualifiedTarget],
    imports: &BTreeMap<String, ImportState>,
) -> Vec<Finding> {
    let mut findings = Vec::new();
    for target in targets {
        for name in &target.names {
            let Some((alias, schema_name)) = name.split_once("::") else {
                continue;
            };
            match imports.get(alias) {
                None => findings.push(validate::schema_finding(
                    &target.path,
                    Some(&target.field),
                    format!(
                        "the target `{name}` names the import `{alias}`, which is not configured"
                    ),
                )),
                Some(ImportState::Absent(_)) => {}
                Some(ImportState::Loaded(imported)) => {
                    let exists = imported
                        .schema_infos()
                        .iter()
                        .any(|info| info.name == schema_name);
                    if !exists {
                        findings.push(validate::schema_finding(
                            &target.path,
                            Some(&target.field),
                            format!(
                                "the target `{name}` names the schema `{schema_name}`, which does not exist in the imported project `{alias}`"
                            ),
                        ));
                    }
                }
            }
        }
    }
    findings
}

/// An unset variable or a location with no project is absent on this machine, never an error:
/// a machine-specific import may not be set up yet. A project there whose config cannot be
/// loaded is an error, since `imports.absent` would hide a mistake that will not fix itself.
fn resolve_import(root: &Path, raw: &str, env: &dyn Env) -> Result<ImportState, Error> {
    let substituted = match crate::imports::substitute(raw, env) {
        Ok(text) => text,
        Err(variable) => {
            return Ok(ImportState::Absent(crate::imports::Absence::Variable(
                variable,
            )));
        }
    };
    let path = {
        let candidate = Path::new(&substituted);
        if candidate.is_absolute() {
            candidate.to_owned()
        } else {
            root.join(candidate)
        }
    };
    if !config_file(&path).is_file() {
        return Ok(ImportState::Absent(crate::imports::Absence::NoProject(
            path,
        )));
    }
    let imported = Project::load_inner(&path, env, false)?;
    Ok(ImportState::Loaded(Box::new(imported)))
}

fn reserved_alias_finding(file: &str, alias: &str) -> Finding {
    validate::schema_finding(
        file,
        None,
        format!(
            "the import name `{alias}` is a URL scheme (`http`, `https`, `mailto` and `file` are reserved), and the two would be told apart wrongly"
        ),
    )
}

fn read_template(collection: &Collection, report: &mut Report) -> Option<Template> {
    match Template::parse(&collection.pattern) {
        Ok(template) => Some(template),
        Err(message) => {
            report.add("config.match-template", &collection.path, message);
            None
        }
    }
}

/// Only across two different files: `config.coded-schema-shared` covers two collections naming
/// the same coded schema file.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Namespace;

    fn space(name: &str, folder: &str) -> Namespace {
        Namespace {
            name: name.to_owned(),
            folder: folder.to_owned(),
        }
    }

    fn name_of(namespaces: &[Namespace], path: &str) -> RefName {
        ref_name_in(namespaces, &Index::default(), None, path)
    }

    #[test]
    fn a_file_under_a_namespace_folder_is_named_in_that_namespace() {
        let spaces = [space("one", "story-1"), space("two", "story-2")];

        assert_eq!(
            name_of(&spaces, "story-2/deep/x.md").namespace.as_deref(),
            Some("two")
        );
        assert_eq!(
            name_of(&spaces, "story-1").namespace.as_deref(),
            Some("one")
        );
    }

    #[test]
    fn a_file_outside_every_namespace_folder_has_no_namespace() {
        let spaces = [space("one", "story-1"), space("two", "story-2")];

        assert_eq!(name_of(&spaces, "README.md").namespace, None);
        // A folder whose name only starts like a namespace folder is not inside it.
        assert_eq!(name_of(&spaces, "story-10/x.md").namespace, None);
    }

    #[test]
    fn with_a_namespace_of_no_folder_every_file_outside_a_collection_is_in_it() {
        let spaces = [space("default", "")];

        assert_eq!(
            name_of(&spaces, "docs/README.md").namespace.as_deref(),
            Some("default")
        );
    }

    #[test]
    fn a_document_with_no_namespace_holds_no_value_for_the_namespace_pseudo_field() {
        let doc = |namespace: Option<&str>| Document {
            path: "README.md".to_owned(),
            namespace: namespace.map(str::to_owned),
            key: None,
            code: None,
            collection: String::new(),
            schema: String::new(),
            project: None,
            fields: Vec::new(),
        };

        assert_eq!(query::field_value(&FieldRef::Namespace, &doc(None)), None);
        assert_eq!(
            query::field_value(&FieldRef::Namespace, &doc(Some("one"))),
            Some(Value::Text("one".to_owned()))
        );
    }

    #[test]
    fn a_name_with_no_namespace_is_the_same_document_only_as_the_same_path_with_none() {
        let name = |namespace: Option<&str>, path: &str| RefName {
            path: path.to_owned(),
            namespace: namespace.map(str::to_owned),
            key: None,
            project: None,
        };

        assert!(name(None, "README.md").is_same_document(&name(None, "README.md")));
        assert!(!name(None, "README.md").is_same_document(&name(None, "OTHER.md")));
        assert!(!name(None, "README.md").is_same_document(&name(Some("one"), "README.md")));
        assert!(name(Some("one"), "a.md").is_same_document(&name(Some("one"), "a.md")));
    }
}
